//! Style properties and Tailwind-style token parsing.

use pulpkit_render::Color;

use crate::theme::Theme;

/// Style properties for an element. All fields have sensible defaults.
#[derive(Debug, Clone, PartialEq)]
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

/// Empty style constant for Spacer and similar.
impl StyleProps {
    pub const EMPTY: StyleProps = StyleProps {
        bg_color: None,
        text_color: None,
        border_color: None,
        padding_top: 0.0,
        padding_right: 0.0,
        padding_bottom: 0.0,
        padding_left: 0.0,
        margin_top: 0.0,
        margin_right: 0.0,
        margin_bottom: 0.0,
        margin_left: 0.0,
        gap: 0.0,
        width: None,
        height: None,
        min_width: None,
        max_width: None,
        border_radius: 0.0,
        font_size: None,
        font_weight: FontWeight::Normal,
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::Start,
        flex_grow: 0.0,
        opacity: 1.0,
    };
}

impl Default for StyleProps {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeValue {
    Px(f32),
    Fill,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FontWeight {
    #[default]
    Normal,
    Medium,
    Bold,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AlignItems {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

/// Parse a Tailwind-style token string into StyleProps, splitting hover tokens.
///
/// Returns `(base_style, hover_override)`. Tokens prefixed with `hover:` are
/// collected into the hover override. If no hover tokens exist, returns `None`.
pub fn parse_with_hover(tokens: &str, theme: &Theme) -> (StyleProps, Option<StyleProps>) {
    let mut base_tokens = Vec::new();
    let mut hover_tokens = Vec::new();

    for token in tokens.split_whitespace() {
        if let Some(stripped) = token.strip_prefix("hover:") {
            hover_tokens.push(stripped);
        } else {
            base_tokens.push(token);
        }
    }

    let base = parse_tokens(&base_tokens, theme);
    let hover = if hover_tokens.is_empty() {
        None
    } else {
        Some(parse_tokens(&hover_tokens, theme))
    };

    (base, hover)
}

/// Parse a token string into StyleProps (no hover splitting).
pub fn parse(tokens: &str, theme: &Theme) -> StyleProps {
    let token_list: Vec<&str> = tokens.split_whitespace().collect();
    parse_tokens(&token_list, theme)
}

fn parse_tokens(tokens: &[&str], theme: &Theme) -> StyleProps {
    let mut props = StyleProps::default();

    for &token in tokens {
        // Padding
        if let Some(val) = parse_spacing_token(token, "p-") {
            props.padding_top = val;
            props.padding_right = val;
            props.padding_bottom = val;
            props.padding_left = val;
        } else if let Some(val) = parse_spacing_token(token, "px-") {
            props.padding_left = val;
            props.padding_right = val;
        } else if let Some(val) = parse_spacing_token(token, "py-") {
            props.padding_top = val;
            props.padding_bottom = val;
        } else if let Some(val) = parse_spacing_token(token, "pt-") {
            props.padding_top = val;
        } else if let Some(val) = parse_spacing_token(token, "pr-") {
            props.padding_right = val;
        } else if let Some(val) = parse_spacing_token(token, "pb-") {
            props.padding_bottom = val;
        } else if let Some(val) = parse_spacing_token(token, "pl-") {
            props.padding_left = val;
        }
        // Margin
        else if let Some(val) = parse_spacing_token(token, "m-") {
            props.margin_top = val;
            props.margin_right = val;
            props.margin_bottom = val;
            props.margin_left = val;
        } else if let Some(val) = parse_spacing_token(token, "mx-") {
            props.margin_left = val;
            props.margin_right = val;
        } else if let Some(val) = parse_spacing_token(token, "my-") {
            props.margin_top = val;
            props.margin_bottom = val;
        }
        // Gap
        else if let Some(val) = parse_spacing_token(token, "gap-") {
            props.gap = val;
        }
        // Sizing
        else if token == "w-full" {
            props.width = Some(SizeValue::Fill);
        } else if token == "h-full" {
            props.height = Some(SizeValue::Fill);
        } else if let Some(val) = parse_spacing_token(token, "w-") {
            props.width = Some(SizeValue::Px(val));
        } else if let Some(val) = parse_spacing_token(token, "h-") {
            props.height = Some(SizeValue::Px(val));
        } else if let Some(val) = parse_spacing_token(token, "min-w-") {
            props.min_width = Some(SizeValue::Px(val));
        } else if let Some(val) = parse_spacing_token(token, "max-w-") {
            props.max_width = Some(SizeValue::Px(val));
        }
        // Border radius
        else if token == "rounded" {
            props.border_radius = 4.0;
        } else if token == "rounded-lg" {
            props.border_radius = 8.0;
        } else if token == "rounded-xl" {
            props.border_radius = 12.0;
        } else if token == "rounded-full" {
            props.border_radius = 9999.0;
        } else if let Some(val) = parse_spacing_token(token, "rounded-") {
            props.border_radius = val;
        }
        // Typography
        else if token == "text-xs" {
            props.font_size = Some(12.0);
        } else if token == "text-sm" {
            props.font_size = Some(14.0);
        } else if token == "text-base" {
            props.font_size = Some(16.0);
        } else if token == "text-lg" {
            props.font_size = Some(20.0);
        } else if token == "text-xl" {
            props.font_size = Some(26.0);
        } else if token == "text-2xl" {
            props.font_size = Some(32.0);
        } else if token == "font-bold" {
            props.font_weight = FontWeight::Bold;
        } else if token == "font-medium" {
            props.font_weight = FontWeight::Medium;
        }
        // Flex
        else if token == "flex-1" {
            props.flex_grow = 1.0;
        } else if token == "items-center" {
            props.align_items = AlignItems::Center;
        } else if token == "items-start" {
            props.align_items = AlignItems::Start;
        } else if token == "items-end" {
            props.align_items = AlignItems::End;
        } else if token == "justify-center" {
            props.justify_content = JustifyContent::Center;
        } else if token == "justify-end" {
            props.justify_content = JustifyContent::End;
        } else if token == "justify-between" {
            props.justify_content = JustifyContent::SpaceBetween;
        }
        // Theme colors
        else if let Some(color_name) = token.strip_prefix("bg-") {
            if let Some(hex) = color_name.strip_prefix('#') {
                props.bg_color = Color::from_hex(&format!("#{hex}"));
            } else if let Some(c) = theme.color(color_name) {
                props.bg_color = Some(c);
            }
        } else if let Some(color_name) = token.strip_prefix("text-") {
            // Skip typography tokens already handled above
            if !matches!(color_name, "xs" | "sm" | "base" | "lg" | "xl") {
                if let Some(hex) = color_name.strip_prefix('#') {
                    props.text_color = Color::from_hex(&format!("#{hex}"));
                } else if let Some(c) = theme.color(color_name) {
                    props.text_color = Some(c);
                }
            }
        }
    }

    props
}

/// Parse a spacing token like "p-2" → 8.0 (2 * 4px base unit).
fn parse_spacing_token(token: &str, prefix: &str) -> Option<f32> {
    let val_str = token.strip_prefix(prefix)?;
    let num: f32 = val_str.parse().ok()?;
    Some(num * 4.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_padding_and_gap() {
        let theme = Theme::default_slate();
        let props = parse("p-2 gap-3", &theme);
        assert_eq!(props.padding_top, 8.0);
        assert_eq!(props.padding_left, 8.0);
        assert_eq!(props.gap, 12.0);
    }

    #[test]
    fn parse_bg_and_text_color() {
        let theme = Theme::default_slate();
        let props = parse("bg-base text-fg", &theme);
        assert!(props.bg_color.is_some());
        assert!(props.text_color.is_some());
    }

    #[test]
    fn parse_hover_split() {
        let theme = Theme::default_slate();
        let (base, hover) = parse_with_hover("bg-base hover:bg-surface p-2", &theme);
        assert!(base.bg_color.is_some());
        assert_eq!(base.padding_top, 8.0);
        let hover = hover.expect("should have hover style");
        assert!(hover.bg_color.is_some());
        // hover should not inherit base padding
        assert_eq!(hover.padding_top, 0.0);
    }

    #[test]
    fn parse_no_hover() {
        let theme = Theme::default_slate();
        let (_, hover) = parse_with_hover("bg-base p-2", &theme);
        assert!(hover.is_none());
    }

    #[test]
    fn parse_typography() {
        let theme = Theme::default_slate();
        let props = parse("text-sm font-bold", &theme);
        assert_eq!(props.font_size, Some(14.0));
        assert_eq!(props.font_weight, FontWeight::Bold);
    }

    #[test]
    fn parse_flex_and_align() {
        let theme = Theme::default_slate();
        let props = parse("flex-1 items-center justify-between", &theme);
        assert_eq!(props.flex_grow, 1.0);
        assert_eq!(props.align_items, AlignItems::Center);
        assert_eq!(props.justify_content, JustifyContent::SpaceBetween);
    }

    #[test]
    fn parse_rounded() {
        let theme = Theme::default_slate();
        assert_eq!(parse("rounded-lg", &theme).border_radius, 8.0);
        assert_eq!(parse("rounded-full", &theme).border_radius, 9999.0);
    }
}
