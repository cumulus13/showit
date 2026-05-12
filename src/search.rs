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
/// Returns references into the original slice so no cloning is needed.
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
        MatchMode::Regex => Some(
            RegexBuilder::new(query)
                .case_insensitive(true)
                .build()?,
        ),
    };

    let matches = windows
        .iter()
        .filter(|w| {
            if let Some(ref r) = re {
                r.is_match(&w.title)
            } else {
                w.title.to_lowercase().contains(&query.to_lowercase())
            }
        })
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
            WindowInfo { hwnd: 1, title: "Firefox — GitHub".into() },
            WindowInfo { hwnd: 2, title: "Visual Studio Code".into() },
            WindowInfo { hwnd: 3, title: "Windows Terminal".into() },
            WindowInfo { hwnd: 4, title: "Notepad — readme.txt".into() },
        ]
    }

    #[test]
    fn substring_match() {
        let ws = make_windows();
        let results = filter_windows(&ws, "terminal", &MatchMode::Substring).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Windows Terminal");
    }

    #[test]
    fn wildcard_star() {
        let ws = make_windows();
        let results = filter_windows(&ws, "*studio*", &MatchMode::Wildcard).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn regex_match() {
        let ws = make_windows();
        let results = filter_windows(&ws, r"fire|note", &MatchMode::Regex).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn detect_wildcard() {
        assert_eq!(detect_mode("*foo*", false), MatchMode::Wildcard);
        assert_eq!(detect_mode("foo?", false), MatchMode::Wildcard);
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
