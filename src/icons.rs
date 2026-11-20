//! App identity and native icon resolution using Windows Shell and AppModel.
//! No special-case app names, downloaded artwork, or WebView2 UI dependencies.
use std::{collections::{HashMap, HashSet}, ffi::c_void, mem, ptr};
use windows::Win32::UI::{Controls::IImageList, Shell::SHGetImageList};

const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const TH32CS_SNAPPROCESS: u32 = 0x0002;
const SHGFI_ICON: u32 = 0x0000_0100;
const SHGFI_DISPLAYNAME: u32 = 0x0000_0200;
const SHGFI_LARGEICON: u32 = 0;
const SHGFI_SYSICONINDEX: u32 = 0x0000_4000;
// 48x48 source icons are sharp at the 28-DIP UI footprint without caching
// 256x256 HICONs for dozens of audio applications.
const SHIL_EXTRALARGE: i32 = 2;
const ILD_TRANSPARENT: u32 = 0x0000_0001;
const SOURCE_ICON_SIZE: i32 = 64;
const SHGFI_PIDL: u32 = 0x0000_0008;
const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
const INVALID_HANDLE_VALUE: isize = -1;

#[repr(C)]
struct ProcessEntry {
    size: u32, usage: u32, process_id: u32, default_heap: usize,
    module_id: u32, threads: u32, parent_id: u32, base_priority: i32,
    flags: u32, name: [u16; 260],
}
#[repr(C)]
struct SHFileInfo {
    icon: isize, index: i32, attributes: u32,
    display_name: [u16; 260], type_name: [u16; 80],
}
#[link(name="kernel32")]
extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
    fn QueryFullProcessImageNameW(handle: isize, flags: u32, buffer: *mut u16, len: *mut u32) -> i32;
    fn GetApplicationUserModelId(handle: isize, len: *mut u32, data: *mut u16) -> i32;
    fn CloseHandle(handle: isize) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
    fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry) -> i32;
    fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry) -> i32;
    fn ExpandEnvironmentStringsW(src: *const u16, dst: *mut u16, len: u32) -> u32;
}
#[link(name="shell32")]
extern "system" {
    fn SHGetFileInfoW(path: *const u16, attrs: u32, fileinfo: *mut SHFileInfo, cb: u32, flags: u32) -> usize;
    fn SHParseDisplayName(name: *const u16, bind_ctx: *mut c_void, pidl: *mut *mut c_void,
        flags: u32, attributes: *mut u32) -> i32;
    fn ExtractIconExW(file: *const u16, index: i32, large: *mut isize, small: *mut isize, count: u32) -> u32;
}
#[link(name="ole32")]
extern "system" { fn CoTaskMemFree(ptr: *mut c_void); }
#[link(name="user32")]
extern "system" {
    fn DestroyIcon(icon: isize) -> i32;
    fn PrivateExtractIconsW(file: *const u16, index: i32, cx: i32, cy: i32,
        icons: *mut isize, ids: *mut u32, count: u32, flags: u32) -> u32;
}
#[link(name="version")]
extern "system" {
    fn GetFileVersionInfoSizeW(file: *const u16, handle: *mut u32) -> u32;
    fn GetFileVersionInfoW(file: *const u16, handle: u32, len: u32, data: *mut c_void) -> i32;
    fn VerQueryValueW(data: *const c_void, sub_block: *const u16,
        value: *mut *mut c_void, len: *mut u32) -> i32;
}

fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() }
fn utf16z(text: &[u16]) -> String {
    let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
    String::from_utf16_lossy(&text[..end])
}
fn is_engine(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with("\\msedgewebview2.exe") || lower.ends_with("/msedgewebview2.exe")
        || lower.ends_with("\\webview2loader.exe")
}

fn process_info(pid: u32) -> (Option<String>, Option<String>) {
    if pid == 0 { return (None, None); }
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 { return (None, None); }
    let mut buffer = [0u16; 32768];
    let mut len = buffer.len() as u32;
    let got_path = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut len) };
    let path = (got_path != 0).then(|| String::from_utf16_lossy(&buffer[..len as usize]));

    // An AppUserModelID is the system's identity for packaged apps, even when
    // the audio session belongs to a helper process with a misleading EXE name.
    let mut id_len = 0u32;
    let result = unsafe { GetApplicationUserModelId(handle, &mut id_len, ptr::null_mut()) };
    let app_id = if result == ERROR_INSUFFICIENT_BUFFER && (1..32768).contains(&id_len) {
        let mut id = vec![0u16; id_len as usize];
        let result = unsafe { GetApplicationUserModelId(handle, &mut id_len, id.as_mut_ptr()) };
        (result == 0).then(|| utf16z(&id)).filter(|s| !s.is_empty())
    } else { None };
    unsafe { CloseHandle(handle); }
    (path, app_id)
}

fn parent_id(pid: u32) -> Option<u32> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE { return None; }
    let mut entry: ProcessEntry = unsafe { mem::zeroed() };
    entry.size = mem::size_of::<ProcessEntry>() as u32;
    let mut parent = None;
    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
    while ok != 0 {
        if entry.process_id == pid { parent = Some(entry.parent_id); break; }
        ok = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    unsafe { CloseHandle(snapshot); }
    parent.filter(|&p| p != 0 && p != pid)
}

/// Windows' AppsFolder exposes the localized name and icon of packaged apps.
/// Both values come from Windows; no bundled application-name mappings exist.
fn shell_app(aumid: &str, want_icon: bool) -> (Option<String>, isize) {
    let parsing_name = wide(&format!("shell:AppsFolder\\{aumid}"));
    let mut pidl = ptr::null_mut();
    let parsed = unsafe {
        SHParseDisplayName(parsing_name.as_ptr(), ptr::null_mut(), &mut pidl, 0, ptr::null_mut())
    };
    if parsed < 0 || pidl.is_null() { return (None, 0); }
    let mut info: SHFileInfo = unsafe { mem::zeroed() };
    let flags = SHGFI_PIDL | SHGFI_DISPLAYNAME
        | if want_icon { SHGFI_SYSICONINDEX } else { 0 };
    let ok = unsafe {
        SHGetFileInfoW(pidl.cast::<u16>(), 0, &mut info,
            mem::size_of::<SHFileInfo>() as u32, flags)
    };
    let name = (ok != 0).then(|| utf16z(&info.display_name)).filter(|s| !s.is_empty());
    let icon = if want_icon && ok != 0 {
        // AppsFolder's cached 48px icon is the application's Windows identity,
        // not the image embedded in its Chromium/WebView helper process.
        let found = shell_image_list_icon(info.index);
        if found != 0 { found } else { pidl_icon(pidl) }
    } else { 0 };
    unsafe { CoTaskMemFree(pidl); }
    (name, icon)
}

// Query Windows' 48px native icon list instead of stretching the Shell's
// default 32px HICON. The returned HICON is *owned* by the caller and must be
// destroyed. COM image lists are scoped to this call.
fn shell_image_list_icon(index: i32) -> isize {
    if index < 0 { return 0; }
    let Ok(list) = (unsafe { SHGetImageList::<IImageList>(SHIL_EXTRALARGE) }) else { return 0; };
    unsafe { list.GetIcon(index, ILD_TRANSPARENT) }
        .map(|icon| icon.0 as isize).unwrap_or(0)
}

fn pidl_icon(pidl: *const c_void) -> isize {
    let mut info: SHFileInfo = unsafe { mem::zeroed() };
    let ok = unsafe {
        SHGetFileInfoW(pidl.cast::<u16>(), 0, &mut info,
            mem::size_of::<SHFileInfo>() as u32,
            SHGFI_PIDL | SHGFI_ICON | SHGFI_LARGEICON)
    };
    if ok == 0 { 0 } else { info.icon }
}

fn file_icon_at_size(path: &str, index: i32) -> isize {
    let mut icon = 0isize;
    let result = unsafe {
        PrivateExtractIconsW(wide(path).as_ptr(), index, SOURCE_ICON_SIZE,
            SOURCE_ICON_SIZE, &mut icon, ptr::null_mut(), 1, 0)
    };
    if result == 1 { icon } else {
        if icon != 0 { unsafe { DestroyIcon(icon); } }
        0
    }
}

fn file_description(path: &str) -> Option<String> {
    let path = wide(path);
    let size = unsafe { GetFileVersionInfoSizeW(path.as_ptr(), ptr::null_mut()) };
    if size == 0 || size > 4 * 1024 * 1024 { return None; }
    let mut blob = vec![0u8; size as usize];
    if unsafe { GetFileVersionInfoW(path.as_ptr(), 0, size, blob.as_mut_ptr().cast()) } == 0 {
        return None;
    }
    // Prefer the file's declared language/codepage; many desktop apps ship
    // en-US only, so include it as a legitimate fallback.
    let mut value: *mut c_void = ptr::null_mut();
    let mut byte_count = 0u32;
    let mut languages = Vec::new();
    if unsafe { VerQueryValueW(blob.as_ptr().cast(), wide("\\VarFileInfo\\Translation").as_ptr(),
        &mut value, &mut byte_count) } != 0 && !value.is_null() && byte_count >= 4 {
        let data = unsafe { std::slice::from_raw_parts(value.cast::<u16>(), (byte_count as usize / 2).min(16)) };
        for pair in data.chunks_exact(2) { languages.push((pair[0], pair[1])); }
    }
    languages.push((0x0409, 0x04B0));
    languages.push((0x0409, 0x04E4));
    for (language, codepage) in languages {
        let key = wide(&format!("\\StringFileInfo\\{language:04x}{codepage:04x}\\FileDescription"));
        let mut ptr = ptr::null_mut();
        let mut count = 0u32;
        if unsafe { VerQueryValueW(blob.as_ptr().cast(), key.as_ptr(), &mut ptr, &mut count) } != 0
            && !ptr.is_null() && count > 1 && count < 512 {
            let text = unsafe { std::slice::from_raw_parts(ptr.cast::<u16>(), count as usize) };
            let description = utf16z(text);
            if !description.trim().is_empty() { return Some(description); }
        }
    }
    None
}

#[derive(Clone, Default)]
pub struct ProcessIdentity {
    pub executable: Option<String>,
    pub aumid: Option<String>,
    pub display_name: Option<String>,
}

/// Reuse expensive AppModel/Shell queries while an audio process is alive.
/// The cache is trimmed to PIDs actually observed at every audio refresh.
#[derive(Default)]
pub struct IdentityResolver {
    by_pid: HashMap<u32, ProcessIdentity>,
    shell_names: HashMap<String, Option<String>>,
}
impl IdentityResolver {
    pub fn resolve(&mut self, pid: u32) -> ProcessIdentity {
        if let Some(previous) = self.by_pid.get(&pid) { return previous.clone(); }
        let (initial_path, initial_id) = process_info(pid);
        let mut executable = initial_path.clone();
        let mut aumid = initial_id;
        if initial_path.as_deref().is_some_and(is_engine) {
            // Only follow helpers to their owners. Never arbitrarily substitute
            // an unrelated parent's icon for a normal desktop application.
            let mut parent = parent_id(pid);
            for _ in 0..5 {
                let Some(p) = parent else { break; };
                let (path, parent_id_value) = process_info(p);
                if aumid.is_none() { aumid = parent_id_value; }
                if path.as_deref().is_some_and(|s| !is_engine(s)) {
                    executable = path;
                    break;
                }
                parent = parent_id(p);
            }
        }
        let display_name = if let Some(id) = aumid.as_deref() {
            self.shell_names.entry(id.to_owned())
                .or_insert_with(|| shell_app(id, false).0).clone()
        } else { None };
        let display_name = display_name.or_else(|| executable.as_deref().and_then(file_description));
        let identity = ProcessIdentity { executable, aumid, display_name };
        self.by_pid.insert(pid, identity.clone());
        identity
    }
    pub fn retain_active(&mut self, pids: &HashSet<u32>) {
        self.by_pid.retain(|pid, _| pids.contains(pid));
        // Keep historical app-name queries bounded on long-running tray sessions.
        if self.shell_names.len() > 256 { self.shell_names.clear(); }
    }
}

// Source icons are up to 64px and cost more than the old 32px frames.
// Keep the resident native-handle cache modest for an always-on tray app.
const MAX_CACHED_ICONS: usize = 128;

pub struct IconCache {
    cache: HashMap<String, isize>,
    raster: HashMap<isize, isize>,
}
impl IconCache {
    pub fn new() -> Self { Self { cache: HashMap::new(), raster: HashMap::new() } }
    /// Cached GDI+ source image for high-quality (and translucent) rendering.
    /// This is created lazily and reused across periodic meter repaints.
    pub fn raster_for(&mut self, icon:isize) -> isize {
        if icon==0 {return 0;}
        *self.raster.entry(icon).or_insert_with(|| crate::smooth::prepare_native_icon(icon))
    }
    pub fn icon_for(&mut self, app_id: Option<&str>, session_icon: Option<&str>, executable: Option<&str>) -> isize {
        if let Some(id) = app_id.filter(|x| !x.is_empty()) {
            let icon = self.load(&format!("aumid:{id}"), || shell_app(id, true).1);
            if icon != 0 { return icon; }
        }
        if let Some(resource) = session_icon.filter(|p| !p.is_empty()) {
            let icon = self.load(&format!("resource:{resource}"), || extract_resource_icon(resource));
            if icon != 0 { return icon; }
        }
        if let Some(path) = executable {
            return self.load(&format!("exe:{path}"), || shell_icon(path));
        }
        0
    }
    fn load(&mut self, key: &str, create: impl FnOnce() -> isize) -> isize {
        if let Some(&icon) = self.cache.get(key) { return icon; }
        // An always-on tray app can encounter many different programs over
        // months. Bound the lifetime of native HICON allocations rather than
        // retaining an unbounded number of icon handles.
        if self.cache.len() >= MAX_CACHED_ICONS { self.clear(); }
        let icon = create();
        self.cache.insert(key.to_owned(), icon);
        icon
    }
    fn clear(&mut self) {
        for (_, raster) in self.raster.drain() { crate::smooth::release_native_icon(raster); }
        for (_, icon) in self.cache.drain() {
            if icon != 0 { unsafe { DestroyIcon(icon); } }
        }
    }
}
impl Drop for IconCache {
    fn drop(&mut self) { self.clear(); }
}

#[cfg(test)]
mod cache_capacity_tests {
    use super::*;

    #[test]
    fn icon_cache_never_grows_without_bound() {
        let mut icons=IconCache::new();
        for n in 0..(MAX_CACHED_ICONS * 3) {
            icons.load(&format!("test:{n}"), || 0);
            assert!(icons.cache.len() <= MAX_CACHED_ICONS);
        }
    }
}
fn shell_icon(path: &str) -> isize {
    // Most desktop programs have multi-resolution .ico frames in the EXE.
    // Request 64px directly instead of upscaling SHGFI_LARGEICON's 32px frame.
    let native = file_icon_at_size(path, 0);
    if native != 0 { return native; }
    let mut info: SHFileInfo = unsafe { mem::zeroed() };
    let filename = wide(path);
    let indexed = unsafe {
        SHGetFileInfoW(filename.as_ptr(), 0, &mut info,
            mem::size_of::<SHFileInfo>() as u32, SHGFI_SYSICONINDEX)
    };
    if indexed != 0 {
        let icon = shell_image_list_icon(info.index);
        if icon != 0 { return icon; }
    }
    let ok = unsafe {
        SHGetFileInfoW(filename.as_ptr(), 0, &mut info,
            mem::size_of::<SHFileInfo>() as u32, SHGFI_ICON | SHGFI_LARGEICON)
    };
    if ok == 0 { 0 } else { info.icon }
}
fn extract_resource_icon(path: &str) -> isize {
    let (file, index) = match path.rsplit_once(',') {
        Some((a, b)) => (a.trim_matches('"'), b.trim().parse().unwrap_or(0)),
        None => (path, 0),
    };
    let expanded = {
        let raw = wide(file);
        let mut buffer = [0u16; 32768];
        let count = unsafe { ExpandEnvironmentStringsW(raw.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
        if count > 0 && (count as usize) < buffer.len() {
            String::from_utf16_lossy(&buffer[..count as usize - 1])
        } else { file.to_owned() }
    };
    let native = file_icon_at_size(&expanded, index);
    if native != 0 { return native; }
    let mut icon = 0;
    unsafe { ExtractIconExW(wide(&expanded).as_ptr(), index, &mut icon, ptr::null_mut(), 1); }
    icon
}
