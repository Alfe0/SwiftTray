//! Per-user Windows startup registration. No elevation, service, or scheduled task.
//! Only the current executable can be enabled; disabling removes SwiftTray's Run value.
use crate::native::wide;
use std::{env, ptr};

type Key = isize;
const HKEY_CURRENT_USER: Key = 0x8000_0001_u32 as i32 as isize;
const KEY_QUERY_VALUE: u32 = 0x0001;
const KEY_SET_VALUE: u32 = 0x0002;
const REG_SZ: u32 = 1;
const NOT_FOUND: i32 = 2;
const PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const ENTRY: &str = "SwiftTray";

#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(root: Key, subkey: *const u16, options: u32, access: u32, result: *mut Key) -> i32;
    fn RegCreateKeyExW(root: Key, subkey: *const u16, reserved: u32, class: *mut u16,
        options: u32, access: u32, security: *const std::ffi::c_void,
        result: *mut Key, disposition: *mut u32) -> i32;
    fn RegQueryValueExW(key: Key, value_name: *const u16, reserved: *mut u32,
        value_type: *mut u32, data: *mut u8, data_len: *mut u32) -> i32;
    fn RegSetValueExW(key: Key, value_name: *const u16, reserved: u32,
        value_type: u32, data: *const u8, data_len: u32) -> i32;
    fn RegDeleteValueW(key: Key, value_name: *const u16) -> i32;
    fn RegCloseKey(key: Key) -> i32;
}

struct OpenKey(Key);
impl Drop for OpenKey {
    fn drop(&mut self) { unsafe { RegCloseKey(self.0); } }
}

fn current_command() -> Result<String, String> {
    let exe = env::current_exe().map_err(|e| format!("Could not locate SwiftTray.exe: {e}"))?;
    Ok(format!("\"{}\"", exe.to_string_lossy()))
}

fn open_key(access: u32, create: bool) -> Result<Option<OpenKey>, String> {
    let path = wide(PATH);
    let mut key: Key = 0;
    let result = unsafe {
        if create {
            RegCreateKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, ptr::null_mut(), 0,
                access, ptr::null(), &mut key, ptr::null_mut())
        } else {
            RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, access, &mut key)
        }
    };
    if result == 0 { Ok(Some(OpenKey(key))) }
    else if !create && result == NOT_FOUND { Ok(None) }
    else { Err(format!("Windows startup registry error: {result}")) }
}

// Windows Store MSIX has package identity and must use the declared StartupTask.
// Portable GitHub builds retain the per-user Run value; these paths must not mix.
const PACKAGE_NAME: &str = "mohammedalfehaid.swifttray_";
const TASK_ID: &str = "SwiftTrayStartup";
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentPackageFullName(length: *mut u32, name: *mut u16) -> i32;
}

pub fn is_store_install() -> bool {
    let mut length = 0u32;
    let status = unsafe { GetCurrentPackageFullName(&mut length, ptr::null_mut()) };
    if status != 122 || length == 0 || length > 4096 { return false; }
    let mut data = vec![0u16; length as usize];
    if unsafe { GetCurrentPackageFullName(&mut length, data.as_mut_ptr()) } != 0 { return false; }
    String::from_utf16_lossy(&data).to_ascii_lowercase().starts_with(PACKAGE_NAME)
}

fn packaged_task() -> Result<windows::ApplicationModel::StartupTask, String> {
    use windows::{ApplicationModel::StartupTask, core::HSTRING};
    StartupTask::GetAsync(&HSTRING::from(TASK_ID))
        .map_err(|e| format!("StartupTask not registered in this MSIX: {e}"))?
        .join().map_err(|e| format!("Could not read Windows startup state: {e}"))
}

fn packaged_is_enabled() -> Result<bool, String> {
    use windows::ApplicationModel::StartupTaskState;
    let state = packaged_task()?.State().map_err(|e| e.to_string())?;
    Ok(state == StartupTaskState::Enabled || state == StartupTaskState::EnabledByPolicy)
}

fn packaged_set_enabled(enabled: bool) -> Result<(), String> {
    use windows::ApplicationModel::StartupTaskState;
    let task = packaged_task()?;
    let state = task.State().map_err(|e| e.to_string())?;
    if enabled {
        if state == StartupTaskState::Enabled || state == StartupTaskState::EnabledByPolicy { return Ok(()); }
        if state == StartupTaskState::DisabledByUser {
            return Err("Windows disabled SwiftTray in Startup apps. Enable it there first.".into());
        }
        if state == StartupTaskState::DisabledByPolicy {
            return Err("Your Windows policy prevents startup apps.".into());
        }
        let new_state = task.RequestEnableAsync()
            .map_err(|e| format!("Could not request startup permission: {e}"))?
            .join().map_err(|e| format!("Windows denied startup permission: {e}"))?;
        if new_state == StartupTaskState::Enabled || new_state == StartupTaskState::EnabledByPolicy { Ok(()) }
        else { Err("Windows did not enable startup. Check Settings > Apps > Startup.".into()) }
    } else {
        task.Disable().map_err(|e| format!("Could not disable startup: {e}"))
    }
}

/// True only if the registered startup entry points to the currently running EXE.
pub fn is_enabled() -> bool {
    if is_store_install() { return packaged_is_enabled().unwrap_or(false); }
    let Ok(Some(key)) = open_key(KEY_QUERY_VALUE, false) else { return false; };
    let name = wide(ENTRY);
    let mut bytes = 0u32;
    let mut data_type = 0u32;
    let status = unsafe {
        RegQueryValueExW(key.0, name.as_ptr(), ptr::null_mut(), &mut data_type,
            ptr::null_mut(), &mut bytes)
    };
    if status != 0 || data_type != REG_SZ || bytes == 0 || bytes > 32_768 { return false; }
    let mut data = vec![0u16; (bytes as usize + 1) / 2];
    let status = unsafe {
        RegQueryValueExW(key.0, name.as_ptr(), ptr::null_mut(), &mut data_type,
            data.as_mut_ptr() as *mut u8, &mut bytes)
    };
    if status != 0 || data_type != REG_SZ { return false; }
    let value = String::from_utf16_lossy(&data);
    let value = value.trim_end_matches('\0');
    current_command().is_ok_and(|cmd| value.eq_ignore_ascii_case(&cmd))
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if is_store_install() { return packaged_set_enabled(enabled); }
    let name = wide(ENTRY);
    if enabled {
        let command = wide(&current_command()?);
        let key = open_key(KEY_SET_VALUE, true)?.ok_or("Startup registry key unavailable")?;
        let bytes = (command.len() * 2) as u32;
        let status = unsafe {
            RegSetValueExW(key.0, name.as_ptr(), 0, REG_SZ,
                command.as_ptr() as *const u8, bytes)
        };
        if status == 0 { Ok(()) }
        else { Err(format!("Could not enable startup (Windows error {status})")) }
    } else {
        let Some(key) = open_key(KEY_SET_VALUE, false)? else { return Ok(()); };
        let status = unsafe { RegDeleteValueW(key.0, name.as_ptr()) };
        if status == 0 || status == NOT_FOUND { Ok(()) }
        else { Err(format!("Could not disable startup (Windows error {status})")) }
    }
}
