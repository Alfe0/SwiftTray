//! Windows screen capture hotkey dispatch for an unpackaged Win32 application.
//!
//! ShellExecuteW of a Snipping Tool URI can open the main Snipping Tool window
//! instead of the capture overlay. For the same behavior as Win+Shift+S, send
//! that key chord to Windows after the configured shortcut has been released.
//! There is no timer, arbitrary delay, or flyout hiding in this module.

use crate::{config::Shortcut, native::*};
use std::{mem, sync::atomic::{AtomicIsize, AtomicU32, Ordering}};

pub const MSG_CAPTURE_KEYS_RELEASED: u32 = WM_APP + 10;

static CAPTURE_WINDOW: AtomicIsize = AtomicIsize::new(0);
static CAPTURE_HOOK: AtomicIsize = AtomicIsize::new(0);
static CAPTURE_KEY: AtomicU32 = AtomicU32::new(0);
static CAPTURE_MODIFIERS: AtomicU32 = AtomicU32::new(0);

const VK_LSHIFT: i32 = 0xA0;
const VK_RSHIFT: i32 = 0xA1;
const VK_LCONTROL: i32 = 0xA2;
const VK_RCONTROL: i32 = 0xA3;
const VK_LMENU: i32 = 0xA4;
const VK_RMENU: i32 = 0xA5;

fn down(virtual_key: i32, just_released: u32) -> bool {
    // Low-level keyboard hooks run before Windows updates GetAsyncKeyState.
    // Treat the currently reported key-up as released already.
    virtual_key as u32 != just_released
        && (unsafe { GetAsyncKeyState(virtual_key) }) < 0
}

fn modifier_down(left: i32, right: i32, just_released: u32) -> bool {
    down(left, just_released) || down(right, just_released)
}

fn shortcut_released(shortcut: Shortcut, just_released: u32) -> bool {
    !down(shortcut.key as i32, just_released)
        && (shortcut.modifiers & 0x0001 == 0
            || !modifier_down(VK_LMENU, VK_RMENU, just_released))
        && (shortcut.modifiers & 0x0002 == 0
            || !modifier_down(VK_LCONTROL, VK_RCONTROL, just_released))
        && (shortcut.modifiers & 0x0004 == 0
            || !modifier_down(VK_LSHIFT, VK_RSHIFT, just_released))
        && (shortcut.modifiers & 0x0008 == 0
            || (!down(VK_LWIN, just_released) && !down(VK_RWIN, just_released)))
}

unsafe extern "system" fn release_hook(code: i32, message: usize, data: isize) -> isize {
    if code == 0 && data != 0
        && (message as u32 == WM_KEYUP || message as u32 == WM_SYSKEYUP)
    {
        let released = (*(data as *const KeyboardLowLevelData)).vk_code;
        let shortcut = Shortcut {
            key: CAPTURE_KEY.load(Ordering::Relaxed),
            modifiers: CAPTURE_MODIFIERS.load(Ordering::Relaxed),
        };
        let window = CAPTURE_WINDOW.load(Ordering::Relaxed);
        if window != 0 && shortcut_released(shortcut, released) {
            // Message is queued rather than sending injected input inside a hook.
            CAPTURE_WINDOW.store(0, Ordering::Relaxed);
            PostMessageW(window, MSG_CAPTURE_KEYS_RELEASED, 0, 0);
        }
    }
    CallNextHookEx(0, code, message, data)
}

pub fn request_capture_after_release(window: H, shortcut: Shortcut) -> Result<(), String> {
    if CAPTURE_HOOK.load(Ordering::Relaxed) != 0 {
        return Err("Screen capture shortcut is already pending".into());
    }
    CAPTURE_KEY.store(shortcut.key, Ordering::Relaxed);
    CAPTURE_MODIFIERS.store(shortcut.modifiers, Ordering::Relaxed);
    CAPTURE_WINDOW.store(window, Ordering::Relaxed);
    if shortcut_released(shortcut, 0) {
        CAPTURE_WINDOW.store(0, Ordering::Relaxed);
        unsafe { PostMessageW(window, MSG_CAPTURE_KEYS_RELEASED, 0, 0); }
        return Ok(());
    }
    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(release_hook), GetModuleHandleW(std::ptr::null()), 0) };
    if hook == 0 {
        CAPTURE_WINDOW.store(0, Ordering::Relaxed);
        return Err("Unable to observe capture shortcut release".into());
    }
    CAPTURE_HOOK.store(hook, Ordering::Relaxed);
    Ok(())
}

pub fn disarm() {
    CAPTURE_WINDOW.store(0, Ordering::Relaxed);
    let hook = CAPTURE_HOOK.swap(0, Ordering::Relaxed);
    if hook != 0 { unsafe { UnhookWindowsHookEx(hook); } }
}

const KEYS: [(u16, u32); 6] = [
    (VK_LWIN as u16, 0),
    (VK_SHIFT as u16, 0),
    (b'S' as u16, 0),
    (b'S' as u16, KEYEVENTF_KEYUP),
    (VK_SHIFT as u16, KEYEVENTF_KEYUP),
    (VK_LWIN as u16, KEYEVENTF_KEYUP),
];

pub fn start_screen_snip() -> Result<(), String> {
    let mut events: Vec<WinInput> = KEYS.iter().map(|&(key, flags)| WinInput {
        kind: INPUT_KEYBOARD,
        data: WinInputData { keyboard: KeyboardInput {
            virtual_key: key, scan: 0, flags, time: 0, extra_info: 0,
        } },
    }).collect();
    let sent = unsafe {
        SendInput(events.len() as u32, events.as_mut_ptr(), mem::size_of::<WinInput>() as i32)
    };
    if sent == events.len() as u32 {
        Ok(())
    } else {
        Err("Windows blocked the capture keyboard shortcut. Elevated windows may restrict input injection.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_chord_is_win_shift_s_with_reverse_release_order() {
        assert_eq!(KEYS[0], (VK_LWIN as u16, 0));
        assert_eq!(KEYS[1], (VK_SHIFT as u16, 0));
        assert_eq!(KEYS[2], (b'S' as u16, 0));
        assert_eq!(KEYS[3], (b'S' as u16, KEYEVENTF_KEYUP));
        assert_eq!(KEYS[4], (VK_SHIFT as u16, KEYEVENTF_KEYUP));
        assert_eq!(KEYS[5], (VK_LWIN as u16, KEYEVENTF_KEYUP));
    }
}
