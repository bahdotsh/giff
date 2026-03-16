use std::collections::HashMap;

use serde::Deserialize;

use crate::ui::theme::{Theme, ThemeConfig};

#[derive(Default, Deserialize)]
pub struct Config {
    pub theme: Option<String>,
    #[serde(default)]
    pub themes: HashMap<String, ThemeConfig>,
}

pub fn load_config() -> Config {
    let config_path = match dirs::config_dir() {
        Some(dir) => dir.join("giff").join("config.toml"),
        None => return Config::default(),
    };

    let contents = match std::fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(_) => return Config::default(),
    };

    match toml::from_str(&contents) {
        Ok(config) => config,
        Err(e) => {
            eprintln!(
                "Warning: failed to parse {}: {}",
                config_path.display(),
                e
            );
            Config::default()
        }
    }
}

pub fn resolve_theme(config: &Config, cli_theme: Option<&str>) -> Theme {
    let theme_name = cli_theme
        .or(config.theme.as_deref())
        .unwrap_or("dark");

    if let Some(theme) = Theme::by_name(theme_name) {
        return theme;
    }

    if let Some(theme_config) = config.themes.get(theme_name) {
        return theme_config.to_theme();
    }

    eprintln!(
        "Warning: unknown theme '{}', falling back to dark",
        theme_name
    );
    Theme::dark()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn empty_config() -> Config {
        Config::default()
    }

    fn config_with_theme(name: &str) -> Config {
        Config {
            theme: Some(name.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn resolve_defaults_to_dark() {
        let t = resolve_theme(&empty_config(), None);
        assert!(t.is_dark);
    }

    #[test]
    fn resolve_cli_overrides_config() {
        let config = config_with_theme("dark");
        let t = resolve_theme(&config, Some("light"));
        assert!(!t.is_dark);
    }

    #[test]
    fn resolve_config_file_theme() {
        let config = config_with_theme("light");
        let t = resolve_theme(&config, None);
        assert!(!t.is_dark);
    }

    #[test]
    fn resolve_unknown_falls_back_to_dark() {
        let t = resolve_theme(&empty_config(), Some("nonexistent"));
        assert!(t.is_dark);
    }

    #[test]
    fn resolve_custom_theme_from_config() {
        let mut config = empty_config();
        config.themes.insert(
            "custom".to_string(),
            ThemeConfig {
                base: Some("light".to_string()),
                accent: Some("#FF0000".to_string()),
                ..Default::default()
            },
        );
        let t = resolve_theme(&config, Some("custom"));
        assert!(!t.is_dark); // based on light
        assert_eq!(t.accent, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn resolve_priority_cli_over_config_over_default() {
        // CLI wins over config
        let config = config_with_theme("light");
        let t = resolve_theme(&config, Some("dark"));
        assert!(t.is_dark);

        // Config wins over default
        let config = config_with_theme("light");
        let t = resolve_theme(&config, None);
        assert!(!t.is_dark);

        // Default is dark
        let t = resolve_theme(&empty_config(), None);
        assert!(t.is_dark);
    }
}
