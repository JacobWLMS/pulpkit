-- ╔══════════════════════════════════════════════════════════════════════════╗
-- ║  Pulpkit Standard Library                                             ║
-- ║  High-level widgets so you don't need a CS degree                     ║
-- ╚══════════════════════════════════════════════════════════════════════════╝

local lib = {}

-- ============================================================================
-- Icons (Nerd Font)
-- ============================================================================

lib.icons = {
  vol_high   = "󰕾",  vol_mid  = "󰖀",  vol_low    = "󰕿",  vol_mute = "󰝟",
  bat_full   = "󰁹",  bat_good = "󰂀",  bat_half   = "󰁾",  bat_low  = "󰁻",
  bat_empty  = "󰂎",  bat_charge = "󰂄",
  wifi_on    = "󰤨",  wifi_off = "󰤭",
  bt_on      = "󰂯",  bt_off   = "󰂲",
  power      = "󰐥",  lock     = "󰌾",  suspend    = "󰤄",
  logout     = "󰗼",  reboot   = "󰜉",
  brightness = "󰃟",  calendar = "󰃭",  settings   = "󰒓",
  search     = "󰍉",  close    = "󰅖",  check      = "󰄬",
  app        = "󰀻",  terminal = "󰆍",  browser    = "󰖟",
  files      = "󰉋",  code     = "󰨞",  music      = "󰝚",
  video      = "󰕧",  image    = "󰋩",  game       = "󰊗",
  mail       = "󰇮",  chat     = "󰍡",  settings2  = "󰒓",
  text       = "󰧮",  download = "󰇚",  camera     = "󰄀",
  chevron_r  = "󰅂",  chevron_l = "󰅁",
  circle     = "󰝥",  circle_f = "󰝤",  dot = "●",
}

-- ============================================================================
-- Core helpers
-- ============================================================================

--- Read a file, return trimmed contents or nil.
function lib.read_file(path)
  local f = io.open(path, "r")
  if not f then return nil end
  local c = f:read("*a")
  f:close()
  return (c:match("^%s*(.-)%s*$"))
end

-- ============================================================================
-- Widgets — the good stuff
-- ============================================================================

--- A button with automatic hover highlighting.
--- Usage: lib.btn("Label", { on_click = fn })
---        lib.btn("Label", { on_click = fn, style = "px-4 py-2" })
function lib.btn(label, opts)
  opts = opts or {}
  local base_style = opts.style or "px-2 py-1"
  local hover_add = opts.hover or "bg-overlay"
  local text_style = opts.text_style or "text-base text-fg"
  local hovered = signal(false)

  local style_fn
  if type(base_style) == "function" then
    -- Reactive style: caller provides full style function, we add hover
    style_fn = function()
      local s = base_style()
      return hovered:get() and (s .. " " .. hover_add) or s
    end
  else
    style_fn = function()
      return hovered:get() and (base_style .. " " .. hover_add) or base_style
    end
  end

  return button(style_fn, {
    on_click = opts.on_click,
    on_scroll_up = opts.on_scroll_up,
    on_scroll_down = opts.on_scroll_down,
    on_hover = function() hovered:set(true) end,
    on_hover_lost = function() hovered:set(false) end,
  }, {
    text(text_style, label),
  })
end

--- An icon button (single glyph, hover highlight).
--- Usage: lib.icon_btn("󰕾", { on_click = fn })
function lib.icon_btn(icon, opts)
  opts = opts or {}
  local style = opts.style or "px-2 py-1"
  local hover = opts.hover or "bg-overlay"
  local icon_style = opts.icon_style or "text-xl text-fg"
  local hovered = signal(false)

  local icon_content = icon
  if type(icon) == "function" then
    icon_content = icon -- reactive icon
  end

  return button(function()
    return hovered:get() and (style .. " " .. hover) or style
  end, {
    on_click = opts.on_click,
    on_scroll_up = opts.on_scroll_up,
    on_scroll_down = opts.on_scroll_down,
    on_hover = function() hovered:set(true) end,
    on_hover_lost = function() hovered:set(false) end,
  }, {
    text(icon_style, icon_content),
  })
end

--- A labeled row with icon + text (common pattern for menu items).
--- Usage: lib.menu_item("󰌾", "Lock", { on_click = fn })
function lib.menu_item(icon, label, opts)
  opts = opts or {}
  local style = opts.style or "px-3 py-2 items-center gap-3"
  local hover = opts.hover or "bg-overlay"
  local icon_style = opts.icon_style or "text-xl text-fg"
  local text_style = opts.text_style or "text-base text-fg"
  local hovered = signal(false)

  return button(function()
    return hovered:get() and (style .. " " .. hover) or style
  end, {
    on_click = opts.on_click,
    on_hover = function() hovered:set(true) end,
    on_hover_lost = function() hovered:set(false) end,
  }, {
    text(icon_style, icon),
    text(text_style, label),
  })
end

--- A slider with label and value display.
--- Usage: lib.slider_row("Volume", vol_signal, { min=0, max=100, on_change=fn })
function lib.slider_row(label, value_signal, opts)
  opts = opts or {}
  return col("gap-2", {
    row("items-center gap-2", {
      text("text-sm font-bold text-fg", label),
      spacer(),
      text("text-sm text-muted", function()
        return math.floor(value_signal:get()) .. "%"
      end),
    }),
    slider("w-full accent-primary", {
      value = value_signal,
      on_change = opts.on_change,
      min = opts.min or 0,
      max = opts.max or 100,
    }),
  })
end

--- A toggle row: label + toggle switch on the right.
--- Usage: lib.toggle_row("Mute", muted_signal, { on_change = fn })
function lib.toggle_row(label, checked_signal, opts)
  opts = opts or {}
  return row("items-center gap-3", {
    text("text-sm text-fg", label),
    spacer(),
    toggle("accent-primary", {
      checked = checked_signal,
      on_change = opts.on_change,
    }),
  })
end

--- A section header.
function lib.header(title)
  return text("text-sm font-bold text-fg", title)
end

--- A muted caption.
function lib.caption(content)
  return text("text-xs text-muted", content)
end

-- ============================================================================
-- System helpers
-- ============================================================================

--- Poll PipeWire volume. Returns (volume_0_100, is_muted).
function lib.poll_volume()
  local raw = exec_output("wpctl get-volume @DEFAULT_AUDIO_SINK@")
  local vol_str = raw:match("Volume:%s+([%d%.]+)")
  local vol = vol_str and math.floor(tonumber(vol_str) * 100 + 0.5) or 0
  local muted = raw:find("%[MUTED%]") ~= nil
  return vol, muted
end

--- Set PipeWire volume (0-100).
function lib.set_volume(v)
  local val = math.floor(math.max(0, math.min(100, v)))
  exec("wpctl set-volume @DEFAULT_AUDIO_SINK@ " .. val .. "%")
  return val
end

--- Toggle PipeWire mute.
function lib.set_mute(muted)
  exec("wpctl set-mute @DEFAULT_AUDIO_SINK@ " .. (muted and "1" or "0"))
end

--- Get volume icon for current state.
function lib.vol_icon(vol, muted)
  if muted then return lib.icons.vol_mute end
  if vol > 66 then return lib.icons.vol_high end
  if vol > 33 then return lib.icons.vol_mid end
  return lib.icons.vol_low
end

--- Poll battery. Returns (percent, status) or (nil, nil) if no battery.
function lib.poll_battery()
  local cap = lib.read_file("/sys/class/power_supply/BAT0/capacity")
  local status = lib.read_file("/sys/class/power_supply/BAT0/status")
  if cap then return tonumber(cap), status end
  return nil, nil
end

--- Get battery icon.
--- Guess an icon for an app based on its name, exec, or .desktop categories.
function lib.app_icon(name, exec, categories)
  local n = (name or ""):lower()
  local e = (exec or ""):lower()
  local c = (categories or ""):lower()

  -- Match by exec name
  if e:find("ghostty") or e:find("alacritty") or e:find("kitty") or e:find("foot")
    or e:find("wezterm") or e:find("terminal") then return lib.icons.terminal end
  if e:find("firefox") or e:find("chrom") or e:find("brave") or e:find("zen")
    or e:find("vivaldi") then return lib.icons.browser end
  if e:find("code") or e:find("nvim") or e:find("vim") or e:find("zed")
    or e:find("emacs") or e:find("helix") then return lib.icons.code end
  if e:find("nautilus") or e:find("thunar") or e:find("dolphin") or e:find("nemo")
    or e:find("pcmanfm") then return lib.icons.files end
  if e:find("spotify") or e:find("music") or e:find("rhythmbox")
    or e:find("lollypop") then return lib.icons.music end
  if e:find("mpv") or e:find("vlc") or e:find("celluloid") or e:find("totem")
    then return lib.icons.video end
  if e:find("gimp") or e:find("inkscape") or e:find("krita")
    then return lib.icons.image end
  if e:find("steam") or e:find("lutris") or e:find("heroic") or e:find("game")
    then return lib.icons.game end
  if e:find("thunderbird") or e:find("geary") or e:find("mail")
    then return lib.icons.mail end
  if e:find("discord") or e:find("telegram") or e:find("signal") or e:find("element")
    then return lib.icons.chat end

  -- Match by category
  if c:find("terminal") then return lib.icons.terminal end
  if c:find("browser") or c:find("web") then return lib.icons.browser end
  if c:find("develop") or c:find("ide") then return lib.icons.code end
  if c:find("filemanager") then return lib.icons.files end
  if c:find("audio") or c:find("music") then return lib.icons.music end
  if c:find("video") then return lib.icons.video end
  if c:find("game") then return lib.icons.game end
  if c:find("setting") then return lib.icons.settings2 end

  return lib.icons.app
end

function lib.bat_icon(pct, status)
  if not pct then return "" end
  if status == "Charging" then return lib.icons.bat_charge end
  if pct > 90 then return lib.icons.bat_full end
  if pct > 60 then return lib.icons.bat_good end
  if pct > 30 then return lib.icons.bat_half end
  if pct > 10 then return lib.icons.bat_low end
  return lib.icons.bat_empty
end

--- Poll NIRI workspaces. Returns { active_idx, count }.
function lib.poll_niri_workspaces()
  if not env("NIRI_SOCKET") then return 1, 5 end
  local raw = exec_output("niri msg -j workspaces 2>/dev/null")
  if raw == "" then return 1, 5 end
  local count = 0
  local active = 1
  for block in raw:gmatch('%b{}') do
    count = count + 1
    local idx = block:match('"idx"%s*:%s*(%d+)')
    if block:match('"is_active"%s*:%s*true') and idx then
      active = tonumber(idx)
    end
  end
  return active, math.max(count, 1)
end

--- Switch NIRI workspace.
function lib.focus_workspace(idx)
  if env("NIRI_SOCKET") then
    exec("niri msg action focus-workspace " .. idx)
  end
end

-- ============================================================================
-- Popup helpers
-- ============================================================================

--- Create a popup visibility toggle system.
--- Returns: show_signal, toggle_fn, close_fn
function lib.popup_toggle()
  local show = signal(false)
  local function toggle()
    show:set(not show:get())
  end
  local function close()
    show:set(false)
  end
  return show, toggle, close
end

return lib
