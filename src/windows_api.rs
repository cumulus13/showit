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
    use windows::Win32::System::Threading::{
        AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowPlacement, GetWindowTextW,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetWindowPos, ShowWindow, HWND_TOP,
        SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
        SWP_SHOWWINDOW, SW_MAXIMIZE, SW_SHOWNA, SW_SHOWNOACTIVATE, WINDOWPLACEMENT,
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

    /// Bring a window to the foreground **without changing its show-state**.
    ///
    /// Public entry point called from `main.rs` right after a window is
    /// picked. On Windows this does **not** raise the window synchronously
    /// in this process. Instead it spawns a short-lived, fully detached
    /// helper (`showit.exe --__raise <hwnd>`) that sleeps briefly and then
    /// performs the raise. See `spawn_delayed_raise` for why.
    ///
    /// If spawning the helper fails for any reason, falls back to raising
    /// synchronously in this process (the old behaviour) as a best effort.
    pub fn bring_to_front(info: &WindowInfo) -> Result<()> {
        if spawn_delayed_raise(info.hwnd).is_ok() {
            return Ok(());
        }
        raise_now(HWND(info.hwnd as isize), &info.title)
    }

    /// Spawn `showit.exe --__raise <hwnd>` as a fully independent,
    /// windowless, detached process and return immediately without
    /// waiting on it.
    ///
    /// ## Why this exists
    ///
    /// Plain `cmd.exe` / PowerShell consoles are each owned by their own
    /// `conhost.exe` window (`ConsoleWindowClass`). The instant a child
    /// process (this program) exits and control returns to the prompt,
    /// conhost calls `SetForegroundWindow` on **its own window** so the
    /// prompt is ready for input again. If we raise the target window
    /// synchronously and then exit, that reclaim happens a moment later
    /// and silently undoes our raise — the target flashes to the front
    /// and the console snaps right back (reported as "output still shows
    /// itself"). Worse, because that reclaim races against our
    /// `AttachThreadInput` + `SetWindowPos` sequence while the previous
    /// foreground owner is mid-teardown, Windows' "nobody currently holds
    /// the foreground lock" fallback can hand the target *real* activation
    /// despite `SWP_NOACTIVATE` (reported as "focus stays on the raised
    /// window"). Both symptoms are the same race.
    ///
    /// Windows Terminal doesn't hit this because the shell runs over a
    /// ConPTY hosted *inside WT's own window* — there is no separate
    /// conhost window contesting the foreground, so the synchronous path
    /// already worked fine there.
    ///
    /// The fix is to not fight that race: deliberately act **after** it.
    /// The helper process sleeps ~220ms — long enough for this process to
    /// have fully exited and for conhost to have already reclaimed its own
    /// foreground unambiguously — and only then performs the exact same
    /// `NOACTIVATE` raise. By that point there's no contested/unsettled
    /// foreground state left for Windows to "help" us activate, so the
    /// raise lands purely as a Z-order/visibility change, exactly as
    /// intended.
    ///
    /// The helper is spawned with `CREATE_NO_WINDOW` and no inherited
    /// stdio so it never itself flashes a console window.
    fn spawn_delayed_raise(hwnd: usize) -> Result<()> {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let exe = std::env::current_exe()?;
        Command::new(exe)
            .arg("--__raise")
            .arg(hwnd.to_string())
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(())
    }

    /// Entry point for the hidden `--__raise <hwnd>` helper invocation
    /// (see `spawn_delayed_raise`). Sleeps briefly, performs the raise,
    /// then exits. Never returns.
    pub fn run_delayed_raise_and_exit(hwnd: usize) -> ! {
        std::thread::sleep(std::time::Duration::from_millis(220));
        let _ = raise_now(HWND(hwnd as isize), "");
        std::process::exit(0);
    }

    /// Algorithm (mirrors the Python reference implementation):
    ///
    /// 1. Snapshot `WINDOWPLACEMENT` so we know the current `showCmd`.
    /// 2. Attach our input queue to the foreground thread (and the target
    ///    thread) so the OS foreground lock cannot block us.
    /// 3. If the window is iconic (minimised) → `SW_SHOWNOACTIVATE` (4).
    ///    Otherwise → `SW_SHOWNA` (8) — make it visible without stealing focus.
    /// 4. `SetWindowPos(..., HWND_TOP, SWP_NOSIZE|SWP_NOMOVE|SWP_NOACTIVATE|...)`
    ///    raises the Z-order without moving/resizing/activating the window.
    /// 5. Detach input queues (always, even on error).
    /// 6. If the window was **not** maximised, restore the original placement so
    ///    position/size are exactly as before.  If it **was** maximised, skip
    ///    `SetWindowPlacement` — calling it on a maximised window forces it back
    ///    to restored size, which is the bug we are fixing.
    fn raise_now(hwnd: HWND, title: &str) -> Result<()> {
        unsafe {
            // ── 1. Snapshot current placement ────────────────────────────────
            let mut wp = WINDOWPLACEMENT {
                length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            if GetWindowPlacement(hwnd, &mut wp).is_err() {
                bail!("GetWindowPlacement failed for '{}'", title);
            }
            let original_show_cmd = wp.showCmd;

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

            // ── 3 & 4. Show without activating, then raise Z-order ───────────
            let show_result = (|| -> Result<()> {
                if IsIconic(hwnd).as_bool() {
                    // Minimised → show without activating
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                } else {
                    // Normal or Maximised → ensure visible without focus
                    ShowWindow(hwnd, SW_SHOWNA);
                }

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

            // ── 6. Restore placement only when NOT maximised ──────────────────
            // Calling SetWindowPlacement on a maximised window forces it to
            // restored size — exactly the bug reported.  Skip it in that case.
            if original_show_cmd != SW_MAXIMIZE.0 as u32 {
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

    /// Non-Windows stub for the hidden `--__raise` helper entry point.
    /// This codepath only ever runs on Windows in practice; kept here so
    /// the crate still compiles on Linux/macOS CI.
    pub fn run_delayed_raise_and_exit(hwnd: usize) -> ! {
        eprintln!(
            "raise is not supported on this platform (hwnd={})",
            hwnd
        );
        std::process::exit(1);
    }
}

// ─── Public re-exports ─────────────────────────────────────────────────────

pub use platform::{
    bring_to_front, close_window, enumerate_windows, run_delayed_raise_and_exit,
};
