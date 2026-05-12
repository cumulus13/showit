// config.rs — Configuration loading and defaults for showit
// Author: Hadi Cahyadi <cumulus13@gmail.com>

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Palette of hex colors cycled through when coloring window titles.
/// Each entry is a valid HTML hex color string, e.g. "#00FFFF".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// List of hex colors to cycle through for window titles.
    #[serde(default = "default_colors")]
    pub colors: Vec<String>,

    /// Color used for the index numbers shown in the list.
    #[serde(default = "default_index_color")]
    pub index_color: String,

    /// Color used for the prompt / status messages.
    #[serde(default = "default_prompt_color")]
    pub prompt_color: String,

    /// Color used for highlighting the matched portion of a title.
    #[serde(default = "default_match_color")]
    pub match_color: String,

    /// Color used for error messages.
    #[serde(default = "default_error_color")]
    pub error_color: String,

    /// Color used for success messages.
    #[serde(default = "default_success_color")]
    pub success_color: String,

    /// Color for the document/path part (left of " - ").
    #[serde(default = "default_doc_color")]
    pub doc_color: String,

    /// Color for the app name (between " - " and user tag).
    #[serde(default = "default_app_color")]
    pub app_color: String,

    /// Color for the user/role tag at the end of the title, e.g. "(ADMIN)" or "(John)".
    #[serde(default = "default_user_color")]
    pub user_color: String,
}

fn default_doc_color() -> String {
    "#00FFFF".into()
}

fn default_app_color() -> String {
    "#FFFF00".into()
}

fn default_user_color() -> String {
    "#AA55FF".into()
}

fn default_colors() -> Vec<String> {
    vec![
        "#00FFFF".into(), // Cyan
        "#FF6B6B".into(), // Coral red
        "#FFD700".into(), // Gold
        "#7CFC00".into(), // Lawn green
        "#FF69B4".into(), // Hot pink
        "#00CED1".into(), // Dark turquoise
        "#FFA500".into(), // Orange
        "#DA70D6".into(), // Orchid
        "#ADFF2F".into(), // Green-yellow
        "#1E90FF".into(), // Dodger blue
        "#FF4500".into(), // Orange-red
        "#98FB98".into(), // Pale green
    ]
}

fn default_index_color() -> String {
    "#888888".into()
}

fn default_prompt_color() -> String {
    "#00BFFF".into()
}

fn default_match_color() -> String {
    "#FFFF00".into()
}

fn default_error_color() -> String {
    "#FF4444".into()
}

fn default_success_color() -> String {
    "#44FF88".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            colors: default_colors(),
            index_color: default_index_color(),
            prompt_color: default_prompt_color(),
            match_color: default_match_color(),
            error_color: default_error_color(),
            success_color: default_success_color(),
        doc_color: default_doc_color(),
        app_color: default_app_color(),
        user_color: default_user_color(),
        }
    }
}

impl Config {
    /// Return the path to the config file:
    ///   %APPDATA%\showit\config.toml  (Windows)
    ///   ~/.config/showit/config.toml  (Unix / fallback)
    pub fn config_path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("showit").join("config.toml")
    }

    /// Load config from disk, or return defaults if the file does not exist.
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;

        let cfg: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", path.display()))?;

        Ok(cfg)
    }

    /// Write the default config template to disk so the user can edit it.
    pub fn write_default(path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let default_cfg = Self::default();
        let content = toml::to_string_pretty(&default_cfg)
            .context("Failed to serialize default config")?;

        let header = "# showit configuration file\n\
                      # Colors must be valid hex strings: \"#RRGGBB\" or \"#RGB\"\n\
                      # Homepage: https://github.com/cumulus13/showit\n\n";

        fs::write(path, format!("{}{}", header, content))
            .with_context(|| format!("Failed to write config: {}", path.display()))?;

        Ok(())
    }

    /// Parse a hex color string into (r, g, b) components.
    /// Accepts "#RRGGBB", "#RGB", "RRGGBB", "RGB".
    pub fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
        let s = hex.trim_start_matches('#');
        match s.len() {
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some((r, g, b))
            }
            3 => {
                let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
                Some((r, g, b))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_hex() {
        assert_eq!(Config::parse_hex_color("#00FFFF"), Some((0, 255, 255)));
        assert_eq!(Config::parse_hex_color("FF6B6B"), Some((255, 107, 107)));
    }

    #[test]
    fn parse_short_hex() {
        assert_eq!(Config::parse_hex_color("#FFF"), Some((255, 255, 255)));
        assert_eq!(Config::parse_hex_color("#000"), Some((0, 0, 0)));
    }

    #[test]
    fn parse_invalid_hex() {
        assert_eq!(Config::parse_hex_color("#ZZZZZZ"), None);
        assert_eq!(Config::parse_hex_color("notacolor"), None);
    }

    #[test]
    fn default_config_has_colors() {
        let cfg = Config::default();
        assert!(!cfg.colors.is_empty());
    }
}
