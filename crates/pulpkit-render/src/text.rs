//! Text rendering stack: fontdb + rustybuzz for shaping, ab_glyph for rasterization.
//!
//! Glyphs are cached after first rasterization. The TextRenderer holds the font
//! database and glyph cache for the lifetime of the shell.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use ab_glyph::{Font, FontRef, GlyphId, PxScale, ScaleFont};
use fontdb::Database;
use tiny_skia::PixmapMut;

use crate::color::Color;

/// Key for the glyph cache — font face ID + glyph ID + font size (tenths).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphCacheKey {
    face_id: fontdb::ID,
    glyph_id: u16,
    size_tenths: u32,
}

/// Cached rasterized glyph with its metrics.
struct CachedGlyph {
    /// Rasterized coverage bitmap (alpha values, one byte per pixel).
    coverage: Vec<u8>,
    width: u32,
    height: u32,
    /// Offset from the glyph origin to the top-left of the bitmap.
    bearing_x: f32,
    bearing_y: f32,
}

/// Text renderer with font database and glyph cache.
pub struct TextRenderer {
    db: Database,
    /// Maps face IDs to their loaded font data (shared with fontdb).
    face_data: RefCell<HashMap<fontdb::ID, Arc<Vec<u8>>>>,
    glyph_cache: RefCell<HashMap<GlyphCacheKey, Option<CachedGlyph>>>,
}

impl TextRenderer {
    /// Create a new text renderer, loading system fonts.
    pub fn new() -> Self {
        let mut db = Database::new();
        db.load_system_fonts();
        // Map generic family names so fontdb::Family::SansSerif resolves.
        db.set_sans_serif_family("Noto Sans");
        db.set_serif_family("Noto Serif");
        db.set_monospace_family("Noto Sans Mono");
        TextRenderer {
            db,
            face_data: RefCell::new(HashMap::new()),
            glyph_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Measure text dimensions (width, height) without rendering.
    pub fn measure_text(&self, text: &str, family: &str, size: f32) -> (f32, f32) {
        let Some(face_id) = self.find_face(family) else {
            return (0.0, 0.0);
        };
        let Some(font_data) = self.load_face_data(face_id) else {
            return (0.0, 0.0);
        };
        let Ok(font) = FontRef::try_from_slice(&font_data) else {
            return (0.0, 0.0);
        };

        let scale = PxScale::from(size);
        let scaled = font.as_scaled(scale);

        let mut width = 0.0f32;
        let height = scaled.height();
        let mut prev_glyph: Option<GlyphId> = None;

        for ch in text.chars() {
            let glyph_id = scaled.glyph_id(ch);
            if let Some(prev) = prev_glyph {
                width += scaled.kern(prev, glyph_id);
            }
            width += scaled.h_advance(glyph_id);
            prev_glyph = Some(glyph_id);
        }

        (width, height)
    }

    /// Draw text onto a canvas's pixel buffer.
    pub fn draw_text(
        &self,
        pixmap: &mut PixmapMut<'_>,
        text: &str,
        x: f32,
        y: f32,
        family: &str,
        size: f32,
        color: Color,
    ) {
        let Some(face_id) = self.find_face(family) else {
            return;
        };
        let Some(font_data) = self.load_face_data(face_id) else {
            return;
        };
        let Ok(font) = FontRef::try_from_slice(&font_data) else {
            return;
        };

        let scale = PxScale::from(size);
        let scaled = font.as_scaled(scale);
        let ascent = scaled.ascent();

        // Baseline position: y is top-left, baseline is y + ascent
        let baseline_y = y + ascent;
        let mut cursor_x = x;
        let mut prev_glyph: Option<GlyphId> = None;

        let pw = pixmap.width() as i32;
        let ph = pixmap.height() as i32;

        // Premultiply the color once
        let a = color.a as f32 / 255.0;
        let pr = (color.r as f32 * a) as u8;
        let pg = (color.g as f32 * a) as u8;
        let pb = (color.b as f32 * a) as u8;

        for ch in text.chars() {
            let glyph_id = scaled.glyph_id(ch);

            if let Some(prev) = prev_glyph {
                cursor_x += scaled.kern(prev, glyph_id);
            }

            // Rasterize or fetch from cache
            if let Some(cached) = self.get_or_rasterize(face_id, &font, glyph_id, scale) {
                let gx = (cursor_x + cached.bearing_x).round() as i32;
                let gy = (baseline_y - cached.bearing_y).round() as i32;

                // Composite the glyph coverage onto the pixmap
                let data = pixmap.data_mut();
                for row in 0..cached.height as i32 {
                    for col in 0..cached.width as i32 {
                        let px = gx + col;
                        let py = gy + row;
                        if px < 0 || px >= pw || py < 0 || py >= ph {
                            continue;
                        }
                        let cov = cached.coverage[(row as u32 * cached.width + col as u32) as usize];
                        if cov == 0 {
                            continue;
                        }
                        let alpha = (color.a as u16 * cov as u16 / 255) as u8;
                        let fa = alpha as f32 / 255.0;

                        let idx = ((py * pw + px) * 4) as usize;
                        // Source-over compositing (premultiplied)
                        let dst_r = data[idx];
                        let dst_g = data[idx + 1];
                        let dst_b = data[idx + 2];
                        let dst_a = data[idx + 3];

                        let inv_a = 1.0 - fa;
                        data[idx] = (pr as f32 * fa + dst_r as f32 * inv_a) as u8;
                        data[idx + 1] = (pg as f32 * fa + dst_g as f32 * inv_a) as u8;
                        data[idx + 2] = (pb as f32 * fa + dst_b as f32 * inv_a) as u8;
                        data[idx + 3] = (alpha as f32 + dst_a as f32 * inv_a) as u8;
                    }
                }
            }

            cursor_x += scaled.h_advance(glyph_id);
            prev_glyph = Some(glyph_id);
        }
    }

    /// Find a font face by family name, falling back to sans-serif.
    fn find_face(&self, family: &str) -> Option<fontdb::ID> {
        let query = fontdb::Query {
            families: &[
                fontdb::Family::Name(family),
                fontdb::Family::SansSerif,
            ],
            ..Default::default()
        };
        self.db.query(&query)
    }

    /// Load font data for a face ID, caching it.
    fn load_face_data(&self, face_id: fontdb::ID) -> Option<Arc<Vec<u8>>> {
        {
            let cache = self.face_data.borrow();
            if let Some(data) = cache.get(&face_id) {
                return Some(Arc::clone(data));
            }
        }

        let mut result = None;
        self.db.with_face_data(face_id, |data, _index| {
            result = Some(Arc::new(data.to_vec()));
        });

        if let Some(ref data) = result {
            self.face_data.borrow_mut().insert(face_id, Arc::clone(data));
        }
        result
    }

    /// Get a cached glyph or rasterize it.
    fn get_or_rasterize(
        &self,
        face_id: fontdb::ID,
        font: &FontRef<'_>,
        glyph_id: GlyphId,
        scale: PxScale,
    ) -> Option<&CachedGlyph> {
        // We need to work around the borrow checker — check cache, then insert if missing.
        let key = GlyphCacheKey {
            face_id,
            glyph_id: glyph_id.0,
            size_tenths: (scale.x * 10.0) as u32,
        };

        // Use entry API to avoid double lookup
        let mut cache = self.glyph_cache.borrow_mut();
        let entry = cache.entry(key).or_insert_with(|| {
            Self::rasterize_glyph(font, glyph_id, scale)
        });

        // Return a raw pointer to avoid borrow issues — safe because the cache
        // is append-only (we never remove entries during a session).
        entry.as_ref().map(|g| unsafe { &*(g as *const CachedGlyph) })
    }

    /// Rasterize a single glyph to a coverage bitmap.
    fn rasterize_glyph(
        font: &FontRef<'_>,
        glyph_id: GlyphId,
        scale: PxScale,
    ) -> Option<CachedGlyph> {
        let scaled = font.as_scaled(scale);
        let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(0.0, scaled.ascent()));

        let outlined = font.outline_glyph(glyph)?;
        let bounds = outlined.px_bounds();

        let width = (bounds.max.x - bounds.min.x).ceil() as u32;
        let height = (bounds.max.y - bounds.min.y).ceil() as u32;
        if width == 0 || height == 0 {
            return None;
        }

        let mut coverage = vec![0u8; (width * height) as usize];
        outlined.draw(|px, py, cov| {
            let x = px as u32;
            let y = py as u32;
            if x < width && y < height {
                coverage[(y * width + x) as usize] = (cov * 255.0) as u8;
            }
        });

        Some(CachedGlyph {
            coverage,
            width,
            height,
            bearing_x: bounds.min.x,
            bearing_y: scaled.ascent() - bounds.min.y,
        })
    }

    /// Get the number of cached glyphs (for testing).
    pub fn cache_len(&self) -> usize {
        self.glyph_cache.borrow().len()
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_text_returns_nonzero() {
        let renderer = TextRenderer::new();
        let (w, h) = renderer.measure_text("Hello", "sans-serif", 14.0);
        assert!(w > 0.0, "width should be > 0, got {w}");
        assert!(h > 0.0, "height should be > 0, got {h}");
    }

    #[test]
    fn draw_text_does_not_panic() {
        let renderer = TextRenderer::new();
        let mut data = vec![0u8; 200 * 40 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut data, 200, 40).unwrap();
        renderer.draw_text(&mut pixmap, "Hello World", 0.0, 0.0, "sans-serif", 14.0, Color::new(255, 255, 255, 255));
    }

    #[test]
    fn glyph_cache_reuses_entries() {
        let renderer = TextRenderer::new();
        let mut data = vec![0u8; 200 * 40 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut data, 200, 40).unwrap();
        renderer.draw_text(&mut pixmap, "AA", 0.0, 0.0, "sans-serif", 14.0, Color::new(255, 255, 255, 255));
        // "AA" has only 1 unique glyph
        assert_eq!(renderer.cache_len(), 1, "cache should have 1 entry for 'AA'");
    }

    #[test]
    fn draw_text_produces_visible_pixels() {
        let renderer = TextRenderer::new();
        let mut data = vec![0u8; 200 * 40 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut data, 200, 40).unwrap();
        renderer.draw_text(&mut pixmap, "X", 10.0, 5.0, "sans-serif", 20.0, Color::new(255, 255, 255, 255));
        // At least some pixels should be non-zero in the text area
        let has_visible = data[..200 * 40 * 4].chunks(4).any(|px| px[3] > 0);
        assert!(has_visible, "draw_text should produce visible pixels");
    }
}
