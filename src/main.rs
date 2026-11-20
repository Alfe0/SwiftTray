#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("SwiftTray is a native Windows 11 application. Build it on Windows.");
}

#[cfg(windows)]
mod audio;
#[cfg(windows)]
mod config;
#[cfg(windows)]
mod capture;
#[cfg(windows)]
mod flyout_lifecycle;
#[cfg(windows)]
mod hotkeys;
#[cfg(windows)]
mod icons;
#[cfg(windows)]
mod native;
#[cfg(windows)]
mod singleton;
#[cfg(windows)]
mod smooth;
#[cfg(windows)]
mod startup;
#[cfg(windows)]
mod backdrop;
#[cfg(windows)]
mod brand;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod volume_glyph;
#[cfg(windows)]
mod updates;

#[cfg(windows)]
fn main() {
    if let Err(error) = ui::run() {
        eprintln!("SwiftTray: {error}");
        let text = native::wide(&error);
        let title = native::wide("SwiftTray");
        unsafe { native::MessageBoxW(0, text.as_ptr(), title.as_ptr(), 0x10); }
    }
}
