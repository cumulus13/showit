// display.rs — colored output helpers for showit
// Author: Hadi Cahyadi <cumulus13@gmail.com>

use crate::config::Config;
use colored::*;

/// Apply a hex color string "#RRGGBB" to text. Falls back to plain if invalid.
pub fn colorize(text: &str, hex: &str) -> String {
    if let Some((r, g, b)) = Config::parse_hex_color(hex) {
        text.truecolor(r, g, b).to_string()
    } else {
        text.to_string()
    }
}

/// Colorize `text` with `base`, except balanced "(...)" groups (nested
/// parens included) which get `group`. An unclosed "(" is left in `base`.
pub fn colorize_groups(text: &str, base: &str, group: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut plain_start = 0usize;
    let mut group_start = 0usize;
    let mut depth = 0usize;

    for (i, c) in text.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    if i > plain_start {
                        out.push_str(&colorize(&text[plain_start..i], base));
                    }
                    group_start = i;
                }
                depth += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    out.push_str(&colorize(&text[group_start..=i], group));
                    plain_start = i + 1;
                }
            }
            _ => {}
        }
    }

    if plain_start < text.len() {
        out.push_str(&colorize(&text[plain_start..], base));
    }
    out
}

pub fn print_error(msg: &str, cfg: &Config) {
    eprintln!("{}", colorize(msg, &cfg.error_color));
}

pub fn print_success(msg: &str, cfg: &Config) {
    println!("{}", colorize(msg, &cfg.success_color));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorize_valid_hex() {
        let s = colorize("hello", "#00FFFF");
        assert!(s.contains("hello"));
    }

    #[test]
    fn colorize_invalid_hex_still_returns_text() {
        let s = colorize("hello", "notacolor");
        assert!(s.contains("hello"));
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut in_esc = false;
        for c in s.chars() {
            if in_esc {
                if c == 'm' {
                    in_esc = false;
                }
            } else if c == '\x1b' {
                in_esc = true;
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn groups_preserve_text() {
        for t in [
            r"C:\a\b.py (django-celery)",
            "so this can be run: pipinfo th • (liputan6)",
            "x (a (b) c) y (d)",
            "unclosed (abc",
            "stray ) paren",
            "no parens",
            "",
        ] {
            assert_eq!(strip_ansi(&colorize_groups(t, "#00FFFF", "#FFAAFF")), t);
        }
    }

    #[test]
    fn groups_use_group_color() {
        colored::control::set_override(true);
        let s = colorize_groups("file.py (proj)", "#00FFFF", "#FFAAFF");
        assert!(s.contains("38;2;255;170;255"));
        assert!(s.contains("38;2;0;255;255"));
    }
}
