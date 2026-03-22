use pulpkit_render::Color;

use crate::theme::Theme;

#[derive(Debug, Clone, Default)]
pub struct StyleProps {
    // Colors
    pub bg_color: Option<Color>,
    pub text_color: Option<Color>,
    pub border_color: Option<Color>,

    // Spacing
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub gap: f32,

    // Sizing
    pub width: Option<SizeValue>,
    pub height: Option<SizeValue>,
    pub min_width: Option<SizeValue>,
    pub max_width: Option<SizeValue>,

    // Rounding
    pub border_radius: f32,

    // Typography
    pub font_size: Option<f32>,
    pub font_weight: FontWeight,

    // Flex
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
    pub flex_grow: f32,

    // Effects
    pub opacity: f32,
}

#[derive(Debug, Clone)]
pub enum SizeValue {
    Px(f32),
    Fill,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FontWeight {
    #[default]
    Normal,
    Medium,
    Bold,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum AlignItems {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

impl StyleProps {
    pub fn parse(tokens: &str, theme: &Theme) -> Self {
        let mut props = Self::default();
        props.opacity = 1.0;
        for token in tokens.split_whitespace() {
            Self::apply_token(&mut props, token, theme);
        }
        props
    }

    fn apply_token(props: &mut Self, token: &str, theme: &Theme) {
        // 1. Typography sizes (exact matches)
        if let Some(size_key) = token.strip_prefix("text-") {
            match size_key {
                "xs" | "sm" | "base" | "lg" | "xl" => {
                    if let Some(&size) = theme.font_sizes.get(size_key) {
                        props.font_size = Some(size);
                        return;
                    }
                }
                // 2. Text behaviors
                "nowrap" | "truncate" | "wrap" => {
                    // TODO: store text behavior when fields are added
                    return;
                }
                // 3. text-<color>
                color_name => {
                    if let Some(&color) = theme.colors.get(color_name) {
                        props.text_color = Some(color);
                        return;
                    }
                }
            }
        }

        // Handle standalone text behaviors
        if token == "truncate" || token == "wrap" {
            return;
        }

        // 4. bg-<color>
        if let Some(color_name) = token.strip_prefix("bg-") {
            if let Some(&color) = theme.colors.get(color_name) {
                props.bg_color = Some(color);
                return;
            }
        }

        // 5. Spacing
        if let Some(n_str) = token.strip_prefix("p-") {
            if let Ok(n) = n_str.parse::<f32>() {
                let val = n * theme.spacing_scale;
                props.padding_top = val;
                props.padding_right = val;
                props.padding_bottom = val;
                props.padding_left = val;
                return;
            }
        }
        if let Some(n_str) = token.strip_prefix("px-") {
            if let Ok(n) = n_str.parse::<f32>() {
                let val = n * theme.spacing_scale;
                props.padding_left = val;
                props.padding_right = val;
                return;
            }
        }
        if let Some(n_str) = token.strip_prefix("py-") {
            if let Ok(n) = n_str.parse::<f32>() {
                let val = n * theme.spacing_scale;
                props.padding_top = val;
                props.padding_bottom = val;
                return;
            }
        }
        if let Some(n_str) = token.strip_prefix("m-") {
            if let Ok(n) = n_str.parse::<f32>() {
                let val = n * theme.spacing_scale;
                props.margin_top = val;
                props.margin_right = val;
                props.margin_bottom = val;
                props.margin_left = val;
                return;
            }
        }
        if let Some(n_str) = token.strip_prefix("mx-") {
            if let Ok(n) = n_str.parse::<f32>() {
                let val = n * theme.spacing_scale;
                props.margin_left = val;
                props.margin_right = val;
                return;
            }
        }
        if let Some(n_str) = token.strip_prefix("my-") {
            if let Ok(n) = n_str.parse::<f32>() {
                let val = n * theme.spacing_scale;
                props.margin_top = val;
                props.margin_bottom = val;
                return;
            }
        }
        if let Some(n_str) = token.strip_prefix("gap-") {
            if let Ok(n) = n_str.parse::<f32>() {
                props.gap = n * theme.spacing_scale;
                return;
            }
        }

        // 6. Sizing
        if let Some(val_str) = token.strip_prefix("w-") {
            if val_str == "full" {
                props.width = Some(SizeValue::Fill);
                return;
            }
            if let Ok(n) = val_str.parse::<f32>() {
                props.width = Some(SizeValue::Px(n * theme.spacing_scale));
                return;
            }
        }
        if let Some(val_str) = token.strip_prefix("h-") {
            if val_str == "full" {
                props.height = Some(SizeValue::Fill);
                return;
            }
            if let Ok(n) = val_str.parse::<f32>() {
                props.height = Some(SizeValue::Px(n * theme.spacing_scale));
                return;
            }
        }
        if let Some(val_str) = token.strip_prefix("max-w-") {
            if let Ok(n) = val_str.parse::<f32>() {
                props.max_width = Some(SizeValue::Px(n * theme.spacing_scale));
                return;
            }
        }

        // 7. Rounding
        if token == "rounded" {
            props.border_radius = *theme.rounding.get("md").unwrap_or(&8.0);
            return;
        }
        if let Some(size_key) = token.strip_prefix("rounded-") {
            if let Some(&val) = theme.rounding.get(size_key) {
                props.border_radius = val;
                return;
            }
        }

        // 8. Typography weight
        match token {
            "font-bold" => {
                props.font_weight = FontWeight::Bold;
                return;
            }
            "font-medium" => {
                props.font_weight = FontWeight::Medium;
                return;
            }
            _ => {}
        }

        // 9. Flex
        match token {
            "items-center" => {
                props.align_items = AlignItems::Center;
                return;
            }
            "items-start" => {
                props.align_items = AlignItems::Start;
                return;
            }
            "items-end" => {
                props.align_items = AlignItems::End;
                return;
            }
            "justify-center" => {
                props.justify_content = JustifyContent::Center;
                return;
            }
            "justify-end" => {
                props.justify_content = JustifyContent::End;
                return;
            }
            "justify-between" => {
                props.justify_content = JustifyContent::SpaceBetween;
                return;
            }
            "flex-1" => {
                props.flex_grow = 1.0;
                return;
            }
            _ => {}
        }

        // 10. Effects
        if let Some(n_str) = token.strip_prefix("opacity-") {
            if let Ok(n) = n_str.parse::<f32>() {
                props.opacity = n / 100.0;
                return;
            }
        }

        // 11. Unknown tokens
        #[cfg(debug_assertions)]
        eprintln!("pulpkit-layout: unknown style token: {:?}", token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn parse_bg_color() {
        let theme = Theme::default_slate();
        let props = StyleProps::parse("bg-surface", &theme);
        assert_eq!(props.bg_color, Some(Color::from_hex("#1a1e22").unwrap()));
    }

    #[test]
    fn parse_padding() {
        let theme = Theme::default_slate();
        let props = StyleProps::parse("p-2", &theme);
        assert_eq!(props.padding_top, 8.0);
        assert_eq!(props.padding_left, 8.0);
    }

    #[test]
    fn parse_padding_x_y() {
        let theme = Theme::default_slate();
        let props = StyleProps::parse("px-3 py-1", &theme);
        assert_eq!(props.padding_left, 12.0);
        assert_eq!(props.padding_right, 12.0);
        assert_eq!(props.padding_top, 4.0);
        assert_eq!(props.padding_bottom, 4.0);
    }

    #[test]
    fn parse_multiple_tokens() {
        let theme = Theme::default_slate();
        let props = StyleProps::parse("bg-base rounded-lg p-2 text-sm text-fg gap-4", &theme);
        assert!(props.bg_color.is_some());
        assert_eq!(props.border_radius, 12.0);
        assert_eq!(props.padding_top, 8.0);
        assert_eq!(props.font_size, Some(12.0));
        assert!(props.text_color.is_some());
        assert_eq!(props.gap, 16.0);
    }

    #[test]
    fn parse_sizing() {
        let theme = Theme::default_slate();
        let props = StyleProps::parse("w-full h-9", &theme);
        assert!(matches!(props.width, Some(SizeValue::Fill)));
        assert!(matches!(props.height, Some(SizeValue::Px(v)) if (v - 36.0).abs() < 0.01));
    }

    #[test]
    fn parse_flex_alignment() {
        let theme = Theme::default_slate();
        let props = StyleProps::parse("items-center justify-end", &theme);
        assert_eq!(props.align_items, AlignItems::Center);
        assert_eq!(props.justify_content, JustifyContent::End);
    }

    #[test]
    fn parse_text_color_vs_size() {
        let theme = Theme::default_slate();
        // text-sm should be font size, text-primary should be color
        let props = StyleProps::parse("text-sm text-primary", &theme);
        assert_eq!(props.font_size, Some(12.0));
        assert_eq!(props.text_color, Some(Color::from_hex("#8cb4d8").unwrap()));
    }
}
