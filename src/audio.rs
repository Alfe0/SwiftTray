//! Audio operations remain on the STA UI thread (COM interfaces are !Send).
//! The endpoint switch below is isolated because Windows has not published
//! a supported desktop API for setting the system-wide default output.
use crate::config::Config;
use std::{collections::{HashMap, HashSet}, ffi::c_void, ptr};
use crate::icons::IdentityResolver;
use windows::{
    core::{GUID, Interface},
    Win32::{
        Foundation::PROPERTYKEY,
        Media::Audio::{
            eMultimedia, eRender, AudioSessionStateActive, AudioSessionStateExpired, DEVICE_STATE_ACTIVE,
            IAudioSessionControl2, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator,
            ISimpleAudioVolume, MMDeviceEnumerator,
        },
        Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioMeterInformation},
        System::Com::{CoCreateInstance as CoCreateInstanceTyped, CLSCTX_ALL, STGM_READ},
    },
};

#[link(name = "ole32")]
extern "system" {
    fn CoTaskMemFree(ptr: *mut c_void);
    fn CoCreateInstance(
        clsid: *const GUID, outer: *mut c_void, ctx: u32,
        iid: *const GUID, object: *mut *mut c_void,
    ) -> i32;
}
#[link(name = "propsys")]
extern "system" {
    fn PropVariantToString(pv: *const c_void, buffer: *mut u16, length: u32) -> i32;
}

const PKEY_FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0), pid: 14,
};
const POLICY_CLSID: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
const POLICY_IID: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);

#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub current: bool,
}

#[derive(Clone)]
pub struct Session {
    pub key: String,
    pub name: String,
    pub executable: Option<String>,
    pub app_user_model_id: Option<String>,
    pub session_icon: Option<String>,
    pub is_system_sounds: bool,
    pub volumes: Vec<ISimpleAudioVolume>,
    // Actual per-session meters. Multiple streams can belong to one app.
    pub meters: Vec<IAudioMeterInformation>,
    pub peak: f32,
    pub level: f32,
    pub muted: bool,
}

pub struct Audio {
    enumerator: IMMDeviceEnumerator,
    identities: IdentityResolver,
    master_endpoint: Option<IAudioEndpointVolume>,
    master_meter: Option<IAudioMeterInformation>,
    pub master_peak: f32,
    pub devices: Vec<Device>,
    pub sessions: Vec<Session>,
    pub master_level: f32,
    pub master_muted: bool,
    current_id: String,
}
impl Audio {
    pub fn new() -> windows::core::Result<Self> {
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstanceTyped(&MMDeviceEnumerator, None, CLSCTX_ALL)?
        };
        let mut out = Self {
            enumerator, identities: IdentityResolver::default(),
            master_endpoint: None, master_meter: None, master_peak: 0.0,
            devices: Vec::new(), sessions: Vec::new(),
            master_level: 0.0, master_muted: false, current_id: String::new(),
        };
        // The audio service may be running without an active output (for
        // example, while a USB headset is being unplugged). Keep the endpoint
        // enumerator alive and let the next device event or flyout open retry.
        let _ = out.refresh();
        Ok(out)
    }
    /// Refresh from the system, including devices that may have arrived or left.
    /// A missing default output is a valid transient state, not a reason for
    /// the tray process to retain stale COM session controls.
    pub fn refresh(&mut self) -> windows::core::Result<()> {
        let current = match unsafe { self.enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) } {
            Ok(device) => device,
            Err(error) => {
                self.clear_output_state();
                return Err(error);
            }
        };
        self.current_id = device_id(&current)?;
        let all = unsafe { self.enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)? };
        let count = unsafe { all.GetCount()? };
        self.devices.clear();
        for n in 0..count {
            if let Ok(device) = unsafe { all.Item(n) } {
                if let Ok(id) = device_id(&device) {
                    let name = friendly_name(&device).unwrap_or_else(|| format!("Audio device {}", n+1));
                    self.devices.push(Device { current: id == self.current_id, id, name });
                }
            }
        }
        // Preserve peak decay history across the periodic session refresh so
        // meter bars never blink off every time the list is rebuilt.
        let previous_peaks: HashMap<String, f32> = self.sessions.iter()
            .map(|session| (session.key.clone(), session.peak)).collect();
        self.sessions.clear();
        let endpoint: IAudioEndpointVolume = unsafe { current.Activate(CLSCTX_ALL, None)? };
        self.master_endpoint = Some(endpoint.clone());
        self.master_meter = unsafe { current.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None).ok() };
        self.master_level = unsafe { endpoint.GetMasterVolumeLevelScalar().unwrap_or(0.5) };
        self.master_muted = unsafe { endpoint.GetMute().map(|v| v.as_bool()).unwrap_or(false) }
            || self.master_level <= f32::EPSILON;

        // A session manager only enumerates sessions on its own endpoint. A game
        // may keep rendering to the *previous* endpoint after the default output
        // changes. Enumerate every active endpoint so its mixer row does not vanish.
        // Process/executable grouping also collapses duplicate application rows.
        let mut groups: HashMap<String, usize> = HashMap::new();
        let mut observed_pids: HashSet<u32> = HashSet::new();
        let mut priorities: Vec<u8> = Vec::new();
        // Prefer the default endpoint when deriving a representative volume.
        let mut endpoints: Vec<IMMDevice> = vec![current.clone()];
        for n in 0..count {
            if let Ok(device) = unsafe { all.Item(n) } {
                if device_id(&device).ok().as_deref() != Some(self.current_id.as_str()) {
                    endpoints.push(device);
                }
            }
        }
        for (endpoint_index, device) in endpoints.iter().enumerate() {
            let Ok(manager) = (unsafe { device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) }) else { continue; };
            let Ok(sessions) = (unsafe { manager.GetSessionEnumerator() }) else { continue; };
            let Ok(session_count) = (unsafe { sessions.GetCount() }) else { continue; };
            for i in 0..session_count {
                let Ok(control) = (unsafe { sessions.GetSession(i) }) else { continue; };
                let state = unsafe { control.GetState().ok() };
                if state == Some(AudioSessionStateExpired) { continue; }
                let priority: u8 = (if state == Some(AudioSessionStateActive) { 2 } else { 0 })
                    + (if endpoint_index == 0 { 1 } else { 0 });
                let Ok(control2) = control.cast::<IAudioSessionControl2>() else { continue; };
                let pid = unsafe { control2.GetProcessId().unwrap_or(0) };
                let Ok(volume) = control.cast::<ISimpleAudioVolume>() else { continue; };
                let level = unsafe { volume.GetMasterVolume().unwrap_or(1.0) };
                let muted = unsafe { volume.GetMute().map(|v| v.as_bool()).unwrap_or(false) }
                    || level <= f32::EPSILON;
                let meter = control.cast::<IAudioMeterInformation>().ok();
                let supplied_name = unsafe { control.GetDisplayName().ok() }
                    .map(|p| take_com_string(p.0)).unwrap_or_default();
                let icon = unsafe { control.GetIconPath().ok() }
                    .map(|p| take_com_string(p.0))
                    .filter(|s| !s.is_empty() && !s.to_ascii_lowercase().contains("webview2"));
                if pid != 0 { observed_pids.insert(pid); }
                // Packaged apps are identified by the Windows application ID,
                // not by their helper process name (for example, a .Root EXE).
                let identity = self.identities.resolve(pid);
                let executable = identity.executable;
                let aumid = identity.aumid;
                let filename = executable.as_deref()
                    .and_then(|p| std::path::Path::new(p).file_stem())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| if pid == 0 {"System sounds".into()} else {"Application".into()});
                let system_resource = supplied_name.to_ascii_lowercase().contains("audiosrv.dll,-202");
                let is_system_sounds = system_resource || pid == 0;
                let valid_session_name = !supplied_name.is_empty()
                    && !supplied_name.to_ascii_lowercase().contains("webview2")
                    && !supplied_name.starts_with('@');
                let name = if is_system_sounds {
                    "System sounds".to_owned()
                } else if let Some(official_name) = identity.display_name {
                    official_name
                } else if valid_session_name {
                    supplied_name
                } else { filename };
                let group_key = if is_system_sounds {
                    "system-sounds".to_owned()
                } else if let Some(id) = aumid.as_ref() {
                    format!("aumid:{}", id.to_lowercase())
                } else if let Some(path) = executable.as_ref() {
                    format!("app:{}", path.to_lowercase())
                } else {
                    format!("pid:{pid}")
                };
                if let Some(&ix) = groups.get(&group_key) {
                    let entry = &mut self.sessions[ix];
                    entry.volumes.push(volume);
                    if let Some(meter) = meter { entry.meters.push(meter); }
                    // Prefer active sessions; among equally active sessions,
                    // prefer the currently selected default endpoint.
                    if priority > priorities[ix] {
                        priorities[ix] = priority;
                        entry.level = level;
                        entry.muted = muted;
                    }
                    entry.is_system_sounds |= is_system_sounds;
                    if entry.session_icon.is_none() { entry.session_icon = icon; }
                    if entry.executable.is_none() { entry.executable = executable; }
                    if entry.app_user_model_id.is_none() { entry.app_user_model_id = aumid; }
                } else {
                    let ix = self.sessions.len();
                    let previous_peak = previous_peaks.get(&group_key).copied().unwrap_or(0.0);
                    groups.insert(group_key.clone(), ix);
                    priorities.push(priority);
                    self.sessions.push(Session {
                        key: group_key, name, executable, app_user_model_id: aumid,
                        session_icon: icon, is_system_sounds,
                        volumes: vec![volume], meters: meter.into_iter().collect(),
                        peak: previous_peak,
                        level, muted,
                    });
                }
            }
        }
        self.identities.retain_active(&observed_pids);
        self.sessions.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(())
    }
    fn clear_output_state(&mut self) {
        self.current_id.clear();
        self.devices.clear();
        self.sessions.clear();
        self.master_endpoint = None;
        self.master_meter = None;
        self.master_peak = 0.0;
        self.master_level = 0.0;
        self.master_muted = true;
    }

    pub fn current_device(&self) -> Option<&Device> {
        self.devices.iter().find(|d| d.current)
    }
    pub fn switch_to_next(&mut self, config: &Config) -> Result<Option<Device>, String> {
        // A hotkey can be pressed with the flyout hidden for hours. Never
        // switch using the stale device list captured on the last UI opening.
        self.refresh().map_err(|error| format!("Audio outputs are unavailable: {error}"))?;
        let enabled: Vec<_> = self.devices.iter().filter(|d| config.device_enabled(&d.id)).collect();
        if enabled.len() < 2 { return Ok(None); }
        let index = enabled.iter().position(|d| d.id == self.current_id);
        let next = enabled[index.map(|i| (i+1) % enabled.len()).unwrap_or(0)].clone();
        set_default_output(&next.id)?;
        self.refresh().map_err(|e| e.to_string())?;
        Ok(Some(next))
    }
    pub fn sample_peaks(&mut self) -> bool {
        let mut changed = false;
        let master_raw = self.master_meter.as_ref()
            .and_then(|meter| unsafe { meter.GetPeakValue().ok() }).unwrap_or(0.0);
        // Meter readings are an activity measurement, not a promise of exact
        // speaker loudness. Scale the visual activity by the corresponding
        // Windows volume control and never allow its decay to exceed the thumb.
        let target = effective_peak(master_raw, self.master_level, self.master_muted);
        let next = smooth_peak(self.master_peak, target).min(unit(self.master_level));
        changed |= (next - self.master_peak).abs() >= 0.003;
        self.master_peak = next;
        for session in &mut self.sessions {
            let raw = session.meters.iter()
                .filter_map(|meter| unsafe { meter.GetPeakValue().ok() })
                .fold(0.0_f32, f32::max);
            // Previously the peak was displayed independent of session volume:
            // Chrome at 3% could still have a nearly full-width visualizer.
            // Approximate the post-session-gain signal for display instead.
            // Do not apply the default endpoint gain to every session here:
            // grouped apps can still be rendering to a different endpoint.
            let target = effective_peak(raw, session.level,
                session.muted || self.master_muted);
            let next = smooth_peak(session.peak, target).min(unit(session.level));
            changed |= (next - session.peak).abs() >= 0.003;
            session.peak = next;
        }
        changed
    }
    pub fn set_level(&mut self, index: usize, value: f32) {
        if let Some(session) = self.sessions.get_mut(index) {
            let value = value.clamp(0.0, 1.0);
            let mute = value <= f32::EPSILON;
            for v in &session.volumes {
                let _ = unsafe { v.SetMasterVolume(value, ptr::null()) };
                let _ = unsafe { v.SetMute(mute, ptr::null()) };
            }
            session.level = value;
            session.muted = mute;
            // Keep the visualizer inside the new thumb immediately, even while
            // slider changes are still waiting for the next meter sample.
            session.peak = if mute { 0.0 } else { session.peak.min(value) };
        }
    }
    pub fn toggle_mute(&mut self, index: usize) {
        if let Some(session) = self.sessions.get_mut(index) {
            let mute = !session.muted;
            for v in &session.volumes { let _ = unsafe { v.SetMute(mute, ptr::null()) }; }
            session.muted = mute;
        }
    }
    pub fn set_master(&mut self, value: f32) {
        if let Some(endpoint) = &self.master_endpoint {
            let value = value.clamp(0.0, 1.0);
            let mute = value <= f32::EPSILON;
            let _ = unsafe { endpoint.SetMasterVolumeLevelScalar(value, ptr::null()) };
            let _ = unsafe { endpoint.SetMute(mute, ptr::null()) };
            self.master_level = value;
            self.master_muted = mute;
            self.master_peak = if mute { 0.0 } else { self.master_peak.min(value) };
        }
    }
    pub fn toggle_master_mute(&mut self) {
        if let Some(endpoint) = &self.master_endpoint {
            let mute = !self.master_muted;
            if unsafe { endpoint.SetMute(mute, ptr::null()) }.is_ok() { self.master_muted = mute; }
        }
    }
}

// Normalize Windows meter and slider values before applying gain. This also
// protects the renderer from an unexpected NaN/Infinity returned by a driver.
fn unit(value: f32) -> f32 {
    if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 }
}

// Approximation of audible activity after the per-session volume control.
// This is not a calibrated post-DSP loudness meter: some drivers/mixers can
// expose peaks at different points in the signal path. Its useful invariant
// is that the visualizer never exceeds the selected session volume.
fn effective_peak(raw: f32, volume: f32, muted: bool) -> f32 {
    if muted { 0.0 } else { unit(raw) * unit(volume) }
}

// Fast attack and slower release prevents flicker on quiet samples.
fn smooth_peak(previous: f32, measured: f32) -> f32 {
    let measured = unit(measured);
    if measured >= previous { measured } else { (previous * 0.72).max(measured) }
}

#[cfg(test)]
mod peak_tests {
    use super::{effective_peak, smooth_peak, unit};

    #[test]
    fn chrome_visualizer_obeys_3_and_65_percent_levels() {
        let raw = 0.90;
        let quiet = effective_peak(raw, 0.03, false);
        let loud = effective_peak(raw, 0.65, false);
        assert!((quiet - 0.027).abs() < 0.00001);
        assert!((loud - 0.585).abs() < 0.00001);
        assert!(quiet <= 0.03 && loud <= 0.65 && loud > quiet);
    }

    #[test]
    fn meter_is_silent_when_muted_or_volume_zero() {
        assert_eq!(effective_peak(1.0, 1.0, true), 0.0);
        assert_eq!(effective_peak(1.0, 0.0, false), 0.0);
        assert_eq!(effective_peak(0.0, 1.0, false), 0.0);
    }

    #[test]
    fn invalid_or_out_of_range_values_are_safe() {
        assert_eq!(unit(f32::NAN), 0.0);
        assert_eq!(unit(f32::INFINITY), 0.0);
        assert_eq!(effective_peak(2.0, 0.50, false), 0.50);
        assert_eq!(effective_peak(-1.0, 1.0, false), 0.0);
    }

    #[test]
    fn decays_without_inventing_audio() {
        assert_eq!(smooth_peak(0.0, 0.0), 0.0);
        assert!(smooth_peak(0.8, 0.0) < 0.8);
        assert_eq!(smooth_peak(0.2, 0.9), 0.9);
    }
}

fn device_id(device: &IMMDevice) -> windows::core::Result<String> {
    let p = unsafe { device.GetId()? };
    Ok(take_com_string(p.0))
}
fn take_com_string(raw: *mut u16) -> String {
    if raw.is_null() { return String::new(); }
    let result = unsafe {
        let len = (0..32768).position(|i| *raw.add(i) == 0).unwrap_or(0);
        String::from_utf16_lossy(std::slice::from_raw_parts(raw, len))
    };
    unsafe { CoTaskMemFree(raw.cast()); }
    result
}
fn friendly_name(device: &IMMDevice) -> Option<String> {
    let store = unsafe { device.OpenPropertyStore(STGM_READ).ok()? };
    let value = unsafe { store.GetValue(&PKEY_FRIENDLY_NAME).ok()? };
    let mut buffer = [0u16; 512];
    let hr = unsafe { PropVariantToString((&value as *const _) as *const c_void, buffer.as_mut_ptr(), 512) };
    if hr < 0 { return None; }
    let len = buffer.iter().position(|&x| x == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]))
}

// IPolicyConfig is undocumented. Kept separate so we can replace/disable it
// cleanly if Windows changes. No external utility or service is launched.
#[repr(C)]
struct PolicyConfig { vtable: *const PolicyVtable }
#[repr(C)]
struct PolicyVtable {
    query: usize, addref: usize,
    release: unsafe extern "system" fn(*mut PolicyConfig) -> u32,
    preceding_methods: [usize; 10],
    set_default_endpoint: unsafe extern "system" fn(*mut PolicyConfig, *const u16, i32) -> i32,
    set_visibility: usize,
}
fn set_default_output(id: &str) -> Result<(), String> {
    let mut ptr: *mut c_void = ptr::null_mut();
    let hr = unsafe {
        CoCreateInstance(&POLICY_CLSID, ptr::null_mut(), 1,
            &POLICY_IID, &mut ptr)
    };
    if hr < 0 || ptr.is_null() { return Err(format!("Windows output switching unavailable: 0x{:08X}", hr as u32)); }
    let policy = ptr as *mut PolicyConfig;
    let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
    let mut error = None;
    unsafe {
        // eConsole (0) and eMultimedia (1) follow the selected output.
        // Keep eCommunications (2) untouched: wireless headsets often expose
        // separate Game and Chat render devices, and changing Chat breaks calls.
        for role in 0..=1 {
            let result = ((*(*policy).vtable).set_default_endpoint)(policy, wide.as_ptr(), role);
            if result < 0 { error = Some(format!("Cannot change output: 0x{:08X}", result as u32)); break; }
        }
        ((*(*policy).vtable).release)(policy);
    }
    error.map_or(Ok(()), Err)
}
