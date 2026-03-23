//! Image loading and caching for icons and other raster content.
//! Supports PNG (via Skia) and SVG (via resvg) with thread-local caching.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use skia_safe::{self, Data, Image};

thread_local! {
    static IMAGE_CACHE: RefCell<HashMap<PathBuf, Option<Image>>> = RefCell::new(HashMap::new());
}

/// Load an image from a file path. Cached per thread.
/// Supports PNG, JPEG, WebP (via Skia) and SVG/SVGZ (via resvg).
pub fn load_image(path: &Path) -> Option<Image> {
    IMAGE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                let ext = path.extension()?.to_str()?;
                match ext {
                    "svg" | "svgz" => load_svg(path),
                    _ => load_raster(path),
                }
            })
            .clone()
    })
}

/// Load a raster image (PNG, JPEG, WebP) via Skia.
fn load_raster(path: &Path) -> Option<Image> {
    let bytes = std::fs::read(path).ok()?;
    let data = Data::new_copy(&bytes);
    Image::from_encoded(data)
}

/// Load and rasterize an SVG file via resvg.
/// Renders to a 128x128 RGBA bitmap, then converts to a Skia Image.
fn load_svg(path: &Path) -> Option<Image> {
    let data = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;

    let render_size = 256u32; // Large for crisp downscaling
    let mut pixmap = resvg::tiny_skia::Pixmap::new(render_size, render_size)?;

    let svg_size = tree.size();
    let scale = render_size as f32 / svg_size.width().max(svg_size.height());
    let offset_x = (render_size as f32 - svg_size.width() * scale) / 2.0;
    let offset_y = (render_size as f32 - svg_size.height() * scale) / 2.0;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
        .post_translate(offset_x, offset_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert tiny_skia RGBA (unpremultiplied) → BGRA premultiplied for Skia.
    let pixels = pixmap.data();
    let mut bgra = Vec::with_capacity(pixels.len());
    for chunk in pixels.chunks_exact(4) {
        let (r, g, b, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
        // Premultiply and swap to BGRA
        let af = a as f32 / 255.0;
        bgra.push((b as f32 * af) as u8);
        bgra.push((g as f32 * af) as u8);
        bgra.push((r as f32 * af) as u8);
        bgra.push(a);
    }

    let skia_data = Data::new_copy(&bgra);
    let info = skia_safe::ImageInfo::new(
        (render_size as i32, render_size as i32),
        skia_safe::ColorType::BGRA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    skia_safe::images::raster_from_data(&info, skia_data, render_size as usize * 4)
}

/// Resolve an icon name from a .desktop file to an actual file path.
/// Searches hicolor theme: largest PNG first, then scalable SVG, then pixmaps.
/// Handles icon name variations (reverse-domain stripping, case).
pub fn resolve_icon_path(icon_name: &str) -> Option<PathBuf> {
    if icon_name.is_empty() {
        return None;
    }

    // Already a full path
    if icon_name.starts_with('/') {
        let p = PathBuf::from(icon_name);
        if p.exists() {
            return Some(p);
        }
    }

    // Try exact name, then variations
    let names = icon_name_variations(icon_name);
    for name in &names {
        if let Some(path) = search_icon(name) {
            return Some(path);
        }
    }

    None
}

/// Generate name variations for icon lookup.
fn icon_name_variations(name: &str) -> Vec<String> {
    let mut names = vec![name.to_string()];

    // Strip reverse-domain: "org.gnome.Nautilus" → "nautilus"
    if name.contains('.') {
        if let Some(last) = name.rsplit('.').next() {
            let lower = last.to_lowercase();
            if lower != name {
                names.push(lower);
            }
        }
    }

    // Lowercase variant
    let lower = name.to_lowercase();
    if lower != name && !names.contains(&lower) {
        names.push(lower);
    }

    names
}

/// Search for an icon by exact name across all sizes and formats.
fn search_icon(name: &str) -> Option<PathBuf> {
    // PNG: largest first for best downscale quality
    let sizes = [
        "256x256", "192x192", "128x128", "96x96",
        "64x64", "48x48", "32x32", "24x24", "16x16",
    ];

    for size in &sizes {
        let path = PathBuf::from(format!(
            "/usr/share/icons/hicolor/{}/apps/{}.png", size, name
        ));
        if path.exists() {
            return Some(path);
        }
    }

    // SVG: scalable directory
    for ext in &["svg", "svgz"] {
        let path = PathBuf::from(format!(
            "/usr/share/icons/hicolor/scalable/apps/{}.{}", name, ext
        ));
        if path.exists() {
            return Some(path);
        }
    }

    // Pixmaps fallback (any format)
    for ext in &[".png", ".svg", ".xpm"] {
        let path = PathBuf::from(format!("/usr/share/pixmaps/{}{}", name, ext));
        if path.exists() {
            return Some(path);
        }
    }

    None
}
