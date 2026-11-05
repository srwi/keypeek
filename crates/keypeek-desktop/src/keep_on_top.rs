//! Keeps the overlay in front on Windows.
//!
//! The window asks for always-on-top once, when it is created. Windows does not
//! keep that promise: while KeyPeek runs, the window can drop out of the topmost
//! band (it then sits behind ordinary windows until KeyPeek is restarted), or be
//! covered by another topmost or full-screen window. Re-sending
//! `ViewportCommand::WindowLevel(AlwaysOnTop)` does not repair either, because
//! winit only calls `SetWindowPos` when its own cached level changes, and its
//! cache still says always-on-top.
//!
//! So this reads the real window instead. Each frame it checks the window's
//! `WS_EX_TOPMOST` style, and raises the window to the top of the topmost band
//! when the overlay appears or when the style is gone. It never activates or
//! focuses the window, and costs one style read per frame.

/// Whether to raise the window this frame.
///
/// Only a visible overlay is raised, since a hidden one has nothing to show.
/// It is raised each time it appears, which brings it back above anything that
/// covered it while it was hidden, and whenever it has lost always-on-top.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn should_raise(visible: bool, was_visible: bool, is_topmost: bool) -> bool {
    visible && (!was_visible || !is_topmost)
}

#[cfg(target_os = "windows")]
pub(crate) struct KeepOnTop {
    // As raw-window-handle gives it.
    hwnd: isize,
    was_visible: bool,
}

#[cfg(target_os = "windows")]
impl KeepOnTop {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Option<Self> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let Ok(RawWindowHandle::Win32(handle)) = cc.window_handle().map(|h| h.as_raw()) else {
            return None;
        };
        Some(Self {
            hwnd: handle.hwnd.get(),
            was_visible: false,
        })
    }

    /// Call once per frame with whether the overlay is visible.
    pub(crate) fn update(&mut self, visible: bool) {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
            SWP_NOSIZE, WS_EX_TOPMOST,
        };

        let hwnd = HWND(self.hwnd as *mut core::ffi::c_void);
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
        let is_topmost = ex_style & WS_EX_TOPMOST.0 != 0;

        if should_raise(visible, self.was_visible, is_topmost) {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
        self.was_visible = visible;
    }
}

#[cfg(test)]
mod tests {
    use super::should_raise;

    #[test]
    fn hidden_overlay_is_never_raised() {
        for was_visible in [false, true] {
            for is_topmost in [false, true] {
                assert!(!should_raise(false, was_visible, is_topmost));
            }
        }
    }

    #[test]
    fn overlay_is_raised_when_it_appears() {
        assert!(should_raise(true, false, true));
        assert!(should_raise(true, false, false));
    }

    #[test]
    fn overlay_that_lost_always_on_top_is_raised() {
        assert!(should_raise(true, true, false));
    }

    #[test]
    fn overlay_already_on_top_is_left_alone() {
        assert!(!should_raise(true, true, true));
    }
}
