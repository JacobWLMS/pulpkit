use std::collections::HashMap;

pub fn resolve_icon(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    if name.starts_with('/') && std::path::Path::new(name).exists() {
        return format!("file://{name}");
    }
    let search = [
        format!("/usr/share/icons/Papirus-Dark/48x48/apps/{name}.svg"),
        format!("/usr/share/icons/Papirus-Dark/64x64/apps/{name}.svg"),
        format!("/usr/share/icons/Papirus/48x48/apps/{name}.svg"),
        format!("/usr/share/icons/hicolor/scalable/apps/{name}.svg"),
        format!("/usr/share/icons/hicolor/scalable/apps/{name}.svgz"),
        format!("/usr/share/icons/hicolor/48x48/apps/{name}.png"),
        format!("/usr/share/icons/hicolor/256x256/apps/{name}.png"),
        format!("/usr/share/pixmaps/{name}.png"),
        format!("/usr/share/pixmaps/{name}.svg"),
    ];
    for p in &search {
        if std::path::Path::new(p).exists() {
            return format!("file://{p}");
        }
    }
    let lower = name.to_lowercase();
    if lower != name {
        for p in &search {
            let pl = p.replace(name, &lower);
            if std::path::Path::new(&pl).exists() {
                return format!("file://{pl}");
            }
        }
    }
    if let Some(last) = name.rsplit('.').next() {
        if last != name {
            return resolve_icon(last);
        }
    }
    String::new()
}

pub fn resolve_icon_from_cache(name: &str, cache: &HashMap<String, String>) -> String {
    cache
        .get(&name.to_lowercase())
        .cloned()
        .unwrap_or_else(|| resolve_icon(name))
}

pub fn build_icon_cache() -> HashMap<String, String> {
    let mut cache = HashMap::new();
    let dirs = [
        "/usr/share/applications".to_string(),
        format!(
            "{}/.local/share/applications",
            std::env::var("HOME").unwrap_or_default()
        ),
    ];
    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "desktop") {
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let mut startup_wm = None;
                let mut icon_name = None;
                let mut in_entry = false;
                for line in content.lines() {
                    if line.starts_with('[') {
                        in_entry = line == "[Desktop Entry]";
                        continue;
                    }
                    if !in_entry {
                        continue;
                    }
                    if let Some(v) = line.strip_prefix("Icon=") {
                        icon_name = Some(v.to_string());
                    } else if let Some(v) = line.strip_prefix("StartupWMClass=") {
                        startup_wm = Some(v.to_lowercase());
                    }
                }
                let fname = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_lowercase());
                if let Some(icon) = &icon_name {
                    let resolved = resolve_icon(icon);
                    if !resolved.is_empty() {
                        if let Some(ref wm) = startup_wm {
                            cache.insert(wm.clone(), resolved.clone());
                        }
                        if let Some(ref f) = fname {
                            cache.insert(f.clone(), resolved.clone());
                        }
                        cache.insert(icon.to_lowercase(), resolved);
                    }
                }
            }
        }
    }
    cache
}
