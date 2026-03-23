//! Theme — colors, fonts, loaded from theme.lua.

use std::collections::HashMap;

use pulpkit_render::Color;

/// Theme configuration loaded from a shell's theme.lua.
#[derive(Debug, Clone)]
pub struct Theme {
    pub font_family: String,
    pub font_size: f32,
    pub colors: HashMap<String, Color>,
}

impl Theme {
    /// Look up a named color (e.g., "base", "surface", "primary").
    pub fn color(&self, name: &str) -> Option<Color> {
        self.colors.get(name).copied()
    }

    /// Default dark slate theme (used when no theme.lua is provided).
    pub fn default_slate() -> Self {
        let mut colors = HashMap::new();
        colors.insert("base".into(), Color::from_hex("#1e2128").unwrap());
        colors.insert("surface".into(), Color::from_hex("#282c34").unwrap());
        colors.insert("primary".into(), Color::from_hex("#8cb4d8").unwrap());
        colors.insert("fg".into(), Color::from_hex("#c8ccd4").unwrap());
        colors.insert("muted".into(), Color::from_hex("#8a929a").unwrap());
        colors.insert("error".into(), Color::from_hex("#e06c75").unwrap());
        colors.insert("success".into(), Color::from_hex("#98c379").unwrap());
        colors.insert("warning".into(), Color::from_hex("#e5c07b").unwrap());

        Theme {
            font_family: "JetBrainsMono Nerd Font".into(),
            font_size: 14.0,
            colors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_slate_has_base_color() {
        let theme = Theme::default_slate();
        assert!(theme.color("base").is_some());
        assert!(theme.color("surface").is_some());
        assert!(theme.color("primary").is_some());
        assert!(theme.color("fg").is_some());
        assert!(theme.color("muted").is_some());
    }

    #[test]
    fn unknown_color_returns_none() {
        let theme = Theme::default_slate();
        assert!(theme.color("nonexistent").is_none());
    }
}
