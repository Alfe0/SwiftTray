//! Windows 10/11 visual compatibility without redistributing Microsoft fonts.
//!
//! Segoe Fluent Icons and Segoe UI Variable ship with Windows 11, while
//! Windows 10 ships Segoe MDL2 Assets and the classic Segoe UI family.
//! RtlGetVersion reports the actual OS build, unlike manifest-dependent
//! legacy version APIs. Cache the result: Canvas is created on every paint.
use std::{mem, sync::OnceLock};

#[repr(C)]
struct OsVersionInfo {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    service_pack: [u16; 128],
}

#[link(name = "ntdll")]
extern "system" {
    fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
}

fn windows_11_or_later(major: u32, build: u32) -> bool {
    major >= 10 && build >= 22_000
}

pub fn uses_windows_11_fonts() -> bool {
    static IS_WINDOWS_11: OnceLock<bool> = OnceLock::new();
    *IS_WINDOWS_11.get_or_init(|| {
        let mut info: OsVersionInfo = unsafe { mem::zeroed() };
        info.size = mem::size_of::<OsVersionInfo>() as u32;
        // NTSTATUS == 0 indicates success. On failure, choose the older,
        // widely available fonts instead of silently drawing tofu squares.
        unsafe { RtlGetVersion(&mut info) == 0 && windows_11_or_later(info.major, info.build) }
    })
}

pub struct UiFontFamilies {
    pub text: &'static str,
    pub title: &'static str,
    pub icons: &'static str,
}

pub fn ui_fonts() -> UiFontFamilies {
    if uses_windows_11_fonts() {
        UiFontFamilies {
            text: "Segoe UI Variable Text",
            title: "Segoe UI Variable Display",
            icons: "Segoe Fluent Icons",
        }
    } else {
        UiFontFamilies {
            text: "Segoe UI",
            title: "Segoe UI",
            icons: "Segoe MDL2 Assets",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::windows_11_or_later;

    #[test]
    fn identifies_windows_10_and_windows_11_by_build() {
        assert!(!windows_11_or_later(10, 19_041)); // Test VM, Windows 10
        assert!(!windows_11_or_later(10, 19_045)); // Windows 10 22H2
        assert!(windows_11_or_later(10, 22_000));  // Windows 11 21H2
        assert!(windows_11_or_later(10, 26_300));  // New Windows 11
        assert!(!windows_11_or_later(6, 9_999));
    }
}
