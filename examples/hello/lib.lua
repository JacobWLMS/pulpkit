-- ╔══════════════════════════════════════════════════════════════════════════╗
-- ║  Pulpkit Standard Library                                             ║
-- ╚══════════════════════════════════════════════════════════════════════════╝

local lib = {}

-- ============================================================================
-- Design tokens — change these to restyle everything
-- ============================================================================

lib.spacing = {
  btn_pad     = "px-2 py-1",     -- standard button padding
  popup_pad   = "p-4",           -- popup inner padding
  popup_gap   = "gap-3",         -- gap between popup sections
  item_gap    = "gap-2",         -- gap between icon and label in items
}

lib.text_styles = {
  icon       = "text-lg",        -- icon size in bar and popups
  icon_large = "text-xl",        -- large icon (popup headers)
  body       = "text-base",      -- body text
  small      = "text-sm",        -- small text
  caption    = "text-xs",        -- captions and muted info
}

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

function lib.read_file(path)
  local f = io.open(path, "r")
  if not f then return nil end
  local c = f:read("*a")
  f:close()
  return (c:match("^%s*(.-)%s*$"))
end

-- ============================================================================
-- Widgets
-- ============================================================================

--- Button with automatic hover.
function lib.btn(label, opts)
  opts = opts or {}
  local base = opts.style or lib.spacing.btn_pad
  local hover = opts.hover or "bg-overlay"
  local ts = opts.text_style or (lib.text_styles.body .. " text-fg")
  local h = signal(false)

  local style_fn
  if type(base) == "function" then
    style_fn = function()
      local s = base()
      return h:get() and (s .. " " .. hover) or s
    end
  else
    style_fn = function()
      return h:get() and (base .. " " .. hover) or base
    end
  end

  return button(style_fn, {
    on_click = opts.on_click,
    on_scroll_up = opts.on_scroll_up,
    on_scroll_down = opts.on_scroll_down,
    on_hover = function() h:set(true) end,
    on_hover_lost = function() h:set(false) end,
  }, {
    text(ts, label),
  })
end

--- Icon button.
function lib.icon_btn(icon, opts)
  opts = opts or {}
  local base = opts.style or lib.spacing.btn_pad
  local hover = opts.hover or "bg-overlay"
  local is = opts.icon_style or (lib.text_styles.icon .. " text-fg")
  local h = signal(false)

  return button(function()
    return h:get() and (base .. " " .. hover) or base
  end, {
    on_click = opts.on_click,
    on_scroll_up = opts.on_scroll_up,
    on_scroll_down = opts.on_scroll_down,
    on_hover = function() h:set(true) end,
    on_hover_lost = function() h:set(false) end,
  }, {
    text(is, type(icon) == "function" and icon or icon),
  })
end

--- Menu item: icon + label row.
function lib.menu_item(icon, label, opts)
  opts = opts or {}
  local base = opts.style or (lib.spacing.btn_pad .. " items-center " .. lib.spacing.item_gap)
  local hover = opts.hover or "bg-overlay"
  local is = opts.icon_style or (lib.text_styles.icon .. " text-fg")
  local ts = opts.text_style or (lib.text_styles.body .. " text-fg")
  local h = signal(false)

  return button(function()
    return h:get() and (base .. " " .. hover) or base
  end, {
    on_click = opts.on_click,
    on_hover = function() h:set(true) end,
    on_hover_lost = function() h:set(false) end,
  }, {
    text(is, icon),
    text(ts, label),
  })
end

--- Slider with optional label.
function lib.slider_row(label, value_signal, opts)
  opts = opts or {}
  local children = {}
  if label and label ~= "" then
    children[#children+1] = row("items-center " .. lib.spacing.item_gap, {
      text(lib.text_styles.small .. " font-bold text-fg", label),
      spacer(),
      text(lib.text_styles.small .. " text-muted", function()
        return math.floor(value_signal:get()) .. "%"
      end),
    })
  end
  children[#children+1] = slider("w-full accent-primary", {
    value = value_signal,
    on_change = opts.on_change,
    min = opts.min or 0,
    max = opts.max or 100,
  })
  return col(lib.spacing.item_gap, children)
end

--- Toggle row.
function lib.toggle_row(label, checked_signal, opts)
  opts = opts or {}
  return row("items-center " .. lib.spacing.item_gap, {
    text(lib.text_styles.small .. " text-fg", label),
    spacer(),
    toggle("accent-primary", {
      checked = checked_signal,
      on_change = opts.on_change,
    }),
  })
end

--- Section header.
function lib.header(title)
  return text(lib.text_styles.small .. " font-bold text-fg", title)
end

--- Caption.
function lib.caption(content)
  return text(lib.text_styles.caption .. " text-muted", content)
end

-- ============================================================================
-- System helpers
-- ============================================================================

function lib.poll_volume()
  local raw = exec_output("wpctl get-volume @DEFAULT_AUDIO_SINK@")
  local vol_str = raw:match("Volume:%s+([%d%.]+)")
  local vol = vol_str and math.floor(tonumber(vol_str) * 100 + 0.5) or 0
  local muted = raw:find("%[MUTED%]") ~= nil
  return vol, muted
end

function lib.set_volume(v)
  local val = math.floor(math.max(0, math.min(100, v)))
  exec("wpctl set-volume @DEFAULT_AUDIO_SINK@ " .. val .. "%")
  return val
end

function lib.set_mute(muted)
  exec("wpctl set-mute @DEFAULT_AUDIO_SINK@ " .. (muted and "1" or "0"))
end

function lib.vol_icon(vol, muted)
  if muted then return lib.icons.vol_mute end
  if vol > 66 then return lib.icons.vol_high end
  if vol > 33 then return lib.icons.vol_mid end
  return lib.icons.vol_low
end

function lib.poll_battery()
  local cap = lib.read_file("/sys/class/power_supply/BAT0/capacity")
  local status = lib.read_file("/sys/class/power_supply/BAT0/status")
  if cap then return tonumber(cap), status end
  return nil, nil
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

function lib.app_icon(name, exec, categories)
  local e = (exec or ""):lower()
  local c = (categories or ""):lower()

  if e:find("ghostty") or e:find("alacritty") or e:find("kitty") or e:find("foot")
    or e:find("wezterm") or e:find("terminal") then return lib.icons.terminal end
  if e:find("firefox") or e:find("chrom") or e:find("brave") or e:find("zen")
    or e:find("vivaldi") then return lib.icons.browser end
  if e:find("code") or e:find("nvim") or e:find("vim") or e:find("zed")
    or e:find("emacs") or e:find("helix") then return lib.icons.code end
  if e:find("nautilus") or e:find("thunar") or e:find("dolphin") or e:find("nemo")
    or e:find("pcmanfm") then return lib.icons.files end
  if e:find("spotify") or e:find("music") or e:find("rhythmbox") then return lib.icons.music end
  if e:find("mpv") or e:find("vlc") or e:find("celluloid") then return lib.icons.video end
  if e:find("gimp") or e:find("inkscape") or e:find("krita") then return lib.icons.image end
  if e:find("steam") or e:find("lutris") or e:find("heroic") then return lib.icons.game end
  if e:find("thunderbird") or e:find("geary") or e:find("mail") then return lib.icons.mail end
  if e:find("discord") or e:find("telegram") or e:find("signal") then return lib.icons.chat end

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

function lib.focus_workspace(idx)
  if env("NIRI_SOCKET") then
    exec("niri msg action focus-workspace " .. idx)
  end
end

-- ============================================================================
-- Popup helpers
-- ============================================================================

function lib.popup_toggle()
  local show = signal(false)
  return show,
    function() show:set(not show:get()) end,
    function() show:set(false) end
end

-- ============================================================================
-- Compound widgets
-- ============================================================================

--- Radio group: mutually exclusive options.
--- Usage: lib.radio_group({"Power Saver", "Balanced", "Performance"}, selected_signal)
function lib.radio_group(options, selected_signal, opts)
  opts = opts or {}
  local children = {}
  for i, option in ipairs(options) do
    local label = type(option) == "table" and option.label or option
    local value = type(option) == "table" and option.value or i
    local icon = type(option) == "table" and option.icon or nil
    local h = signal(false)

    local btn_children = {}
    if icon then
      btn_children[#btn_children+1] = text(lib.text_styles.icon .. " text-fg", icon)
    end
    btn_children[#btn_children+1] = text(function()
      return selected_signal:get() == value
        and (lib.text_styles.body .. " text-primary font-bold")
        or (lib.text_styles.body .. " text-fg")
    end, label)

    children[#children+1] = button(function()
      local sel = selected_signal:get() == value
      local hov = h:get()
      if sel then return lib.spacing.btn_pad .. " items-center " .. lib.spacing.item_gap .. " bg-overlay" end
      if hov then return lib.spacing.btn_pad .. " items-center " .. lib.spacing.item_gap .. " bg-overlay" end
      return lib.spacing.btn_pad .. " items-center " .. lib.spacing.item_gap
    end, {
      on_click = function()
        selected_signal:set(value)
        if opts.on_change then opts.on_change(value) end
      end,
      on_hover = function() h:set(true) end,
      on_hover_lost = function() h:set(false) end,
    }, btn_children)
  end

  local direction = opts.direction or "col"
  if direction == "row" then
    return row("items-center " .. (opts.gap or lib.spacing.item_gap), children)
  else
    return col(opts.gap or lib.spacing.item_gap, children)
  end
end

--- Scrollable list wrapper.
--- Usage: lib.scroll_list(style, children)
function lib.scroll_list(style, children)
  return scroll(style, children)
end

--- Separator line.
function lib.separator()
  return row("w-full h-1 bg-outline", {})
end

--- Info row: label on left, value on right.
function lib.info_row(label, value)
  return row("items-center " .. lib.spacing.item_gap, {
    text(lib.text_styles.small .. " text-muted", label),
    spacer(),
    text(lib.text_styles.small .. " text-fg", value),
  })
end

return lib
