// windows_api.rs — Windows API bindings for showit
// Author: Hadi Cahyadi <cumulus13@gmail.com>
//
// On non-Windows platforms this module provides stub implementations
// so the crate can still compile and tests can run on Linux/macOS CI.

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: usize, // opaque handle, stored as usize for portability
    pub title: String,
    /// Executable name of the owning process, e.g. "wt.exe", "Code.exe".
    /// Empty string if it could not be determined.
    pub process_name: String,
    /// Win32 window class name, e.g. "CASCADIA_HOSTING_WINDOW_CLASS".
    /// Empty string if it could not be determined.
    pub class_name: String,
}

impl WindowInfo {
    /// A short human-readable "type" label to display instead of (or alongside)
    /// the raw title when the title alone does not identify the app reliably.
    ///
    /// Rules (first match wins):
    ///   1. Known class names that map to a canonical app label.
    ///   2. Known process names (exe stem, lower-case) that map to a label.
    ///   3. Fall back to the exe stem with the first letter capitalised.
    ///   4. Empty string if both process_name and class_name are unknown.
    pub fn type_label(&self) -> String {
        let class_lc = self.class_name.to_lowercase();
        let proc_stem = exe_stem(&self.process_name);

        // ── Class-based identification ────────────────────────────────────────
        // These window classes are stable identifiers regardless of title.
        let by_class: Option<&str> = match class_lc.as_str() {
            "cascadia_hosting_window_class" => Some("Windows Terminal"),
            "chrome_widgetwin_1" => Some("Chrome"),
            "mozillawindowclass" => Some("Firefox"),
            "operawindowclass" => Some("Opera"),
            "bravebrowser" => Some("Brave"),
            "vscode_main_window" => Some("VS Code"),
            "notepad++" => Some("Notepad++"),
            "konsole_mainwindow" => Some("Konsole"),
            "gnome-terminal-window" => Some("GNOME Terminal"),
            _ => None,
        };

        if let Some(label) = by_class {
            return label.to_string();
        }

        // ── Process-name-based identification ─────────────────────────────────
        let by_proc: Option<&str> = match proc_stem.as_str() {
            "wt" => Some("Windows Terminal"),
            "windowsterminal" => Some("Windows Terminal"),
            "code" => Some("VS Code"),
            "code - insiders" => Some("VS Code Insiders"),
            "firefox" => Some("Firefox"),
            "chrome" => Some("Chrome"),
            "msedge" => Some("Edge"),
            "brave" => Some("Brave"),
            "opera" => Some("Opera"),
            "notepad" => Some("Notepad"),
            "notepad++" => Some("Notepad++"),
            "powershell" => Some("PowerShell"),
            "pwsh" => Some("PowerShell"),
            "cmd" => Some("CMD"),
            "explorer" => Some("Explorer"),
            "slack" => Some("Slack"),
            "teams" => Some("Teams"),
            "discord" => Some("Discord"),
            "spotify" => Some("Spotify"),
            "rider64" | "rider" => Some("Rider"),
            "idea64" | "idea" => Some("IntelliJ IDEA"),
            "clion64" | "clion" => Some("CLion"),
            "pycharm64" | "pycharm" => Some("PyCharm"),
            "webstorm64" | "webstorm" => Some("WebStorm"),
            "devenv" => Some("Visual Studio"),
            "sublime_text" => Some("Sublime Text"),
            "atom" => Some("Atom"),
            "vim" | "gvim" | "nvim-qt" => Some("Vim"),
            "emacs" => Some("Emacs"),
            "obsidian" => Some("Obsidian"),
            "notion" => Some("Notion"),
            "thunderbird" => Some("Thunderbird"),
            "signal" => Some("Signal"),
            "telegram" => Some("Telegram"),
            _ => None,
        };

        if let Some(label) = by_proc {
            return label.to_string();
        }

        // ── Generic fallback: capitalised exe stem ────────────────────────────
        if proc_stem.is_empty() {
            String::new()
        } else {
            let mut chars = proc_stem.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
    }

    /// Returns true when multiple windows of the same type are expected to
    /// have completely different (user-chosen) titles, so the list should
    /// show the type label prominently rather than relying on the title alone.
    pub fn title_is_unreliable(&self) -> bool {
        let class_lc = self.class_name.to_lowercase();
        let proc_stem = exe_stem(&self.process_name);

        matches!(
            class_lc.as_str(),
            "cascadia_hosting_window_class" // wt: tab titles are user-set
        ) || matches!(
            proc_stem.as_str(),
            "wt" | "windowsterminal" | "powershell" | "pwsh" | "cmd" | "vim" | "gvim" | "nvim-qt"
        )
    }
}

/// Extract the lowercase stem of a process name: "wt.exe" → "wt"
fn exe_stem(process_name: &str) -> String {
    std::path::Path::new(process_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
}

// ─── Windows implementation ────────────────────────────────────────────────

#[cfg(windows)]
mod platform {
    use super::WindowInfo;
    use anyhow::{bail, Result};
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
    use windows::Win32::System::Console::GetConsoleWindow;
    use windows::Win32::System::Threading::{
        AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_MENU};
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowPlacement,
        GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow,
        SetWindowPos, ShowWindow, HWND_TOP, SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_SHOWWINDOW, SW_MAXIMIZE, SW_RESTORE, SW_SHOW,
        WINDOWPLACEMENT,
    };

    // Fixed 512-wchar stack buffer — avoids GetWindowTextLengthW entirely.
    // GetWindowTextLengthW sends WM_GETTEXTLENGTH to every window (cross-thread
    // message pump round-trip). With hundreds of windows that's the main
    // source of startup lag. GetWindowTextW with a fixed buffer is non-blocking
    // for windows on other threads (it uses an internal timeout-free path).
    const BUF_LEN: usize = 512;

    /// Retrieve the exe base-name (e.g. "wt.exe") for the given HWND.
    unsafe fn get_process_name(hwnd: HWND) -> String {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return String::new();
        }
        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => return String::new(),
        };
        let mut buf = [0u16; BUF_LEN];
        let mut size = BUF_LEN as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        if ok.is_err() || size == 0 {
            return String::new();
        }
        let path = OsString::from_wide(&buf[..size as usize])
            .to_string_lossy()
            .into_owned();
        std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }

    /// Retrieve the Win32 class name for the given HWND.
    unsafe fn get_class_name(hwnd: HWND) -> String {
        let mut buf = [0u16; 256];
        let written = GetClassNameW(hwnd, &mut buf);
        if written <= 0 {
            return String::new();
        }
        OsString::from_wide(&buf[..written as usize])
            .to_string_lossy()
            .into_owned()
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let vec_ptr = lparam.0 as *mut Vec<WindowInfo>;
        if vec_ptr.is_null() {
            return BOOL(1);
        }

        // Skip invisible windows fast — no message send needed
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        // Skip our own console/host window. GetConsoleWindow() returns the
        // HWND of the console we (showit) are attached to — under cmd.exe
        // this is the same window the user typed the command into, so
        // without this check showit would list itself whenever the query
        // matched the console's own title (e.g. "cmd", the current
        // directory, or a custom prompt title).
        if hwnd == GetConsoleWindow() {
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
            let process_name = get_process_name(hwnd);
            let class_name = get_class_name(hwnd);
            (*vec_ptr).push(WindowInfo {
                hwnd: hwnd.0 as usize,
                title,
                process_name,
                class_name,
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

    /// Bring a window to the foreground and give it real input focus.
    ///
    /// Algorithm:
    ///
    /// 1. Snapshot `WINDOWPLACEMENT` so we know the current `showCmd`.
    /// 2. Attach our input queue to the foreground thread (and the target
    ///    thread) so window/focus state is shared with them.
    /// 3. If the window is iconic (minimised) → `SW_RESTORE`. Otherwise
    ///    → `SW_SHOW`. Both activate the window (unlike the old
    ///    `SW_SHOWNA`/`SW_SHOWNOACTIVATE`, which deliberately never did).
    /// 4. Synthesize a harmless ALT keypress, then call
    ///    `SetForegroundWindow`. Windows only grants a foreground switch to
    ///    a process that "recently received input" — a console child
    ///    process of `cmd.exe`/`conhost.exe` never satisfies that (conhost,
    ///    not showit.exe, received the keystrokes), so a plain
    ///    `SetForegroundWindow` call is silently ignored there even though
    ///    it succeeds when showit is run from Windows Terminal (which
    ///    relays that permission to child processes). The synthetic
    ///    keypress is the standard, widely-used workaround: it makes our
    ///    thread look like it just received input, so the OS grants the
    ///    switch regardless of which shell launched us.
    /// 5. Detach input queues (always, even on error).
    /// 6. If the window was **not** maximised and was **not** minimised,
    ///    restore the original placement so position/size are exactly as
    ///    before. Skip it when maximised (`SetWindowPlacement` would force
    ///    it back to restored size) and skip it when it was minimised
    ///    (the snapshot still has the old minimised `showCmd`, and
    ///    replaying it would immediately re-minimise the window we just
    ///    restored and activated).
    pub fn bring_to_front(info: &WindowInfo) -> Result<()> {
        let hwnd = HWND(info.hwnd as isize);

        unsafe {
            // ── 1. Snapshot current placement ────────────────────────────────
            let mut wp = WINDOWPLACEMENT {
                length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            if GetWindowPlacement(hwnd, &mut wp).is_err() {
                bail!("GetWindowPlacement failed for '{}'", info.title);
            }
            let original_show_cmd = wp.showCmd;
            let was_iconic = IsIconic(hwnd).as_bool();

            // ── 2. Attach input queues ────────────────────────────────────────
            let our_tid = GetCurrentThreadId();
            let fg_hwnd = GetForegroundWindow();
            let fg_tid = GetWindowThreadProcessId(fg_hwnd, None);
            let target_tid = GetWindowThreadProcessId(hwnd, None);

            let attached_fg = fg_tid != 0
                && fg_tid != our_tid
                && AttachThreadInput(our_tid, fg_tid, true).as_bool();

            let attached_target = target_tid != 0
                && target_tid != our_tid
                && target_tid != fg_tid
                && AttachThreadInput(our_tid, target_tid, true).as_bool();

            // ── 3 & 4. Actually activate and raise the window ─────────────────
            let show_result = (|| -> Result<()> {
                if was_iconic {
                    ShowWindow(hwnd, SW_RESTORE);
                } else {
                    ShowWindow(hwnd, SW_SHOW);
                }

                keybd_event(VK_MENU.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
                let _ = BringWindowToTop(hwnd);
                let activated = SetForegroundWindow(hwnd).as_bool();
                keybd_event(VK_MENU.0 as u8, 0, KEYEVENTF_KEYUP, 0);

                if !activated {
                    // Fallback: at least raise the z-order even if we
                    // couldn't steal focus outright.
                    SetWindowPos(
                        hwnd,
                        HWND_TOP,
                        0,
                        0,
                        0,
                        0,
                        SET_WINDOW_POS_FLAGS(
                            SWP_NOSIZE.0
                                | SWP_NOMOVE.0
                                | SWP_NOACTIVATE.0
                                | SWP_NOOWNERZORDER.0
                                | SWP_SHOWWINDOW.0,
                        ),
                    )?;
                }

                Ok(())
            })();

            // ── 5. Always detach ──────────────────────────────────────────────
            if attached_fg {
                let _ = AttachThreadInput(our_tid, fg_tid, false);
            }
            if attached_target {
                let _ = AttachThreadInput(our_tid, target_tid, false);
            }

            show_result?;

            // ── 6. Restore placement only when NOT maximised and NOT
            //       previously minimised ─────────────────────────────────────
            if original_show_cmd != SW_MAXIMIZE.0 as u32 && !was_iconic {
                // best-effort; ignore errors (window may have moved legitimately)
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPlacement(hwnd, &wp);
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
            WindowInfo {
                hwnd: 1,
                title: "Firefox — GitHub".into(),
                process_name: "firefox.exe".into(),
                class_name: "MozillaWindowClass".into(),
            },
            WindowInfo {
                hwnd: 2,
                title: "Visual Studio Code".into(),
                process_name: "Code.exe".into(),
                class_name: "Chrome_WidgetWin_1".into(),
            },
            WindowInfo {
                hwnd: 3,
                title: "my-project".into(),
                process_name: "wt.exe".into(),
                class_name: "CASCADIA_HOSTING_WINDOW_CLASS".into(),
            },
            WindowInfo {
                hwnd: 4,
                title: "ssh prod-server".into(),
                process_name: "wt.exe".into(),
                class_name: "CASCADIA_HOSTING_WINDOW_CLASS".into(),
            },
            WindowInfo {
                hwnd: 5,
                title: "Notepad — readme.txt".into(),
                process_name: "notepad.exe".into(),
                class_name: "Notepad".into(),
            },
            WindowInfo {
                hwnd: 6,
                title: "Task Manager".into(),
                process_name: "Taskmgr.exe".into(),
                class_name: "TaskManagerWindow".into(),
            },
        ])
    }

    pub fn bring_to_front(info: &WindowInfo) -> Result<()> {
        bail!(
            "bring_to_front is not supported on this platform (hwnd={})",
            info.hwnd
        );
    }

    pub fn close_window(info: &WindowInfo) -> Result<()> {
        bail!(
            "close_window is not supported on this platform (hwnd={})",
            info.hwnd
        );
    }
}

// ─── Public re-exports ─────────────────────────────────────────────────────

pub use platform::{bring_to_front, close_window, enumerate_windows};
