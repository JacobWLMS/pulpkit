//! Text measurement using Skia's font system.

use skia_safe::{Font, FontMgr, FontStyle};

/// Measures text dimensions using the system font manager.
pub struct TextRenderer {
    font_mgr: FontMgr,
}

impl TextRenderer {
    /// Create a new text renderer with the default system font manager.
    pub fn new() -> Self {
        Self {
            font_mgr: FontMgr::default(),
        }
    }

    /// Measure the dimensions of the given text string.
    ///
    /// Returns `(width, height)` in pixels. The height is based on the
    /// font metrics (ascent + descent), not the bounding box of the
    /// specific glyphs rendered.
    pub fn measure(&self, text: &str, font_size: f32, font_family: &str) -> (f32, f32) {
        let typeface = self
            .font_mgr
            .match_family_style(font_family, FontStyle::default())
            .or_else(|| {
                self.font_mgr
                    .match_family_style("sans-serif", FontStyle::default())
            });

        let font = match typeface {
            Some(tf) => Font::from_typeface(tf, font_size),
            None => Font::default(),
        };

        let (width, _bounds) = font.measure_str(text, None);
        let (_, metrics) = font.metrics();
        let height = metrics.descent - metrics.ascent;

        (width, height)
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}
