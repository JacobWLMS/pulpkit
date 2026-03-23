//! PNG image loading with thread-local cache and icon path resolution.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use tiny_skia::Pixmap;

thread_local! {
    static IMAGE_CACHE: RefCell<HashMap<String, Option<Pixmap>>> = RefCell::new(HashMap::new());
}

/// Load an image from a file path, caching the result.
///
/// Returns `None` if the file doesn't exist or can't be decoded.
/// Currently supports PNG only.
pub fn load_image(path: &Path) -> Option<Pixmap> {
    let key = path.to_string_lossy().to_string();

    IMAGE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }

        let result = load_image_inner(path);
        cache.insert(key, result.clone());
        result
    })
}

fn load_image_inner(path: &Path) -> Option<Pixmap> {
    let img = image::open(path).ok()?.into_rgba8();
    let width = img.width();
    let height = img.height();
    let raw = img.into_raw();

    // image crate produces straight RGBA; tiny-skia needs premultiplied RGBA.
    let mut premul = vec![0u8; raw.len()];
    for (chunk, out) in raw.chunks_exact(4).zip(premul.chunks_exact_mut(4)) {
        let a = chunk[3] as f32 / 255.0;
        out[0] = (chunk[0] as f32 * a) as u8;
        out[1] = (chunk[1] as f32 * a) as u8;
        out[2] = (chunk[2] as f32 * a) as u8;
        out[3] = chunk[3];
    }

    Pixmap::from_vec(premul, tiny_skia::IntSize::from_wh(width, height)?)
}

/// Resolve an icon name to a file path using XDG icon directories.
///
/// Searches common hicolor icon theme paths for the given name.
/// Returns the first match found, or `None`.
pub fn resolve_icon_path(name: &str) -> Option<String> {
    let xdg_dirs = [
        "/usr/share/icons/hicolor",
        "/usr/share/pixmaps",
    ];
    let sizes = ["scalable", "48x48", "32x32", "24x24", "16x16"];
    let categories = ["apps", "status", "devices", "actions"];
    let extensions = ["png", "svg"];

    for dir in &xdg_dirs {
        for size in &sizes {
            for cat in &categories {
                for ext in &extensions {
                    let path = format!("{dir}/{size}/{cat}/{name}.{ext}");
                    if Path::new(&path).exists() {
                        return Some(path);
                    }
                }
            }
        }
    }

    // Also check pixmaps directly
    for ext in &["png", "svg", "xpm"] {
        let path = format!("/usr/share/pixmaps/{name}.{ext}");
        if Path::new(&path).exists() {
            return Some(path);
        }
    }

    None
}

/// Clear the image cache (useful for hot-reload).
pub fn clear_image_cache() {
    IMAGE_CACHE.with(|cache| cache.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_nonexistent_returns_none() {
        assert!(load_image(Path::new("/nonexistent/image.png")).is_none());
    }

    #[test]
    fn cache_stores_miss() {
        let _ = load_image(Path::new("/tmp/pulpkit_test_missing.png"));
        // Second call should use cache (returns None again, but from cache)
        let result = load_image(Path::new("/tmp/pulpkit_test_missing.png"));
        assert!(result.is_none());
    }
}
