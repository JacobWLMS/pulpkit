//! Image loading and caching for icons and other raster content.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use skia_safe::{Data, Image};

thread_local! {
    static IMAGE_CACHE: RefCell<HashMap<PathBuf, Option<Image>>> = RefCell::new(HashMap::new());
}

/// Load an image from a file path. Cached per thread.
/// Returns None if the file doesn't exist or can't be decoded.
pub fn load_image(path: &Path) -> Option<Image> {
    IMAGE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                let bytes = std::fs::read(path).ok()?;
                let data = Data::new_copy(&bytes);
                Image::from_encoded(data)
            })
            .clone()
    })
}

/// Resolve an icon name from a .desktop file to an actual file path.
/// Searches hicolor theme at common sizes.
pub fn resolve_icon_path(icon_name: &str) -> Option<PathBuf> {
    // Already a full path
    if icon_name.starts_with('/') {
        let p = PathBuf::from(icon_name);
        if p.exists() {
            return Some(p);
        }
    }

    // Search hicolor at preferred sizes — PNG only (Skia can't decode SVG).
    // Prefer larger sizes for better quality when scaling down.
    let sizes = ["48x48", "64x64", "128x128", "32x32", "256x256", "24x24", "192x192", "16x16"];

    for size in &sizes {
        let path = PathBuf::from(format!(
            "/usr/share/icons/hicolor/{}/apps/{}.png", size, icon_name
        ));
        if path.exists() {
            return Some(path);
        }
    }

    // Check pixmaps (PNG only)
    for ext in &[".png", ""] {
        let path = PathBuf::from(format!("/usr/share/pixmaps/{}{}", icon_name, ext));
        if path.exists() && path.extension().is_some_and(|e| e == "png") {
            return Some(path);
        }
    }

    None
}
