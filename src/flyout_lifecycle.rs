//! Event-driven flyout lifetime during native screen capture.
//!
//! A screen capture intentionally moves focus away from the mixer. Instead of
//! hiding the flyout or delaying that hide, this module tracks the capture
//! transition from Windows foreground events and clipboard changes. The normal
//! click-away close rule remains intact outside a capture session.

use crate::native::*;
use std::sync::atomic::{AtomicIsize, Ordering};

pub const MSG_CAPTURE_FOREGROUND: u32 = WM_APP + 9;
const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
static TARGET: AtomicIsize = AtomicIsize::new(0);

unsafe extern "system" fn foreground_callback(
    _hook: H, event: u32, foreground: H, _object: i32, _child: i32,
    _thread: u32, _timestamp: u32,
) {
    if event == EVENT_SYSTEM_FOREGROUND && foreground != 0 {
        let window = TARGET.load(Ordering::Relaxed);
        if window != 0 {
            PostMessageW(window, MSG_CAPTURE_FOREGROUND, foreground as usize, 0);
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum CaptureStage {
    Idle,
    AwaitingOverlay,
    OverlayForeground { process_id: u32 },
}

// Pure state transition: it does not sleep, poll, or manipulate windows.
fn advance_capture(stage: &mut CaptureStage, own_window: bool, pid: u32) -> bool {
    if *stage == CaptureStage::Idle { return false; }
    if own_window { return true; }
    match *stage {
        CaptureStage::Idle => false,
        CaptureStage::AwaitingOverlay => {
            *stage = CaptureStage::OverlayForeground { process_id: pid };
            false
        },
        CaptureStage::OverlayForeground { process_id } => pid != process_id,
    }
}

pub struct CaptureSession {
    stage: CaptureStage,
    foreground_hook: H,
    listening_clipboard: bool,
}

impl CaptureSession {
    pub const fn new() -> Self {
        Self { stage: CaptureStage::Idle, foreground_hook: 0, listening_clipboard: false }
    }

    pub fn active(&self) -> bool { self.stage != CaptureStage::Idle }
    pub fn suppress_focus_close(&self) -> bool { self.active() }

    pub fn begin(&mut self, window: H) -> Result<(), String> {
        self.finish(window);
        TARGET.store(window, Ordering::Relaxed);
        let hook = unsafe {
            SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, 0,
                Some(foreground_callback), 0, 0, WINEVENT_OUTOFCONTEXT)
        };
        if hook == 0 {
            TARGET.store(0, Ordering::Relaxed);
            return Err("Unable to track Snipping Tool foreground events".into());
        }
        self.foreground_hook = hook;
        self.listening_clipboard = unsafe { AddClipboardFormatListener(window) != 0 };
        self.stage = CaptureStage::AwaitingOverlay;
        Ok(())
    }

    /// Returns true when foreground leaves the capture overlay. This is an
    /// operating-system event, not a time-delayed estimate of capture duration.
    pub fn on_foreground(&mut self, window: H, foreground: H) -> bool {
        if !self.active() { return false; }
        let own_window = foreground == window;
        let mut pid = 0;
        if !own_window {
            unsafe { GetWindowThreadProcessId(foreground, &mut pid); }
            if pid == 0 { return false; }
        }
        if advance_capture(&mut self.stage, own_window, pid) {
            self.finish(window);
            return true;
        }
        false
    }

    pub fn finish(&mut self, window: H) {
        self.stage = CaptureStage::Idle;
        TARGET.store(0, Ordering::Relaxed);
        if self.foreground_hook != 0 {
            unsafe { UnhookWinEvent(self.foreground_hook); }
            self.foreground_hook = 0;
        }
        if self.listening_clipboard {
            unsafe { RemoveClipboardFormatListener(window); }
            self.listening_clipboard = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_keeps_flyout_visible_during_one_overlay_process() {
        let mut stage = CaptureStage::AwaitingOverlay;
        assert!(!advance_capture(&mut stage, false, 1234));
        assert_eq!(stage, CaptureStage::OverlayForeground { process_id: 1234 });
        assert!(!advance_capture(&mut stage, false, 1234));
        assert!(advance_capture(&mut stage, false, 2222));
    }

    #[test]
    fn returning_to_flyout_ends_capture_without_timeout() {
        let mut stage = CaptureStage::AwaitingOverlay;
        assert!(!advance_capture(&mut stage, false, 1234));
        assert!(advance_capture(&mut stage, true, 0));
    }

    #[test]
    fn idle_ignores_foreground_events() {
        let mut stage = CaptureStage::Idle;
        assert!(!advance_capture(&mut stage, false, 1234));
    }

    #[test]
    fn default_has_normal_click_away_behavior() {
        let session = CaptureSession::new();
        assert!(!session.active());
        assert!(!session.suppress_focus_close());
    }
}
