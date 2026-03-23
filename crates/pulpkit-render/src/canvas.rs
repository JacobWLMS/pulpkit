//! tiny-skia raster canvas that renders into a raw pixel buffer.
//!
//! The pixel buffer is in Wayland's ARGB8888 format. tiny-skia uses
//! premultiplied RGBA internally, so we configure the pixmap accordingly.

use tiny_skia::{
    FillRule, Mask, Paint, PathBuilder, Pixmap, PixmapMut, PixmapRef, Rect, Transform,
};

use crate::color::Color;

/// Saved canvas state for clip/transform stack.
struct CanvasState {
    transform: Transform,
    clip: Option<Mask>,
}

/// A tiny-skia raster surface that draws into a caller-owned pixel buffer.
pub struct Canvas<'a> {
    pixmap: PixmapMut<'a>,
    transform: Transform,
    clip: Option<Mask>,
    state_stack: Vec<CanvasState>,
}

impl<'a> Canvas<'a> {
    /// Create a canvas backed by the given pixel buffer (RGBA premultiplied).
    ///
    /// Buffer must be `width * height * 4` bytes.
    pub fn new(pixmap: PixmapMut<'a>) -> Self {
        Canvas {
            pixmap,
            transform: Transform::identity(),
            clip: None,
            state_stack: Vec::new(),
        }
    }

    /// Create a canvas from a raw byte buffer.
    pub fn from_buffer(data: &'a mut [u8], width: u32, height: u32) -> Option<Self> {
        let pixmap = PixmapMut::from_bytes(data, width, height)?;
        Some(Self::new(pixmap))
    }

    /// Clear the entire surface with the given color.
    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(color.to_tiny_skia());
    }

    /// Fill a rectangle with the given color.
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let Some(rect) = Rect::from_xywh(x, y, w, h) else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia());
        paint.anti_alias = false;
        self.pixmap.fill_rect(
            rect,
            &paint,
            self.transform,
            self.clip.as_ref(),
        );
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
        if radius <= 0.0 {
            self.fill_rect(x, y, w, h, color);
            return;
        }

        let r = radius.min(w / 2.0).min(h / 2.0);
        let mut pb = PathBuilder::new();
        pb.move_to(x + r, y);
        pb.line_to(x + w - r, y);
        pb.quad_to(x + w, y, x + w, y + r);
        pb.line_to(x + w, y + h - r);
        pb.quad_to(x + w, y + h, x + w - r, y + h);
        pb.line_to(x + r, y + h);
        pb.quad_to(x, y + h, x, y + h - r);
        pb.line_to(x, y + r);
        pb.quad_to(x, y, x + r, y);
        pb.close();

        let Some(path) = pb.finish() else { return };
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia());
        paint.anti_alias = true;

        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            self.transform,
            self.clip.as_ref(),
        );
    }

    /// Draw an image (Pixmap) at (x, y) scaled to (w, h).
    pub fn draw_image(&mut self, x: f32, y: f32, w: f32, h: f32, image: &Pixmap) {
        let iw = image.width() as f32;
        let ih = image.height() as f32;
        if iw == 0.0 || ih == 0.0 {
            return;
        }

        let sx = w / iw;
        let sy = h / ih;
        let image_transform = Transform::from_scale(sx, sy).post_translate(x, y);
        let combined = self.transform.pre_concat(image_transform);

        let paint = tiny_skia::PixmapPaint::default();
        self.pixmap.draw_pixmap(
            0,
            0,
            image.as_ref(),
            &paint,
            combined,
            self.clip.as_ref(),
        );
    }

    /// Save the current canvas state (transform + clip).
    pub fn save(&mut self) {
        self.state_stack.push(CanvasState {
            transform: self.transform,
            clip: self.clip.clone(),
        });
    }

    /// Restore the previously saved canvas state.
    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.transform = state.transform;
            self.clip = state.clip;
        }
    }

    /// Clip all subsequent drawing to the given rectangle.
    /// Coordinates are in logical space — the current transform is applied.
    pub fn clip_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let width = self.pixmap.width();
        let height = self.pixmap.height();

        let Some(rect) = Rect::from_xywh(x, y, w, h) else {
            return;
        };
        let clip_path = PathBuilder::from_rect(rect);

        if let Some(ref mut mask) = self.clip {
            let _ = mask.intersect_path(
                &clip_path,
                FillRule::Winding,
                false,
                self.transform,
            );
        } else {
            let mut mask = Mask::new(width, height).unwrap();
            mask.fill_path(
                &clip_path,
                FillRule::Winding,
                false,
                self.transform,
            );
            self.clip = Some(mask);
        }
    }

    /// Translate all subsequent drawing by (dx, dy).
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.transform = self.transform.pre_translate(dx, dy);
    }

    /// Scale all subsequent drawing by (sx, sy).
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.transform = self.transform.pre_scale(sx, sy);
    }

    /// Get pixmap dimensions.
    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Access the underlying pixmap as a reference (for reading pixels in tests).
    pub fn pixmap_ref(&self) -> PixmapRef<'_> {
        self.pixmap.as_ref()
    }

    /// Draw text onto the canvas via the TextRenderer.
    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        family: &str,
        color: crate::Color,
        renderer: &crate::TextRenderer,
    ) {
        // Apply full transform (scale + translation) to position and font size.
        // The transform matrix is: [sx 0 tx; 0 sy ty; 0 0 1]
        // So transformed point = (x * sx + tx, y * sy + ty)
        let tx = x * self.transform.sx + self.transform.tx;
        let ty = y * self.transform.sy + self.transform.ty;
        let scaled_size = size * self.transform.sy;
        renderer.draw_text(&mut self.pixmap, text, tx, ty, family, scaled_size, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_sets_all_pixels() {
        let mut data = vec![0u8; 100 * 100 * 4];
        let mut canvas = Canvas::from_buffer(&mut data, 100, 100).unwrap();
        canvas.clear(Color::new(255, 255, 255, 255));
        let px = canvas.pixmap_ref().pixel(0, 0).unwrap();
        assert_eq!(px.red(), 255);
        assert_eq!(px.green(), 255);
        assert_eq!(px.blue(), 255);
        assert_eq!(px.alpha(), 255);
    }

    #[test]
    fn fill_rect_writes_pixels() {
        let mut data = vec![0u8; 100 * 100 * 4];
        let mut canvas = Canvas::from_buffer(&mut data, 100, 100).unwrap();
        canvas.clear(Color::new(0, 0, 0, 255));
        canvas.fill_rect(10.0, 10.0, 20.0, 20.0, Color::new(255, 0, 0, 255));
        let px = canvas.pixmap_ref().pixel(15, 15).unwrap();
        assert_eq!(px.red(), 255);
        assert_eq!(px.green(), 0);
        let px2 = canvas.pixmap_ref().pixel(5, 5).unwrap();
        assert_eq!(px2.red(), 0);
    }

    #[test]
    fn rounded_rect_draws_without_panic() {
        let mut data = vec![0u8; 100 * 100 * 4];
        let mut canvas = Canvas::from_buffer(&mut data, 100, 100).unwrap();
        canvas.draw_rounded_rect(10.0, 10.0, 50.0, 30.0, 8.0, Color::new(0, 128, 255, 255));
        let px = canvas.pixmap_ref().pixel(35, 25).unwrap();
        assert!(px.alpha() > 0);
    }

    #[test]
    fn save_restore_resets_transform() {
        let mut data = vec![0u8; 100 * 100 * 4];
        let mut canvas = Canvas::from_buffer(&mut data, 100, 100).unwrap();
        canvas.save();
        canvas.translate(50.0, 50.0);
        canvas.restore();
        canvas.fill_rect(0.0, 0.0, 5.0, 5.0, Color::new(255, 0, 0, 255));
        let px = canvas.pixmap_ref().pixel(2, 2).unwrap();
        assert_eq!(px.red(), 255);
    }
}
