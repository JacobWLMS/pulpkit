-- ╔══════════════════════════════════════════════════════════════════════════╗
-- ║  Pulpkit Shell                                                        ║
-- ║  Built with the standard library — clean and hackable                 ║
-- ╚══════════════════════════════════════════════════════════════════════════╝

-- Load stdlib — shell_dir is set by the runtime before shell.lua runs
local lib = dofile(shell_dir .. "/lib.lua")
local icons = lib.icons

-- ============================================================================
-- State
-- ============================================================================

local volume      = signal(0)
local muted       = signal(false)
local bat_pct     = signal(0)
local bat_status  = signal("")
local active_ws   = signal(1)
local ws_count    = signal(5)
local time_str    = signal(os.date("%H:%M"))
local date_str    = signal(os.date("%A, %B %d"))
local has_battery = lib.read_file("/sys/class/power_supply/BAT0/capacity") ~= nil

-- ============================================================================
-- Polling
-- ============================================================================

local function refresh_all()
  local v, m = lib.poll_volume()
  volume:set(v); muted:set(m)
  if has_battery then
    local p, s = lib.poll_battery()
    if p then bat_pct:set(p); bat_status:set(s) end
  end
  local a, c = lib.poll_niri_workspaces()
  active_ws:set(a); ws_count:set(c)
end
refresh_all()

set_interval(function() time_str:set(os.date("%H:%M")) end, 1000)
set_interval(function() refresh_all() end, 2000)
set_interval(function() date_str:set(os.date("%A, %B %d")) end, 30000)

-- ============================================================================
-- Popups
-- ============================================================================

local show_audio,   toggle_audio,   close_audio   = lib.popup_toggle()
local show_power,   toggle_power,   close_power   = lib.popup_toggle()
local show_launcher, toggle_launcher, close_launcher = lib.popup_toggle()
local search_query   = signal("")
local selected_index = signal(1)
local cursor_blink   = signal(true)

set_interval(function()
  if show_launcher:get() then cursor_blink:set(not cursor_blink:get()) end
end, 530)

function close_all_popups()
  close_audio(); close_power(); close_launcher()
  search_query:set(""); selected_index:set(1)
end

-- Global IPC handlers
function _toggle_launcher() close_all_popups(); toggle_launcher() end
function _toggle_power()    close_all_popups(); toggle_power() end
function _toggle_audio()    close_all_popups(); toggle_audio() end

-- ============================================================================
-- Bar
-- ============================================================================

window("bar", {
  monitor = "all",
  anchor  = "top",
  exclusive = true,
  height  = 48,
}, function(ctx)
  return row("w-full h-12 bg-base px-2 items-center gap-1", {

    -- Left
    row("flex-1 items-center gap-1", {
      lib.icon_btn(icons.search, {
        on_click = function() close_all_popups(); toggle_launcher() end,
      }),

      each(function()
        local r = {}
        for i = 1, ws_count:get() do r[#r+1] = { id = i } end
        return r
      end, function(ws)
        return lib.icon_btn(function()
          return active_ws:get() == ws.id and icons.circle_f or icons.circle
        end, {
          icon_style = function()
            return active_ws:get() == ws.id and "text-base text-primary" or "text-base text-muted"
          end,
          on_click = function()
            lib.focus_workspace(ws.id)
            active_ws:set(ws.id)
          end,
        })
      end, function(ws) return tostring(ws.id) end, "row"),
    }),

    -- Center
    text("text-lg text-fg font-medium", function() return time_str:get() end),

    -- Right
    row("flex-1 items-center justify-end gap-1", {
      -- Battery
      (function()
        if not has_battery then return spacer() end
        return row("items-center gap-1 px-1", {
          text("text-lg text-fg", function()
            return lib.bat_icon(bat_pct:get(), bat_status:get())
          end),
          text("text-sm text-muted", function() return bat_pct:get() .. "%" end),
        })
      end)(),

      -- Volume
      lib.icon_btn(function()
        return lib.vol_icon(volume:get(), muted:get())
      end, {
        on_click = function() close_all_popups(); toggle_audio() end,
        on_scroll_up = function()
          volume:set(lib.set_volume(volume:get() + 5))
        end,
        on_scroll_down = function()
          volume:set(lib.set_volume(volume:get() - 5))
        end,
      }),

      -- Power
      lib.icon_btn(icons.power, {
        on_click = function() close_all_popups(); toggle_power() end,
      }),
    }),
  })
end)

-- ============================================================================
-- Audio Popup
-- ============================================================================

popup("audio", {
  parent = "bar",
  anchor = "top right",
  visible = show_audio,
  dismiss_on_outside = true,
  width = 320, height = 200,
}, function()
  return col("bg-surface p-5 gap-4", {
    row("items-center gap-3", {
      text("text-xl text-fg", function()
        return lib.vol_icon(volume:get(), muted:get())
      end),
      col("gap-1", {
        lib.header("Volume"),
        lib.caption(function()
          return muted:get() and "Muted" or (volume:get() .. "%")
        end),
      }),
    }),

    lib.slider_row("", volume, {
      on_change = function(v)
        volume:set(lib.set_volume(v))
      end,
    }),

    lib.toggle_row("Mute", muted, {
      on_change = function(v) muted:set(v); lib.set_mute(v) end,
    }),
  })
end)

-- ============================================================================
-- Power Popup
-- ============================================================================

popup("power", {
  parent = "bar",
  anchor = "center",
  visible = show_power,
  dismiss_on_outside = true,
  width = 300, height = 320,
  keyboard = true,
  on_key = function(key) if key == "Escape" then close_all_popups() end end,
}, function()
  return col("bg-surface p-5 gap-3", {
    lib.header(function() return date_str:get() end),
    lib.caption(function() return time_str:get() end),
    spacer(),
    lib.menu_item(icons.lock,    "Lock",    { on_click = function() exec("loginctl lock-session") end }),
    lib.menu_item(icons.suspend, "Suspend", { on_click = function() exec("systemctl suspend") end }),
    lib.menu_item(icons.reboot,  "Reboot",  { on_click = function() exec("systemctl reboot") end }),
    lib.menu_item(icons.logout,  "Log Out", {
      text_style = "text-base text-error",
      icon_style = "text-xl text-error",
      on_click = function()
        exec("loginctl terminate-session " .. (env("XDG_SESSION_ID") or ""))
      end,
    }),
  })
end)

-- ============================================================================
-- App Launcher
-- ============================================================================

local all_apps = {}
do
  local function scan(path)
    for file in exec_output("ls " .. path .. "/*.desktop 2>/dev/null"):gmatch("[^\n]+") do
      local c = lib.read_file(file)
      if c then
        local name = c:match("Name=([^\n]+)")
        local ex   = c:match("Exec=([^\n]+)")
        local cats = c:match("Categories=([^\n]+)") or ""
        if name and ex and not c:match("NoDisplay=true") then
          ex = ex:gsub("%%[uUfFdDnNickvm]", ""):gsub("%s+$", "")
          local icon_name = c:match("Icon=([^\n]+)") or ""
          local icon_path = resolve_icon(icon_name)
          all_apps[#all_apps+1] = {
            name = name,
            exec = ex,
            icon_path = icon_path,
            icon_fallback = lib.app_icon(name, ex, cats),
          }
        end
      end
    end
  end
  scan("/usr/share/applications")
  scan((env("HOME") or "") .. "/.local/share/applications")
  table.sort(all_apps, function(a, b) return a.name:lower() < b.name:lower() end)
end

local function filtered_apps()
  local q = search_query:get():lower()
  if q == "" then return all_apps end
  local r = {}
  for _, app in ipairs(all_apps) do
    if app.name:lower():find(q, 1, true) then r[#r+1] = app end
  end
  return r
end

local function launch_selected()
  local apps = filtered_apps()
  local idx = selected_index:get()
  if idx >= 1 and idx <= #apps then
    exec(apps[idx].exec)
    close_all_popups()
  end
end

popup("launcher", {
  parent = "bar",
  anchor = "center",
  visible = show_launcher,
  dismiss_on_outside = true,
  width = 500, height = 440,
  keyboard = true,
  on_key = function(key, utf8)
    if key == "Escape" then
      close_all_popups()
    elseif key == "Return" then
      launch_selected()
    elseif key == "BackSpace" then
      local q = search_query:get()
      if #q > 0 then search_query:set(q:sub(1, #q - 1)); selected_index:set(1) end
    elseif key == "Up" then
      local i = selected_index:get()
      if i > 1 then selected_index:set(i - 1) end
    elseif key == "Down" then
      local i = selected_index:get()
      if i < #filtered_apps() then selected_index:set(i + 1) end
    elseif #utf8 == 1 and utf8:byte() >= 32 then
      search_query:set(search_query:get() .. utf8)
      selected_index:set(1)
    end
  end,
}, function()
  return col("bg-surface p-4 gap-2", {
    row("bg-base p-3 gap-2 items-center", {
      text("text-lg text-muted", icons.search),
      text("text-base text-fg", function()
        return search_query:get() .. (cursor_blink:get() and "│" or " ")
      end),
    }),

    -- Virtual scroll: show a window of 8 items around the selection
    each(function()
      local apps = filtered_apps()
      local sel = selected_index:get()
      local max_visible = 8
      -- Compute scroll window
      local start = 1
      if sel > max_visible then
        start = sel - max_visible + 1
      end
      local r = {}
      for i = start, math.min(start + max_visible - 1, #apps) do
        r[#r+1] = {
          idx = i,
          name = apps[i].name,
          exec = apps[i].exec,
          icon_path = apps[i].icon_path,
          icon_fallback = apps[i].icon_fallback,
        }
      end
      return r
    end, function(item)
      local children = {}
      -- App icon (real PNG or fallback glyph)
      if item.icon_path then
        children[#children+1] = image(item.icon_path, 24, 24)
      else
        children[#children+1] = text("text-lg text-muted", item.icon_fallback or icons.app)
      end
      children[#children+1] = text("text-base text-fg", item.name)

      local hovered = signal(false)
      return button(function()
        local is_sel = selected_index:get() == item.idx
        local is_hov = hovered:get()
        if is_sel then return "px-2 py-1 items-center gap-3 bg-overlay" end
        if is_hov then return "px-2 py-1 items-center gap-3 bg-overlay" end
        return "px-2 py-1 items-center gap-3"
      end, {
        on_click = function()
          selected_index:set(item.idx)
          launch_selected()
        end,
        on_hover = function() hovered:set(true) end,
        on_hover_lost = function() hovered:set(false) end,
      }, children)
    end, function(item) return item.name end),
  })
end)
