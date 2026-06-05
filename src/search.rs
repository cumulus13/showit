// search.rs — Window title searching and filtering for showit
// Author: Hadi Cahyadi <cumulus13@gmail.com>

use crate::windows_api::WindowInfo;
use anyhow::Result;
use regex::{Regex, RegexBuilder};

/// Strategy used to match a query against window titles.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchMode {
    /// Plain substring match (case-insensitive).
    Substring,
    /// Shell-style wildcard: `*` matches any sequence of characters.
    Wildcard,
    /// Full regular-expression match (case-insensitive).
    Regex,
}

/// Determine the match mode from the raw query string.
pub fn detect_mode(query: &str, force_regex: bool) -> MatchMode {
    if force_regex {
        return MatchMode::Regex;
    }
    if query.contains('*') || query.contains('?') {
        MatchMode::Wildcard
    } else {
        MatchMode::Substring
    }
}

/// Convert a shell wildcard pattern to an equivalent regex string.
fn wildcard_to_regex(pattern: &str) -> String {
    let mut re = String::from("(?i)^");
    for ch in pattern.chars() {
        match ch {
            '*' => re.push_str(".*"),
            '?' => re.push('.'),
            c => re.push_str(&regex::escape(&c.to_string())),
        }
    }
    re.push('$');
    re
}

/// Filter `windows` by `query` using the given mode.
///
/// The search corpus for each window is:
///   - `title`        — the raw window title (always searched)
///   - `type_label()` — canonical app name derived from class/process
///     e.g. "Windows Terminal", "VS Code", "PowerShell"
///
/// This means `showit terminal` will find a `wt.exe` window whose tab
/// title is "my-project", because its type_label is "Windows Terminal".
fn window_matches(w: &WindowInfo, query: &str, re: &Option<Regex>) -> bool {
    let type_label = w.type_label();
    let candidates = [w.title.as_str(), type_label.as_str()];
    if let Some(ref r) = re {
        candidates.iter().any(|s| r.is_match(s))
    } else {
        let q = query.to_lowercase();
        candidates.iter().any(|s| s.to_lowercase().contains(&q))
    }
}

pub fn filter_windows<'a>(
    windows: &'a [WindowInfo],
    query: &str,
    mode: &MatchMode,
) -> Result<Vec<&'a WindowInfo>> {
    let re: Option<Regex> = match mode {
        MatchMode::Substring => None, // plain contains() loop
        MatchMode::Wildcard => {
            let pat = wildcard_to_regex(query);
            Some(Regex::new(&pat)?)
        }
        MatchMode::Regex => Some(RegexBuilder::new(query).case_insensitive(true).build()?),
    };

    let matches = windows
        .iter()
        .filter(|w| window_matches(w, query, &re))
        .collect();

    Ok(matches)
}

/// Return `true` if `query` looks like a valid regex (can compile).
#[allow(dead_code)]
pub fn is_valid_regex(query: &str) -> bool {
    Regex::new(query).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::windows_api::WindowInfo;

    fn make_windows() -> Vec<WindowInfo> {
        vec![
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
            // wt tab with a custom title — title does NOT contain "terminal"
            WindowInfo {
                hwnd: 3,
                title: "my-project".into(),
                process_name: "wt.exe".into(),
                class_name: "CASCADIA_HOSTING_WINDOW_CLASS".into(),
            },
            WindowInfo {
                hwnd: 4,
                title: "ssh prod".into(),
                process_name: "wt.exe".into(),
                class_name: "CASCADIA_HOSTING_WINDOW_CLASS".into(),
            },
            WindowInfo {
                hwnd: 5,
                title: "Notepad — readme.txt".into(),
                process_name: "notepad.exe".into(),
                class_name: "Notepad".into(),
            },
        ]
    }

    #[test]
    fn substring_match_title() {
        let ws = make_windows();
        let r = filter_windows(&ws, "readme", &MatchMode::Substring).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].title, "Notepad — readme.txt");
    }

    /// "terminal" should hit both wt windows via their type_label,
    /// even though neither title contains the word.
    #[test]
    fn substring_match_type_label() {
        let ws = make_windows();
        let r = filter_windows(&ws, "terminal", &MatchMode::Substring).unwrap();
        assert_eq!(r.len(), 2, "both wt windows should match 'terminal'");
        assert!(r.iter().all(|w| w.process_name == "wt.exe"));
    }

    /// "wt" process-name stem also resolves to "Windows Terminal".
    #[test]
    fn substring_match_wt_label() {
        let ws = make_windows();
        let r = filter_windows(&ws, "windows terminal", &MatchMode::Substring).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn wildcard_star() {
        let ws = make_windows();
        let r = filter_windows(&ws, "*studio*", &MatchMode::Wildcard).unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn regex_match() {
        let ws = make_windows();
        let r = filter_windows(&ws, r"fire|note", &MatchMode::Regex).unwrap();
        assert_eq!(r.len(), 2);
    }

    /// Regex on type_label: "terminal" regex should catch wt windows.
    #[test]
    fn regex_match_type_label() {
        let ws = make_windows();
        let r = filter_windows(&ws, "terminal", &MatchMode::Regex).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn detect_wildcard() {
        assert_eq!(detect_mode("*foo*", false), MatchMode::Wildcard);
    }
    #[test]
    fn detect_substring() {
        assert_eq!(detect_mode("firefox", false), MatchMode::Substring);
    }
    #[test]
    fn detect_forced_regex() {
        assert_eq!(detect_mode("firefox", true), MatchMode::Regex);
    }
}
