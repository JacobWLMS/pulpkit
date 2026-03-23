//! Color type — RGBA, hex parsing, premultiplied ARGB conversion.

/// An RGBA color with 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Parse a hex color string: `#RGB`, `#RRGGBB`, or `#AARRGGBB`.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#')?;
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                Some(Self::new(r, g, b, 255))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::new(r, g, b, 255))
            }
            8 => {
                let a = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let r = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let g = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let b = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::new(r, g, b, a))
            }
            _ => None,
        }
    }

    /// Convert to premultiplied ARGB u32 (Wayland ARGB8888 format).
    pub fn to_premultiplied_argb_u32(self) -> u32 {
        let a = self.a as u32;
        let r = (self.r as u32 * a / 255) & 0xFF;
        let g = (self.g as u32 * a / 255) & 0xFF;
        let b = (self.b as u32 * a / 255) & 0xFF;
        (a << 24) | (r << 16) | (g << 8) | b
    }

    /// Convert to a `tiny_skia::Color` (premultiplied float RGBA).
    pub fn to_tiny_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_hex_6_digit() {
        let c = Color::from_hex("#ff0000").unwrap();
        assert_eq!(c, Color::new(255, 0, 0, 255));
    }

    #[test]
    fn from_hex_8_digit_alpha() {
        let c = Color::from_hex("#80ff0000").unwrap();
        assert_eq!(c, Color::new(255, 0, 0, 128));
    }

    #[test]
    fn from_hex_3_digit() {
        let c = Color::from_hex("#f00").unwrap();
        assert_eq!(c, Color::new(255, 0, 0, 255));
    }

    #[test]
    fn default_is_transparent_black() {
        assert_eq!(Color::default(), Color::new(0, 0, 0, 0));
    }

    #[test]
    fn invalid_hex_returns_none() {
        assert!(Color::from_hex("#gg0000").is_none());
        assert!(Color::from_hex("not-a-color").is_none());
        assert!(Color::from_hex("#12345").is_none());
    }

    #[test]
    fn premultiplied_argb() {
        let c = Color::new(255, 0, 0, 128);
        let argb = c.to_premultiplied_argb_u32();
        let a = (argb >> 24) & 0xFF;
        let r = (argb >> 16) & 0xFF;
        assert_eq!(a, 128);
        assert_eq!(r, 128); // 255 * 128 / 255 = 128
    }
}
