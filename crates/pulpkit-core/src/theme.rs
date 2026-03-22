//! Theme loading from Lua configuration files.

use std::collections::HashMap;
use std::path::Path;

use mlua::prelude::*;
use pulpkit_layout::Theme;
use pulpkit_render::Color;

/// Load a Theme from `theme.lua` in the shell directory.
///
/// If `theme.lua` does not exist, returns a default slate theme.
pub fn load_theme(lua: &Lua, shell_dir: &Path) -> anyhow::Result<Theme> {
    let theme_path = shell_dir.join("theme.lua");
    if !theme_path.exists() {
        log::info!("No theme.lua found, using default slate theme");
        return Ok(Theme::default_slate());
    }

    let code = std::fs::read_to_string(&theme_path)?;
    let theme_table: LuaTable = lua
        .load(&code)
        .set_name(theme_path.to_string_lossy())
        .eval()
        .map_err(|e| anyhow::anyhow!("Failed to evaluate theme.lua: {e}"))?;

    let mut colors = HashMap::new();
    if let Ok(colors_table) = theme_table.get::<LuaTable>("colors") {
        for pair in colors_table.pairs::<String, String>() {
            let (name, hex) = pair.map_err(|e| anyhow::anyhow!("Error reading colors: {e}"))?;
            if let Some(c) = Color::from_hex(&hex) {
                colors.insert(name, c);
            }
        }
    }

    let spacing_scale: f32 = theme_table
        .get::<Option<f32>>("spacing_scale")
        .map_err(|e| anyhow::anyhow!("Error reading spacing_scale: {e}"))?
        .unwrap_or(4.0);

    let mut rounding = HashMap::new();
    if let Ok(rounding_table) = theme_table.get::<LuaTable>("rounding") {
        for pair in rounding_table.pairs::<String, f32>() {
            let (name, val) = pair.map_err(|e| anyhow::anyhow!("Error reading rounding: {e}"))?;
            rounding.insert(name, val);
        }
    }

    let mut font_sizes = HashMap::new();
    if let Ok(sizes_table) = theme_table.get::<LuaTable>("font_sizes") {
        for pair in sizes_table.pairs::<String, f32>() {
            let (name, val) =
                pair.map_err(|e| anyhow::anyhow!("Error reading font_sizes: {e}"))?;
            font_sizes.insert(name, val);
        }
    }

    let font_family: String = theme_table
        .get::<Option<String>>("font_family")
        .map_err(|e| anyhow::anyhow!("Error reading font_family: {e}"))?
        .unwrap_or_else(|| "JetBrainsMono Nerd Font".into());

    let default = Theme::default_slate();
    if colors.is_empty() {
        colors = default.colors;
    }
    if rounding.is_empty() {
        rounding = default.rounding;
    }
    if font_sizes.is_empty() {
        font_sizes = default.font_sizes;
    }

    Ok(Theme {
        colors,
        spacing_scale,
        rounding,
        font_sizes,
        font_family,
    })
}
