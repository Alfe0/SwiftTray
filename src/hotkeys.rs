//! Registration and dispatch of global keyboard shortcuts.
//!
//! Windows reserves unmodified F12 for debugger tooling. Only when explicitly
//! configured, SwiftTray uses a narrowly scoped WH_KEYBOARD_LL fallback for
//! that one key. All other shortcuts use RegisterHotKey.

use crate::{config::Shortcut, native::*};
use std::{ptr, sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering}};

pub const HOTKEY_POPUP: i32 = 101;
pub const HOTKEY_SWITCH: i32 = 102;
pub const HOTKEY_SNIP: i32 = 103;
pub const MSG_F12_SHORTCUT: u32 = WM_APP + 8;
pub const VK_F12: u32 = 0x7B;

static F12_ACTION: AtomicI32 = AtomicI32::new(-1);
static F12_WINDOW: AtomicIsize = AtomicIsize::new(0);
static F12_PRESSED: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn f12_keyboard_hook(code: i32, message: usize, data: isize) -> isize {
    if code == 0 && data != 0 {
        let key = (*(data as *const KeyboardLowLevelData)).vk_code;
        if key == VK_F12 {
            let is_up = message as u32 == WM_KEYUP || message as u32 == WM_SYSKEYUP;
            if is_up { F12_PRESSED.store(false, Ordering::Relaxed); }
            let no_modifiers = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
                .iter().all(|key| GetAsyncKeyState(*key) >= 0);
            if no_modifiers {
                if (message as u32 == WM_KEYDOWN || message as u32 == WM_SYSKEYDOWN)
                    && !F12_PRESSED.swap(true, Ordering::Relaxed)
                {
                    let window = F12_WINDOW.load(Ordering::Relaxed);
                    if window != 0 { PostMessageW(window, MSG_F12_SHORTCUT, 0, 0); }
                }
                return 1;
            }
        }
    }
    CallNextHookEx(0, code, message, data)
}

pub fn f12_action() -> i32 { F12_ACTION.load(Ordering::Relaxed) }

pub struct HotkeyManager { f12_hook: H }

impl HotkeyManager {
    pub const fn new() -> Self { Self { f12_hook: 0 } }

    pub fn unregister(&mut self, window: H, index: usize, shortcut: Shortcut) {
        if shortcut.key == VK_F12 && shortcut.modifiers == 0 && self.f12_hook != 0
            && f12_action() == index as i32
        {
            unsafe { UnhookWindowsHookEx(self.f12_hook); }
            self.f12_hook = 0;
            F12_ACTION.store(-1, Ordering::Relaxed);
            F12_WINDOW.store(0, Ordering::Relaxed);
            F12_PRESSED.store(false, Ordering::Relaxed);
        }
        unsafe { UnregisterHotKey(window, ids()[index]); }
    }

    pub fn register(&mut self, window: H, index: usize, shortcut: Shortcut) -> bool {
        if shortcut.key == VK_F12 && shortcut.modifiers == 0 {
            if self.f12_hook != 0 { return false; }
            let hook = unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(f12_keyboard_hook),
                    GetModuleHandleW(ptr::null()), 0)
            };
            if hook == 0 { return false; }
            self.f12_hook = hook;
            F12_WINDOW.store(window, Ordering::Relaxed);
            F12_ACTION.store(index as i32, Ordering::Relaxed);
            F12_PRESSED.store(false, Ordering::Relaxed);
            return true;
        }
        unsafe {
            RegisterHotKey(window, ids()[index], shortcut.modifiers | MOD_NOREPEAT,
                shortcut.key) != 0
        }
    }
}

const fn ids() -> [i32; 3] { [HOTKEY_POPUP, HOTKEY_SWITCH, HOTKEY_SNIP] }
