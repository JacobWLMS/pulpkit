use pulpkit_render::Color;
use std::collections::HashMap;

pub struct Theme {
    pub colors: HashMap<String, Color>,
    pub spacing_scale: f32,
    pub rounding: HashMap<String, f32>,
    pub font_sizes: HashMap<String, f32>,
    pub font_family: String,
}

impl Theme {
    /// Hardcoded Slate theme for testing
    pub fn default_slate() -> Self {
        let mut colors = HashMap::new();
        colors.insert("base".into(), Color::from_hex("#121618").unwrap());
        colors.insert("surface".into(), Color::from_hex("#1a1e22").unwrap());
        colors.insert("overlay".into(), Color::from_hex("#1e2226").unwrap());
        colors.insert("card".into(), Color::from_hex("#282e32").unwrap());
        colors.insert("fg".into(), Color::from_hex("#e2e6ea").unwrap());
        colors.insert("dim".into(), Color::from_hex("#c0c8d0").unwrap());
        colors.insert("muted".into(), Color::from_hex("#8a929a").unwrap());
        colors.insert("outline".into(), Color::from_hex("#404850").unwrap());
        colors.insert("primary".into(), Color::from_hex("#8cb4d8").unwrap());
        colors.insert("secondary".into(), Color::from_hex("#b0c4d4").unwrap());
        colors.insert("accent".into(), Color::from_hex("#a8c4a8").unwrap());
        colors.insert("error".into(), Color::from_hex("#ffb4ab").unwrap());
        colors.insert("warning".into(), Color::from_hex("#f0c878").unwrap());

        let mut rounding = HashMap::new();
        rounding.insert("sm".into(), 4.0);
        rounding.insert("md".into(), 8.0);
        rounding.insert("lg".into(), 12.0);
        rounding.insert("xl".into(), 16.0);
        rounding.insert("full".into(), 9999.0);

        let mut font_sizes = HashMap::new();
        font_sizes.insert("xs".into(), 10.0);
        font_sizes.insert("sm".into(), 12.0);
        font_sizes.insert("base".into(), 14.0);
        font_sizes.insert("lg".into(), 16.0);
        font_sizes.insert("xl".into(), 20.0);

        Self {
            colors,
            spacing_scale: 4.0,
            rounding,
            font_sizes,
            font_family: "JetBrainsMono Nerd Font".into(),
        }
    }
}
