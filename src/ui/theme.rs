use ratatui::style::Color;
use serde::Deserialize;

#[derive(Clone, PartialEq)]
pub struct Theme {
    pub is_dark: bool,
    // General UI
    pub accent: Color,
    pub border_focused: Color,
    pub border_dim: Color,
    pub fg_dim: Color,
    pub fg_normal: Color,
    pub fg_bright: Color,
    pub fg_added: Color,
    pub fg_removed: Color,
    pub fg_key: Color,
    pub bg_header: Color,
    pub bg_selection: Color,
    pub bg_accepted: Color,
    pub bg_rejected: Color,
    pub bg_modal_dim: Color,
    pub bg_modal: Color,
    pub border_modal: Color,
    pub bg_key_badge: Color,
    pub fg_separator: Color,
    pub fg_badge: Color,
    // Syntax / diff
    pub bg_added: Color,
    pub bg_removed: Color,
    pub fg_line_num: Color,
    pub fg_added_marker: Color,
    pub fg_removed_marker: Color,
    pub syntax_theme: String,
    // Root background
    pub bg_default: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Theme {
            is_dark: true,
            accent: Color::Rgb(137, 180, 250),
            border_focused: Color::Rgb(137, 180, 250),
            border_dim: Color::Rgb(55, 56, 68),
            fg_dim: Color::Rgb(108, 112, 134),
            fg_normal: Color::Rgb(186, 194, 222),
            fg_bright: Color::Rgb(205, 214, 244),
            fg_added: Color::Rgb(166, 218, 149),
            fg_removed: Color::Rgb(243, 139, 168),
            fg_key: Color::Rgb(249, 226, 175),
            bg_header: Color::Rgb(24, 24, 37),
            bg_selection: Color::Rgb(40, 42, 56),
            bg_accepted: Color::Rgb(20, 38, 24),
            bg_rejected: Color::Rgb(42, 22, 26),
            bg_modal_dim: Color::Rgb(14, 14, 22),
            bg_modal: Color::Rgb(30, 30, 46),
            border_modal: Color::Rgb(88, 91, 112),
            bg_key_badge: Color::Rgb(42, 43, 58),
            fg_separator: Color::Rgb(45, 46, 58),
            fg_badge: Color::Rgb(24, 24, 37),
            bg_added: Color::Rgb(20, 38, 24),
            bg_removed: Color::Rgb(42, 22, 26),
            fg_line_num: Color::Rgb(88, 91, 112),
            fg_added_marker: Color::Rgb(166, 218, 149),
            fg_removed_marker: Color::Rgb(243, 139, 168),
            syntax_theme: "base16-ocean.dark".to_string(),
            bg_default: Color::Reset,
        }
    }

    pub fn light() -> Self {
        Theme {
            is_dark: false,
            accent: Color::Rgb(56, 118, 208),
            border_focused: Color::Rgb(56, 118, 208),
            border_dim: Color::Rgb(210, 214, 222),
            fg_dim: Color::Rgb(128, 136, 154),
            fg_normal: Color::Rgb(46, 52, 64),
            fg_bright: Color::Rgb(26, 32, 42),
            fg_added: Color::Rgb(32, 146, 66),
            fg_removed: Color::Rgb(210, 56, 64),
            fg_key: Color::Rgb(172, 110, 8),
            bg_header: Color::Rgb(247, 248, 250),
            bg_selection: Color::Rgb(235, 241, 252),
            bg_accepted: Color::Rgb(220, 245, 225),
            bg_rejected: Color::Rgb(255, 225, 223),
            bg_modal_dim: Color::Rgb(240, 241, 244),
            bg_modal: Color::Rgb(255, 255, 255),
            border_modal: Color::Rgb(190, 196, 210),
            bg_key_badge: Color::Rgb(238, 240, 246),
            fg_separator: Color::Rgb(228, 230, 236),
            fg_badge: Color::Rgb(255, 255, 255),
            bg_added: Color::Rgb(220, 245, 225),
            bg_removed: Color::Rgb(255, 225, 223),
            fg_line_num: Color::Rgb(158, 166, 182),
            fg_added_marker: Color::Rgb(32, 146, 66),
            fg_removed_marker: Color::Rgb(210, 56, 64),
            syntax_theme: "base16-ocean.light".to_string(),
            bg_default: Color::Rgb(252, 252, 254),
        }
    }

    pub fn by_name(name: &str) -> Option<Theme> {
        match name {
            "dark" => Some(Theme::dark()),
            "light" => Some(Theme::light()),
            _ => None,
        }
    }
}

pub fn parse_color(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

#[derive(Default, Deserialize)]
pub struct ThemeConfig {
    pub base: Option<String>,
    pub accent: Option<String>,
    pub border_focused: Option<String>,
    pub border_dim: Option<String>,
    pub fg_dim: Option<String>,
    pub fg_normal: Option<String>,
    pub fg_bright: Option<String>,
    pub fg_added: Option<String>,
    pub fg_removed: Option<String>,
    pub fg_key: Option<String>,
    pub bg_header: Option<String>,
    pub bg_selection: Option<String>,
    pub bg_accepted: Option<String>,
    pub bg_rejected: Option<String>,
    pub bg_modal_dim: Option<String>,
    pub bg_modal: Option<String>,
    pub border_modal: Option<String>,
    pub bg_key_badge: Option<String>,
    pub fg_separator: Option<String>,
    pub fg_badge: Option<String>,
    pub bg_added: Option<String>,
    pub bg_removed: Option<String>,
    pub fg_line_num: Option<String>,
    pub fg_added_marker: Option<String>,
    pub fg_removed_marker: Option<String>,
    pub syntax_theme: Option<String>,
    pub bg_default: Option<String>,
}

macro_rules! override_color {
    ($theme:expr, $config:expr, $( $field:ident ),+ $(,)?) => {
        $(
            if let Some(ref s) = $config.$field {
                if let Some(c) = parse_color(s) {
                    $theme.$field = c;
                } else {
                    eprintln!(
                        "Warning: invalid color '{}' for '{}', using default",
                        s,
                        stringify!($field)
                    );
                }
            }
        )+
    };
}

impl ThemeConfig {
    pub fn to_theme(&self) -> Theme {
        let mut theme = match self.base.as_deref() {
            Some("light") => Theme::light(),
            _ => Theme::dark(),
        };

        override_color!(
            theme,
            self,
            accent,
            border_focused,
            border_dim,
            fg_dim,
            fg_normal,
            fg_bright,
            fg_added,
            fg_removed,
            fg_key,
            bg_header,
            bg_selection,
            bg_accepted,
            bg_rejected,
            bg_modal_dim,
            bg_modal,
            border_modal,
            bg_key_badge,
            fg_separator,
            fg_badge,
            bg_added,
            bg_removed,
            fg_line_num,
            fg_added_marker,
            fg_removed_marker,
            bg_default,
        );

        if let Some(ref s) = self.syntax_theme {
            theme.syntax_theme = s.clone();
        }

        theme
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_color ─────────────────────────────────────────────────────

    #[test]
    fn parse_color_valid() {
        assert_eq!(parse_color("#FF0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_color("#0000FF"), Some(Color::Rgb(0, 0, 255)));
        assert_eq!(parse_color("#1a2B3c"), Some(Color::Rgb(0x1a, 0x2b, 0x3c)));
    }

    #[test]
    fn parse_color_missing_hash() {
        assert_eq!(parse_color("FF0000"), None);
    }

    #[test]
    fn parse_color_wrong_length() {
        assert_eq!(parse_color("#FFF"), None);
        assert_eq!(parse_color("#FF00FF00"), None);
        assert_eq!(parse_color("#"), None);
        assert_eq!(parse_color(""), None);
    }

    #[test]
    fn parse_color_invalid_hex() {
        assert_eq!(parse_color("#GGGGGG"), None);
        assert_eq!(parse_color("#ZZZZZZ"), None);
    }

    // ── Theme constructors ──────────────────────────────────────────────

    #[test]
    fn dark_theme_is_dark() {
        let t = Theme::dark();
        assert!(t.is_dark);
        assert_eq!(t.syntax_theme, "base16-ocean.dark");
    }

    #[test]
    fn light_theme_is_light() {
        let t = Theme::light();
        assert!(!t.is_dark);
        assert_eq!(t.syntax_theme, "base16-ocean.light");
    }

    #[test]
    fn by_name_known() {
        assert!(Theme::by_name("dark").unwrap().is_dark);
        assert!(!Theme::by_name("light").unwrap().is_dark);
    }

    #[test]
    fn by_name_unknown() {
        assert!(Theme::by_name("neon").is_none());
    }

    // ── Theme structural invariants ─────────────────────────────────────

    #[test]
    fn dark_theme_structural_invariants() {
        let t = Theme::dark();
        assert!(t.is_dark);
        assert_ne!(t.bg_added, t.bg_removed);
        assert_ne!(t.fg_added, t.fg_removed);
        assert_ne!(t.fg_added_marker, t.fg_removed_marker);
        assert_ne!(t.bg_accepted, t.bg_rejected);
        assert_eq!(t.syntax_theme, "base16-ocean.dark");
        assert_eq!(t.bg_default, Color::Reset);
    }

    #[test]
    fn light_theme_structural_invariants() {
        let t = Theme::light();
        assert!(!t.is_dark);
        assert_ne!(t.bg_added, t.bg_removed);
        assert_ne!(t.fg_added, t.fg_removed);
        assert_ne!(t.fg_added_marker, t.fg_removed_marker);
        assert_ne!(t.bg_accepted, t.bg_rejected);
        assert_eq!(t.syntax_theme, "base16-ocean.light");
        assert_ne!(t.bg_default, Color::Reset);
    }

    // ── ThemeConfig::to_theme ───────────────────────────────────────────

    #[test]
    fn to_theme_defaults_to_dark() {
        let cfg = ThemeConfig::default();
        let t = cfg.to_theme();
        assert!(t.is_dark);
        assert_eq!(t.accent, Theme::dark().accent);
    }

    #[test]
    fn to_theme_with_light_base() {
        let cfg = ThemeConfig {
            base: Some("light".to_string()),
            ..Default::default()
        };
        let t = cfg.to_theme();
        assert!(!t.is_dark);
        assert_eq!(t.accent, Theme::light().accent);
    }

    #[test]
    fn to_theme_unknown_base_falls_back_to_dark() {
        let cfg = ThemeConfig {
            base: Some("neon".to_string()),
            ..Default::default()
        };
        let t = cfg.to_theme();
        assert!(t.is_dark);
    }

    #[test]
    fn to_theme_overrides_single_field() {
        let cfg = ThemeConfig {
            accent: Some("#FF0000".to_string()),
            ..Default::default()
        };
        let t = cfg.to_theme();
        assert_eq!(t.accent, Color::Rgb(255, 0, 0));
        // Other fields remain dark defaults
        assert_eq!(t.fg_dim, Theme::dark().fg_dim);
    }

    #[test]
    fn to_theme_overrides_syntax_theme() {
        let cfg = ThemeConfig {
            syntax_theme: Some("Solarized (Dark)".to_string()),
            ..Default::default()
        };
        let t = cfg.to_theme();
        assert_eq!(t.syntax_theme, "Solarized (Dark)");
    }

    #[test]
    fn to_theme_ignores_invalid_color() {
        let cfg = ThemeConfig {
            accent: Some("#GGGGGG".to_string()),
            ..Default::default()
        };
        let t = cfg.to_theme();
        // Invalid hex is ignored (warning printed), keeps dark default
        assert_eq!(t.accent, Theme::dark().accent);
    }
}
