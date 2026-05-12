// windows_api.rs — Windows API bindings for showit
// Author: Hadi Cahyadi <cumulus13@gmail.com>
//
// On non-Windows platforms this module provides stub implementations
// so the crate can still compile and tests can run on Linux/macOS CI.

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: usize, // opaque handle, stored as usize for portability
    pub title: String,
}

// ─── Windows implementation ────────────────────────────────────────────────

#[cfg(windows)]
mod platform {
    use super::WindowInfo;
    use anyhow::{bail, Result};
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, IsWindowVisible,
        SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    // Fixed 512-wchar stack buffer — avoids GetWindowTextLengthW entirely.
    // GetWindowTextLengthW sends WM_GETTEXTLENGTH to every window (cross-thread
    // message pump round-trip). With hundreds of windows that's the main
    // source of startup lag. GetWindowTextW with a fixed buffer is non-blocking
    // for windows on other threads (it uses an internal timeout-free path).
    const BUF_LEN: usize = 512;

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let vec_ptr = lparam.0 as *mut Vec<WindowInfo>;
        if vec_ptr.is_null() {
            return BOOL(1);
        }

        // Skip invisible windows fast — no message send needed
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        // Stack-allocated buffer: no heap alloc per window
        let mut buf = [0u16; BUF_LEN];
        let written = GetWindowTextW(hwnd, &mut buf);
        if written <= 0 {
            return BOOL(1);
        }

        let title = OsString::from_wide(&buf[..written as usize])
            .to_string_lossy()
            .into_owned();

        if !title.is_empty() {
            (*vec_ptr).push(WindowInfo {
                hwnd: hwnd.0 as usize,
                title,
            });
        }

        BOOL(1)
    }

    pub fn enumerate_windows() -> Result<Vec<WindowInfo>> {
        let mut windows: Vec<WindowInfo> = Vec::with_capacity(128);
        let ptr = &mut windows as *mut Vec<WindowInfo>;
        unsafe {
            EnumWindows(Some(enum_proc), LPARAM(ptr as isize))?;
        }
        Ok(windows)
    }

    pub fn bring_to_front(info: &WindowInfo) -> Result<()> {
        let hwnd = HWND(info.hwnd as isize);
        unsafe {
            // Restore minimised windows before foregrounding
            ShowWindow(hwnd, SW_RESTORE);
            // SetForegroundWindow is async — it posts, doesn't block
            if !SetForegroundWindow(hwnd).as_bool() {
                bail!("SetForegroundWindow failed for '{}'", info.title);
            }
        }
        Ok(())
    }

    pub fn close_window(info: &WindowInfo) -> Result<()> {
        use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
        let hwnd = HWND(info.hwnd as isize);
        unsafe {
            // PostMessageW is fire-and-forget — never blocks
            PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0))?;
        }
        Ok(())
    }
}

// ─── Non-Windows stub ──────────────────────────────────────────────────────

#[cfg(not(windows))]
mod platform {
    use super::WindowInfo;
    use anyhow::{bail, Result};

    pub fn enumerate_windows() -> Result<Vec<WindowInfo>> {
        // On non-Windows, return a synthetic list so tests can exercise
        // the search / coloring logic without a real display.
        Ok(vec![
            WindowInfo { hwnd: 1, title: "Firefox — GitHub".into() },
            WindowInfo { hwnd: 2, title: "Visual Studio Code".into() },
            WindowInfo { hwnd: 3, title: "Windows Terminal".into() },
            WindowInfo { hwnd: 4, title: "Notepad — readme.txt".into() },
            WindowInfo { hwnd: 5, title: "Task Manager".into() },
        ])
    }

    pub fn bring_to_front(info: &WindowInfo) -> Result<()> {
        bail!("bring_to_front is not supported on this platform (hwnd={})", info.hwnd);
    }

    pub fn close_window(info: &WindowInfo) -> Result<()> {
        bail!("close_window is not supported on this platform (hwnd={})", info.hwnd);
    }
}

// ─── Public re-exports ─────────────────────────────────────────────────────

pub use platform::{bring_to_front, close_window, enumerate_windows};
