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

    // Search hicolor at preferred sizes (48px for bar icons)
    let sizes = ["48x48", "32x32", "64x64", "scalable", "128x128", "24x24"];
    let dirs = ["/usr/share/icons/hicolor", "/usr/share/pixmaps"];

    for dir in &dirs {
        for size in &sizes {
            let base = format!("{}/{}/apps/{}", dir, size, icon_name);
            for ext in &["", ".png", ".svg", ".svgz"] {
                let path = PathBuf::from(format!("{}{}", base, ext));
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    // Check pixmaps directly
    for ext in &[".png", ".svg", ".xpm", ""] {
        let path = PathBuf::from(format!("/usr/share/pixmaps/{}{}", icon_name, ext));
        if path.exists() {
            return Some(path);
        }
    }

    None
}
