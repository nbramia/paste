//! TOML configuration loading and validation.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration loaded from ~/.config/paste/config.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub hotkeys: HotkeyConfig,
    pub clipboard: ClipboardConfig,
    pub storage: StorageConfig,
    pub ui: UiConfig,
    pub expander: ExpanderConfig,
    pub injection: InjectionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub toggle_overlay: String,
    pub paste_stack_mode: String,
    pub quick_copy_to_pinboard: String,
    pub toggle_expander: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipboardConfig {
    pub monitor_primary: bool,
    pub monitor_clipboard: bool,
    pub excluded_apps: Vec<String>,
    pub max_content_size_mb: u32,
    pub merge_growing: bool,
    pub debounce_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub max_history_days: u32,
    pub max_history_count: u32,
    pub max_image_size_mb: u32,
    pub max_total_storage_mb: u32,
    pub db_path: String,
    pub image_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    pub filmstrip_height: u32,
    pub cards_visible: u32,
    pub animation_speed: f64,
    pub blur_background: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExpanderConfig {
    pub enabled: bool,
    pub trigger: String,
    pub typing_speed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InjectionConfig {
    pub method: String,
}

// ---------------------------------------------------------------------------
// Default implementations
// ---------------------------------------------------------------------------

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            toggle_overlay: "Super+Alt+V".into(),
            paste_stack_mode: "Super+Shift+V".into(),
            quick_copy_to_pinboard: "Super+Shift+C".into(),
            toggle_expander: "Ctrl+Alt+Space".into(),
        }
    }
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            monitor_primary: true,
            monitor_clipboard: true,
            excluded_apps: vec![
                "1password".into(),
                "keepassxc".into(),
                "bitwarden".into(),
                "lastpass".into(),
            ],
            max_content_size_mb: 10,
            merge_growing: true,
            debounce_ms: 500,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_history_days: 90,
            max_history_count: 10000,
            max_image_size_mb: 10,
            max_total_storage_mb: 500,
            db_path: "~/.local/share/paste/paste.db".into(),
            image_dir: "~/.local/share/paste/images".into(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            filmstrip_height: 300,
            cards_visible: 6,
            animation_speed: 1.0,
            blur_background: true,
        }
    }
}

impl Default for ExpanderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger: "word_boundary".into(),
            typing_speed: 0,
        }
    }
}

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            method: "auto".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("Validation error: {0}")]
    Validation(String),
}

// ---------------------------------------------------------------------------
// Loading, parsing, and validation
// ---------------------------------------------------------------------------

impl AppConfig {
    /// Load config from ~/.config/paste/config.toml.
    /// Creates the default config file if it doesn't exist.
    /// Missing fields in the file fall back to defaults (via `#[serde(default)]`).
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            // Create parent directory and write default config
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let default = Self::default();
            let toml_str = toml::to_string_pretty(&default)?;
            std::fs::write(&config_path, toml_str)?;
            return Ok(default);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load from a specific path (for testing).
    pub fn load_from(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Parse config from a TOML string (for testing).
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(toml_str)?;
        config.validate()?;
        Ok(config)
    }

    /// Get the default config file path.
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .expect("could not determine config directory")
            .join("paste")
            .join("config.toml")
    }

    /// Resolve the db_path, expanding ~ to home directory.
    pub fn resolved_db_path(&self) -> PathBuf {
        expand_tilde(&self.storage.db_path)
    }

    /// Resolve the image_dir, expanding ~ to home directory.
    pub fn resolved_image_dir(&self) -> PathBuf {
        expand_tilde(&self.storage.image_dir)
    }

    /// Validate configuration values.
    fn validate(&self) -> Result<(), ConfigError> {
        // UI validation
        if self.ui.animation_speed < 0.0 {
            return Err(ConfigError::Validation(
                "ui.animation_speed must be >= 0".into(),
            ));
        }
        if self.ui.filmstrip_height == 0 {
            return Err(ConfigError::Validation(
                "ui.filmstrip_height must be > 0".into(),
            ));
        }
        if self.ui.cards_visible == 0 {
            return Err(ConfigError::Validation(
                "ui.cards_visible must be > 0".into(),
            ));
        }

        // Theme validation
        match self.ui.theme.as_str() {
            "system" | "light" | "dark" => {}
            _ => {
                return Err(ConfigError::Validation(format!(
                    "ui.theme must be 'system', 'light', or 'dark', got '{}'",
                    self.ui.theme
                )))
            }
        }

        // Expander trigger validation
        match self.expander.trigger.as_str() {
            "word_boundary" | "immediate" => {}
            _ => {
                return Err(ConfigError::Validation(format!(
                    "expander.trigger must be 'word_boundary' or 'immediate', got '{}'",
                    self.expander.trigger
                )))
            }
        }

        // Injection method validation
        match self.injection.method.as_str() {
            "auto" | "xdotool" | "ydotool" | "wtype" | "clipboard" => {}
            _ => {
                return Err(ConfigError::Validation(format!(
                    "injection.method must be 'auto', 'xdotool', 'ydotool', 'wtype', or 'clipboard', got '{}'",
                    self.injection.method
                )))
            }
        }

        Ok(())
    }
}

/// Expand ~ to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .expect("could not determine home directory")
            .join(rest)
    } else {
        PathBuf::from(path)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.hotkeys.toggle_overlay, "Super+V");
        assert_eq!(config.hotkeys.paste_stack_mode, "Super+Shift+V");
        assert_eq!(config.clipboard.monitor_primary, true);
        assert_eq!(config.clipboard.monitor_clipboard, true);
        assert_eq!(config.clipboard.excluded_apps.len(), 4);
        assert_eq!(config.storage.max_history_days, 90);
        assert_eq!(config.storage.max_history_count, 10000);
        assert_eq!(config.ui.theme, "system");
        assert_eq!(config.ui.filmstrip_height, 300);
        assert_eq!(config.ui.cards_visible, 6);
        assert!((config.ui.animation_speed - 1.0).abs() < f64::EPSILON);
        assert_eq!(config.ui.blur_background, true);
        assert_eq!(config.expander.enabled, true);
        assert_eq!(config.expander.trigger, "word_boundary");
        assert_eq!(config.injection.method, "auto");
    }

    #[test]
    fn test_from_empty_toml() {
        // Empty TOML should give all defaults
        let config = AppConfig::from_toml("").unwrap();
        assert_eq!(config.hotkeys.toggle_overlay, "Super+V");
        assert_eq!(config.storage.max_history_days, 90);
    }

    #[test]
    fn test_partial_toml() {
        // Only override some fields; rest should be defaults
        let toml = r#"
[ui]
theme = "dark"
filmstrip_height = 400

[expander]
enabled = false
"#;
        let config = AppConfig::from_toml(toml).unwrap();
        assert_eq!(config.ui.theme, "dark");
        assert_eq!(config.ui.filmstrip_height, 400);
        assert_eq!(config.ui.cards_visible, 6); // default
        assert_eq!(config.expander.enabled, false);
        assert_eq!(config.hotkeys.toggle_overlay, "Super+V"); // default
    }

    #[test]
    fn test_invalid_theme() {
        let toml = r#"
[ui]
theme = "neon"
"#;
        let result = AppConfig::from_toml(toml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("theme"));
    }

    #[test]
    fn test_invalid_trigger() {
        let toml = r#"
[expander]
trigger = "auto"
"#;
        let result = AppConfig::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_injection_method() {
        let toml = r#"
[injection]
method = "magic"
"#;
        let result = AppConfig::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_animation_speed() {
        let toml = r#"
[ui]
animation_speed = -1.0
"#;
        let result = AppConfig::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_filmstrip_height() {
        let toml = r#"
[ui]
filmstrip_height = 0
"#;
        let result = AppConfig::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed = AppConfig::from_toml(&toml_str).unwrap();
        assert_eq!(config.hotkeys.toggle_overlay, parsed.hotkeys.toggle_overlay);
        assert_eq!(
            config.storage.max_history_days,
            parsed.storage.max_history_days
        );
        assert_eq!(config.ui.theme, parsed.ui.theme);
    }

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/foo/bar");
        assert!(!expanded.to_string_lossy().contains('~'));
        assert!(expanded.to_string_lossy().ends_with("foo/bar"));

        let absolute = expand_tilde("/absolute/path");
        assert_eq!(absolute, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_resolved_paths() {
        let config = AppConfig::default();
        let db_path = config.resolved_db_path();
        assert!(db_path.to_string_lossy().contains("paste"));
        assert!(!db_path.to_string_lossy().contains('~'));

        let img_dir = config.resolved_image_dir();
        assert!(img_dir.to_string_lossy().contains("images"));
    }

    #[test]
    fn test_load_from_file() {
        let dir = std::env::temp_dir().join("paste_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        let toml = r#"
[hotkeys]
toggle_overlay = "Ctrl+V"

[storage]
max_history_days = 30
"#;
        std::fs::write(&path, toml).unwrap();

        let config = AppConfig::load_from(&path).unwrap();
        assert_eq!(config.hotkeys.toggle_overlay, "Ctrl+V");
        assert_eq!(config.storage.max_history_days, 30);
        // Defaults for unspecified
        assert_eq!(config.ui.theme, "system");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_config_is_clone_debug_serialize_deserialize() {
        let config = AppConfig::default();
        let cloned = config.clone();
        let _ = format!("{:?}", cloned);
        let serialized = toml::to_string(&config).unwrap();
        let _: AppConfig = toml::from_str(&serialized).unwrap();
    }

    #[test]
    fn test_custom_excluded_apps() {
        let toml = r#"
[clipboard]
excluded_apps = ["myapp", "otherapp"]
"#;
        let config = AppConfig::from_toml(toml).unwrap();
        assert_eq!(config.clipboard.excluded_apps, vec!["myapp", "otherapp"]);
    }

    #[test]
    fn test_full_config() {
        let toml = r#"
[hotkeys]
toggle_overlay = "Ctrl+Shift+V"
paste_stack_mode = "Ctrl+Shift+C"
quick_copy_to_pinboard = "Ctrl+Shift+P"
toggle_expander = "Ctrl+Shift+E"

[clipboard]
monitor_primary = false
monitor_clipboard = true
excluded_apps = []
max_content_size_mb = 20

[storage]
max_history_days = 30
max_history_count = 5000
max_image_size_mb = 5
max_total_storage_mb = 200
db_path = "/tmp/paste.db"
image_dir = "/tmp/paste/images"

[ui]
theme = "dark"
filmstrip_height = 250
cards_visible = 8
animation_speed = 0.5
blur_background = false

[expander]
enabled = false
trigger = "immediate"
typing_speed = 10

[injection]
method = "xdotool"
"#;
        let config = AppConfig::from_toml(toml).unwrap();
        assert_eq!(config.hotkeys.toggle_overlay, "Ctrl+Shift+V");
        assert_eq!(config.clipboard.monitor_primary, false);
        assert_eq!(config.clipboard.max_content_size_mb, 20);
        assert_eq!(config.storage.max_history_days, 30);
        assert_eq!(config.storage.db_path, "/tmp/paste.db");
        assert_eq!(config.ui.theme, "dark");
        assert_eq!(config.ui.filmstrip_height, 250);
        assert!((config.ui.animation_speed - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.expander.enabled, false);
        assert_eq!(config.expander.trigger, "immediate");
        assert_eq!(config.injection.method, "xdotool");
    }
}
