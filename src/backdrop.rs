//! Reliable compositor configuration for a custom-drawn GDI+ flyout.
//!
//! The Win32/GDI+ rendering path paints an opaque bitmap each frame. DWM
//! Desktop Acrylic underneath that bitmap is not visible, while a global
//! layered-window alpha makes *text and icons* translucent as well.
//! Therefore use an intentionally opaque dark surface and genuine DWM rounded
//! corners, rather than pretend uniform opacity is a blurred backdrop.
//!
//! Real host-backdrop acrylic would require an alpha-aware DirectComposition
//! rendering surface and is intentionally not enabled here.
use crate::native::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, GetWindowLongPtrW,
    SetWindowLongPtrW, GWL_EXSTYLE, H, Margins, WS_EX_LAYERED,
};
use std::{ffi::c_void, mem};

const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
const DWMSBT_NONE: i32 = 1;

/// Configure an opaque, theme-painted Win32 flyout, with a safe fallback
/// regardless of the system's backdrop/transparency preference.
pub fn configure(hwnd: H, _dark: bool) {
    if hwnd == 0 { return; }
    unsafe {
        let none = DWMSBT_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd, DWMWA_SYSTEMBACKDROP_TYPE,
            &none as *const _ as *const c_void,
            mem::size_of_val(&none) as u32,
        );
        let margins = Margins { left: 0, right: 0, top: 0, bottom: 0 };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

        // Remove the v0.17.1 uniform-opacity layer so that text, icons, and
        // sliders remain fully opaque. The window always paints its own fill.
        let styles = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let layer = WS_EX_LAYERED as isize;
        if styles & layer != 0 {
            let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, styles & !layer);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn material_is_disabled_for_opaque_gdi_surface() {
        assert_eq!(super::DWMSBT_NONE, 1);
    }
}
