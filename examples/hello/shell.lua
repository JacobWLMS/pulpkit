-- ╔══════════════════════════════════════════════════════════════════════════╗
-- ║  Pulpkit Shell                                                        ║
-- ║  Functional bar with real system integration                          ║
-- ╚══════════════════════════════════════════════════════════════════════════╝

-- ============================================================================
-- Icons (Nerd Font)
-- ============================================================================

local icons = {
  -- Volume
  vol_high   = "󰕾",
  vol_mid    = "󰖀",
  vol_low    = "󰕿",
  vol_mute   = "󰝟",

  -- Battery
  bat_full   = "󰁹",
  bat_good   = "󰂀",
  bat_half   = "󰁾",
  bat_low    = "󰁻",
  bat_empty  = "󰂎",
  bat_charge = "󰂄",

  -- Network
  wifi_on    = "󰤨",
  wifi_off   = "󰤭",

  -- Bluetooth
  bt_on      = "󰂯",
  bt_off     = "󰂲",
  bt_device  = "󰂱",

  -- System
  power      = "󰐥",
  lock       = "󰌾",
  suspend    = "󰤄",
  logout     = "󰗼",
  reboot     = "󰜉",

  -- UI
  brightness = "󰃟",
  calendar   = "󰃭",
  settings   = "󰒓",
  chevron_r  = "󰅂",
  circle     = "󰝥",
  circle_f   = "󰝤",
  dot        = "●",
}

-- ============================================================================
-- Helpers
-- ============================================================================

local function trim(s)
  return s:match("^%s*(.-)%s*$") or s
end

local function read_file(path)
  local f = io.open(path, "r")
  if not f then return nil end
  local c = f:read("*a")
  f:close()
  return trim(c)
end

local function hbutton(style, hover_style, opts, children)
  local hovered = signal(false)
  opts.on_hover = function() hovered:set(true) end
  opts.on_hover_lost = function() hovered:set(false) end
  return button(function()
    return hovered:get() and hover_style or style
  end, opts, children)
end

-- Icon button: common pattern for bar items
local function icon_btn(icon_signal, opts)
  return hbutton(
    "px-3 py-2 items-center",
    "px-3 py-2 items-center bg-overlay",
    opts,
    { text("text-xl text-fg", icon_signal) }
  )
end

-- ============================================================================
-- System State
-- ============================================================================

-- Audio
local volume     = signal(0)
local muted      = signal(false)

local function poll_audio()
  local raw = exec_output("wpctl get-volume @DEFAULT_AUDIO_SINK@")
  local vol_str = raw:match("Volume:%s+([%d%.]+)")
  if vol_str then volume:set(math.floor(tonumber(vol_str) * 100 + 0.5)) end
  muted:set(raw:find("%[MUTED%]") ~= nil)
end
poll_audio()

local function vol_icon()
  if muted:get() then return icons.vol_mute end
  local v = volume:get()
  if v > 66 then return icons.vol_high end
  if v > 33 then return icons.vol_mid end
  return icons.vol_low
end

local function set_volume(v)
  local val = math.floor(math.max(0, math.min(100, v)))
  volume:set(val)
  exec("wpctl set-volume @DEFAULT_AUDIO_SINK@ " .. val .. "%")
end

-- Battery
local bat_pct    = signal(0)
local bat_status = signal("")
local has_battery = read_file("/sys/class/power_supply/BAT0/capacity") ~= nil

local function poll_battery()
  if not has_battery then return end
  local cap = read_file("/sys/class/power_supply/BAT0/capacity")
  local st  = read_file("/sys/class/power_supply/BAT0/status")
  if cap then bat_pct:set(tonumber(cap) or 0) end
  if st then bat_status:set(st) end
end
poll_battery()

local function bat_icon()
  if not has_battery then return "" end
  if bat_status:get() == "Charging" then return icons.bat_charge end
  local p = bat_pct:get()
  if p > 90 then return icons.bat_full end
  if p > 60 then return icons.bat_good end
  if p > 30 then return icons.bat_half end
  if p > 10 then return icons.bat_low end
  return icons.bat_empty
end

-- Clock
local time_str = signal(os.date("%H:%M"))
local date_str = signal(os.date("%A, %B %d"))

-- Workspaces (NIRI IPC or fallback)
local active_ws  = signal(1)
local ws_count   = signal(5)
local is_niri    = env("NIRI_SOCKET") ~= nil

local function poll_workspaces()
  if not is_niri then return end
  local raw = exec_output("niri msg -j workspaces 2>/dev/null")
  if raw == "" then return end
  -- Parse JSON manually (LuaJIT has no json lib by default)
  -- Count workspaces and find active one
  local count = 0
  local active = 1
  for id in raw:gmatch('"id"%s*:%s*(%d+)') do
    count = count + 1
  end
  -- Find active workspace
  -- niri JSON: {"id":1,"idx":1,"name":null,"output":"eDP-1","is_active":true,...}
  for block in raw:gmatch('%b{}') do
    local id = block:match('"idx"%s*:%s*(%d+)')
    local is_active = block:match('"is_active"%s*:%s*true')
    if id and is_active then
      active = tonumber(id)
    end
  end
  if count > 0 then ws_count:set(count) end
  active_ws:set(active)
end
poll_workspaces()

-- ============================================================================
-- Timers
-- ============================================================================

set_interval(function()
  time_str:set(os.date("%H:%M"))
end, 1000)

set_interval(function()
  poll_workspaces()
end, 1000)

set_interval(function()
  poll_audio()
end, 2000)

set_interval(function()
  poll_battery()
  date_str:set(os.date("%A, %B %d"))
end, 30000)

-- ============================================================================
-- Popup Visibility
-- ============================================================================

local show_audio    = signal(false)
local show_power    = signal(false)
local show_launcher = signal(false)
local search_query  = signal("")
local selected_index = signal(1)
local cursor_blink  = signal(true)

-- Blink cursor every 530ms
set_interval(function()
  if show_launcher:get() then
    cursor_blink:set(not cursor_blink:get())
  end
end, 530)

function close_all_popups()
  show_audio:set(false)
  show_power:set(false)
  show_launcher:set(false)
  search_query:set("")
  selected_index:set(1)
end

function toggle_popup(sig)
  local was = sig:get()
  close_all_popups()
  sig:set(not was)
end

-- Global IPC handlers (called by: pulpkit-core msg "toggle_launcher()")
function toggle_launcher() toggle_popup(show_launcher) end
function toggle_power() toggle_popup(show_power) end
function toggle_audio() toggle_popup(show_audio) end

-- ============================================================================
-- Bar
-- ============================================================================

window("bar", {
  monitor = "all",
  anchor  = "top",
  exclusive = true,
  height  = 48,
}, function(ctx)
  return row("w-full h-12 bg-base px-4 items-center", {

    -- ── Left: flex-1 so left+right take equal space → clock is centered
    row("flex-1 items-center gap-2 px-2", {
      -- App launcher button
      icon_btn("󰍉", {
        on_click = function()
          close_all_popups()
          show_launcher:set(not show_launcher:get())
        end,
      }),

      each(function()
        local result = {}
        for i = 1, ws_count:get() do
          table.insert(result, { id = i })
        end
        return result
      end, function(ws)
        return hbutton(
          "px-2 py-2 items-center justify-center",
          "px-2 py-2 items-center justify-center bg-overlay",
          {
            on_click = function()
              if is_niri then
                exec("niri msg action focus-workspace " .. ws.id)
              end
              active_ws:set(ws.id)
            end,
          },
          {
            text(function()
              return active_ws:get() == ws.id
                and "text-base text-primary"
                or "text-base text-muted"
            end, function()
              return active_ws:get() == ws.id
                and icons.circle_f
                or icons.circle
            end),
          }
        )
      end, function(ws) return tostring(ws.id) end, "row"),
    }),

    -- ── Center: Clock (true center — left and right are flex-1) ───────
    text("text-lg text-fg font-medium", function()
      return time_str:get()
    end),

    -- ── Right: flex-1, items pushed to the end ────────────────────────
    row("flex-1 items-center justify-end gap-2", {

      -- Battery
      (function()
        if not has_battery then return spacer() end
        return row("items-center gap-1 px-2", {
          text("text-lg text-fg", function() return bat_icon() end),
          text("text-sm text-muted", function()
            return bat_pct:get() .. "%"
          end),
        })
      end)(),

      -- Volume
      icon_btn(function() return vol_icon() end, {
        on_click = function() toggle_popup(show_audio) end,
        on_scroll_up = function()
          set_volume(volume:get() + 5)
        end,
        on_scroll_down = function()
          set_volume(volume:get() - 5)
        end,
      }),

      -- Power / Date
      icon_btn(icons.power, {
        on_click = function() toggle_popup(show_power) end,
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
  offset = { x = 0, y = 0 },
  visible = show_audio,
  dismiss_on_outside = true,
  width  = 320,
  height = 200,
}, function()
  return col("bg-surface p-5 gap-4", {
    -- Header
    row("items-center gap-3", {
      text("text-xl text-fg", function() return vol_icon() end),
      col("gap-1", {
        text("text-base font-bold text-fg", "Volume"),
        text("text-sm text-muted", function()
          if muted:get() then return "Muted" end
          return volume:get() .. "%"
        end),
      }),
    }),

    -- Slider
    slider("w-full accent-primary", {
      value     = volume,
      on_change = function(v) set_volume(v) end,
      min = 0,
      max = 100,
    }),

    -- Mute toggle
    row("items-center gap-3", {
      text("text-lg text-fg", icons.vol_mute),
      text("text-sm text-dim", "Mute"),
      spacer(),
      toggle("accent-primary", {
        checked   = muted,
        on_change = function(v)
          muted:set(v)
          exec("wpctl set-mute @DEFAULT_AUDIO_SINK@ " .. (v and "1" or "0"))
        end,
      }),
    }),
  })
end)

-- ============================================================================
-- Power Popup
-- ============================================================================

popup("power", {
  parent = "bar",
  anchor = "center",
  offset = { x = 0, y = 0 },
  visible = show_power,
  dismiss_on_outside = true,
  width  = 300,
  height = 320,
  keyboard = true,
  on_key = function(key, utf8)
    if key == "Escape" then show_power:set(false) end
  end,
}, function()
  return col("bg-surface p-5 gap-3", {
    -- Date header
    text("text-base font-bold text-fg", function()
      return date_str:get()
    end),
    text("text-sm text-muted", function()
      return time_str:get()
    end),

    -- Separator space
    spacer(),

    -- Actions
    hbutton(
      "px-4 py-3 items-center gap-3",
      "px-4 py-3 items-center gap-3 bg-overlay",
      { on_click = function() exec("loginctl lock-session") end },
      {
        text("text-xl text-fg", icons.lock),
        text("text-base text-fg", "Lock"),
      }
    ),
    hbutton(
      "px-4 py-3 items-center gap-3",
      "px-4 py-3 items-center gap-3 bg-overlay",
      { on_click = function() exec("systemctl suspend") end },
      {
        text("text-xl text-fg", icons.suspend),
        text("text-base text-fg", "Suspend"),
      }
    ),
    hbutton(
      "px-4 py-3 items-center gap-3",
      "px-4 py-3 items-center gap-3 bg-overlay",
      { on_click = function() exec("systemctl reboot") end },
      {
        text("text-xl text-fg", icons.reboot),
        text("text-base text-fg", "Reboot"),
      }
    ),
    hbutton(
      "px-4 py-3 items-center gap-3",
      "px-4 py-3 items-center gap-3 bg-overlay",
      { on_click = function()
        exec("loginctl terminate-session " .. (env("XDG_SESSION_ID") or ""))
      end },
      {
        text("text-xl text-error", icons.logout),
        text("text-base text-error", "Log Out"),
      }
    ),
  })
end)

-- ============================================================================
-- App Launcher
-- ============================================================================

-- Load desktop entries once at startup
local all_apps = {}
do
  local dir = "/usr/share/applications"
  local home_dir = (env("HOME") or "") .. "/.local/share/applications"
  local function scan_dir(path)
    local ls = exec_output("ls " .. path .. "/*.desktop 2>/dev/null")
    for file in ls:gmatch("[^\n]+") do
      local f = io.open(file, "r")
      if f then
        local content = f:read("*a")
        f:close()
        local name = content:match("Name=([^\n]+)")
        local ex = content:match("Exec=([^\n]+)")
        local nodisplay = content:match("NoDisplay=true")
        if name and ex and not nodisplay then
          -- Strip field codes from Exec (%u, %U, %f, %F, etc.)
          ex = ex:gsub("%%[uUfFdDnNickvm]", ""):gsub("%s+$", "")
          table.insert(all_apps, { name = name, exec = ex })
        end
      end
    end
  end
  scan_dir(dir)
  scan_dir(home_dir)
  table.sort(all_apps, function(a, b) return a.name:lower() < b.name:lower() end)
end

local function filtered_apps()
  local q = search_query:get():lower()
  if q == "" then return all_apps end
  local result = {}
  for _, app in ipairs(all_apps) do
    if app.name:lower():find(q, 1, true) then
      table.insert(result, app)
    end
  end
  return result
end

local function launch_selected()
  local apps = filtered_apps()
  local idx = selected_index:get()
  if idx >= 1 and idx <= #apps then
    exec(apps[idx].exec)
    show_launcher:set(false)
    search_query:set("")
    selected_index:set(1)
  end
end

popup("launcher", {
  parent = "bar",
  anchor = "center",
  offset = { x = 0, y = -100 },  -- nudge slightly above center
  visible = show_launcher,
  dismiss_on_outside = true,
  width = 500,
  height = 440,
  keyboard = true,
  on_key = function(key, utf8)
    if key == "Escape" then
      show_launcher:set(false)
      search_query:set("")
      selected_index:set(1)
    elseif key == "Return" then
      launch_selected()
    elseif key == "BackSpace" then
      local q = search_query:get()
      if #q > 0 then
        search_query:set(q:sub(1, #q - 1))
        selected_index:set(1)
      end
    elseif key == "Up" then
      local idx = selected_index:get()
      if idx > 1 then selected_index:set(idx - 1) end
    elseif key == "Down" then
      local apps = filtered_apps()
      local idx = selected_index:get()
      if idx < #apps then selected_index:set(idx + 1) end
    elseif #utf8 == 1 and utf8:byte() >= 32 then
      search_query:set(search_query:get() .. utf8)
      selected_index:set(1)
    end
  end,
}, function()
  local max_results = 8
  return col("bg-surface p-4 gap-2", {
    -- Search input display
    row("bg-base p-3 gap-2 items-center", {
      text("text-lg text-muted", "󰍉"),
      text("text-base text-fg", function()
        local q = search_query:get()
        local cursor = cursor_blink:get() and "│" or " "
        return q .. cursor
      end),
    }),

    -- Results
    each(function()
      local apps = filtered_apps()
      local result = {}
      for i = 1, math.min(#apps, max_results) do
        table.insert(result, { idx = i, name = apps[i].name, exec = apps[i].exec })
      end
      return result
    end, function(item)
      return hbutton(
        function()
          return selected_index:get() == item.idx
            and "px-3 py-2 items-center gap-3 bg-overlay"
            or "px-3 py-2 items-center gap-3"
        end,
        function()
          return selected_index:get() == item.idx
            and "px-3 py-2 items-center gap-3 bg-card"
            or "px-3 py-2 items-center gap-3 bg-overlay"
        end,
        {
          on_click = function()
            selected_index:set(item.idx)
            launch_selected()
          end,
        },
        {
          text("text-base text-fg", item.name),
        }
      )
    end, function(item) return item.name end),
  })
end)
