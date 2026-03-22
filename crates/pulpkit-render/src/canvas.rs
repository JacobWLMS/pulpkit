//! Skia raster canvas that renders into a raw pixel buffer.
//!
//! The pixel buffer is expected to be in Wayland's ARGB8888 format,
//! which on little-endian systems is BGRA in memory — matching Skia's
//! `ColorType::BGRA8888`.

use skia_safe::{
    AlphaType, ColorType, Font, FontMgr, FontStyle, ImageInfo, Paint, PaintStyle, RRect, Rect,
    Surface, surfaces,
};
use skia_safe::Borrows;

use crate::color::Color;

/// A Skia raster surface that draws into a caller-owned pixel buffer.
pub struct Canvas<'a> {
    surface: Borrows<'a, Surface>,
}

impl<'a> Canvas<'a> {
    /// Create a Skia raster surface backed by the given pixel buffer.
    ///
    /// Buffer format: Wayland ARGB8888 / Skia BGRA8888 on little-endian,
    /// with premultiplied alpha. `width` and `height` are in pixels.
    ///
    /// The buffer must be at least `width * height * 4` bytes.
    pub fn from_buffer(data: &'a mut [u8], width: i32, height: i32) -> Option<Self> {
        let info = ImageInfo::new(
            (width, height),
            ColorType::BGRA8888,
            AlphaType::Premul,
            None,
        );
        let row_bytes = width as usize * 4;

        let surface = surfaces::wrap_pixels(&info, data, Some(row_bytes), None)?;

        Some(Canvas { surface })
    }

    /// Clear the entire surface with the given color.
    pub fn clear(&mut self, color: Color) {
        self.surface.canvas().clear(color.to_skia_color());
    }

    /// Draw a filled rounded rectangle.
    pub fn draw_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        color: Color,
    ) {
        let rect = Rect::from_xywh(x, y, w, h);
        let rrect = RRect::new_rect_xy(rect, radius, radius);

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);
        paint.set_color(color.to_skia_color());

        self.surface.canvas().draw_rrect(rrect, &paint);
    }

    /// Draw text at the given position.
    ///
    /// `(x, y)` is the top-left corner of the text. The font is looked up
    /// by family name from the system font manager.
    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        font_family: &str,
        color: Color,
    ) {
        let font_mgr = FontMgr::default();
        let typeface = font_mgr
            .match_family_style(font_family, FontStyle::default())
            .or_else(|| font_mgr.match_family_style("sans-serif", FontStyle::default()));

        let font = match typeface {
            Some(tf) => Font::from_typeface(tf, font_size),
            None => Font::default(),
        };

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(color.to_skia_color());

        // Skia draws text at the baseline, so offset y by the ascent
        // to make (x, y) act as the top-left of the text.
        let (_, metrics) = font.metrics();
        let baseline_y = y - metrics.ascent;

        self.surface
            .canvas()
            .draw_str(text, (x, baseline_y), &font, &paint);
    }

    /// Flush all pending drawing operations to the backing pixel buffer.
    ///
    /// For raster (CPU) surfaces, drawing is immediate so this is a no-op.
    /// Provided for API consistency with GPU-backed surfaces.
    pub fn flush(&mut self) {
        // Raster surfaces write directly to the pixel buffer — nothing to flush.
    }
}
