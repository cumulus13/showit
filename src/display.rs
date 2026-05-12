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
}
