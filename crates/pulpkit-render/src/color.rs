//! Color type with hex parsing and Skia conversion.

/// An RGBA color with 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create a new color from RGBA components.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Parse a hex color string: `"#rrggbb"` or `"#rrggbbaa"`.
    ///
    /// Returns `None` if the string is not a valid hex color.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#')?;
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self { r, g, b, a: 255 })
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self { r, g, b, a })
            }
            _ => None,
        }
    }

    /// Convert to Skia's `Color4f` (float components, 0.0..1.0).
    pub fn to_skia(&self) -> skia_safe::Color4f {
        skia_safe::Color4f::new(
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }

    /// Convert to Skia's packed `Color` (ARGB u32).
    pub fn to_skia_color(&self) -> skia_safe::Color {
        skia_safe::Color::from_argb(self.a, self.r, self.g, self.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_color() {
        let c = Color::from_hex("#ff8040").unwrap();
        assert_eq!((c.r, c.g, c.b, c.a), (255, 128, 64, 255));
    }

    #[test]
    fn parse_hex_with_alpha() {
        let c = Color::from_hex("#ff804080").unwrap();
        assert_eq!(c.a, 128);
    }

    #[test]
    fn parse_invalid_hex() {
        assert!(Color::from_hex("not_a_color").is_none());
        assert!(Color::from_hex("#xyz").is_none());
    }
}
