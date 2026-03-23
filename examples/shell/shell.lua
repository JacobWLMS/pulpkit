-- ╔══════════════════════════════════════════════════╗
-- ║  Pulpkit v3 Shell — Niri desktop bar            ║
-- ╚══════════════════════════════════════════════════╝

-- Nerd Font icons
local I = {
  search   = "\u{f002}",
  circle   = "\u{f111}",
  circle_o = "\u{f10c}",
  clock    = "\u{f017}",
  calendar = "\u{f073}",
  wifi     = "\u{f1eb}",
  bt       = "\u{f294}",
  vol_hi   = "\u{f028}",
  vol_lo   = "\u{f027}",
  vol_off  = "\u{f026}",
  vol_mute = "\u{f6a9}",
  bat_full = "\u{f240}",
  bat_half = "\u{f242}",
  bat_low  = "\u{f243}",
  bat_chrg = "\u{f0e7}",
  bright   = "\u{f185}",
  power    = "\u{f011}",
  lock     = "\u{f023}",
  logout   = "\u{f2f5}",
  reboot   = "\u{f021}",
  shut     = "\u{f011}",
  suspend  = "\u{f186}",
  nix      = "\u{f313}",
}

-- ============================================================================
-- State
-- ============================================================================

function init()
  return {
    -- System
    time      = os.date("%H:%M"),
    date      = os.date("%A, %B %d"),
    user      = "...",
    host      = "...",
    -- Audio
    vol       = 0,
    muted     = false,
    -- Battery
    bat_pct   = 0,
    bat_state = "unknown",
    -- Brightness
    bright    = 50,
    -- Workspaces
    workspaces = {},
    -- Popups
    popup     = nil,  -- "audio" | "power" | "calendar" | nil
  }
end

-- ============================================================================
-- Update
-- ============================================================================

function update(state, msg)
  local t = msg.type

  -- Timers
  if t == "tick" then
    state.time = os.date("%H:%M")
    state.date = os.date("%A, %B %d")

  -- System info
  elseif t == "user" then state.user = msg.data or "?"
  elseif t == "host" then state.host = msg.data or "?"

  -- Audio
  elseif t == "audio_info" then
    if msg.data then
      local vol = msg.data:match("(%d+)%%")
      if vol then state.vol = tonumber(vol) end
      state.muted = msg.data:find("%[MUTED%]") ~= nil
    end
  elseif t == "set_vol" then
    state.vol = math.floor(msg.data or state.vol)
    os.execute("wpctl set-volume @DEFAULT_AUDIO_SINK@ " .. (state.vol / 100))

  -- Battery
  elseif t == "bat_info" then
    if msg.data then
      local pct = msg.data:match("(%d+)")
      if pct then state.bat_pct = tonumber(pct) end
    end
  elseif t == "bat_state_info" then
    if msg.data then state.bat_state = msg.data end

  -- Brightness
  elseif t == "bright_info" then
    if msg.data then
      local b = msg.data:match("(%d+)")
      if b then state.bright = tonumber(b) end
    end
  elseif t == "set_bright" then
    state.bright = math.floor(msg.data or state.bright)
    os.execute("brightnessctl set " .. state.bright .. "%")

  -- Workspaces
  elseif t == "ws_info" then
    if msg.data then
      local ws = {}
      for id, idx, focused in msg.data:gmatch('"id":(%d+).-"idx":(%d+).-"is_focused":(.-)[,}]') do
        table.insert(ws, {
          id = tostring(id),
          idx = tonumber(idx),
          active = focused:find("true") ~= nil,
        })
      end
      if #ws > 0 then state.workspaces = ws end
    end

  -- Popup toggle
  elseif t == "toggle" then
    local name = msg.data
    if type(name) == "string" then
      state.popup = state.popup == name and nil or name
    end
  elseif t == "dismiss" then
    state.popup = nil

  -- Power actions
  elseif t == "lock"     then os.execute("loginctl lock-session &")
  elseif t == "logout"   then os.execute("niri msg action quit &")
  elseif t == "suspend"  then os.execute("systemctl suspend &")
  elseif t == "reboot"   then os.execute("systemctl reboot &")
  elseif t == "shutdown" then os.execute("systemctl poweroff &")
  end

  return state
end

-- ============================================================================
-- Helpers
-- ============================================================================

local function icon_btn(icon, action, extra_style)
  return button({ on_click = action, style = "p-2 rounded-full hover:bg-surface " .. (extra_style or "") },
    text({ style = "text-base text-fg" }, icon)
  )
end

local function vol_icon(vol, muted)
  if muted then return I.vol_mute end
  if vol > 50 then return I.vol_hi end
  if vol > 0  then return I.vol_lo end
  return I.vol_off
end

local function bat_icon(pct, state)
  if state == "Charging" then return I.bat_chrg end
  if pct > 60 then return I.bat_full end
  if pct > 20 then return I.bat_half end
  return I.bat_low
end

-- ============================================================================
-- View
-- ============================================================================

function view(state)
  local surfaces = {}

  -- === Bar ===
  table.insert(surfaces, window("bar", {
    anchor = "top", height = 40, exclusive = true, monitor = "all",
  },
    row({ style = "bg-base w-full h-full px-2 items-center gap-1" },

      -- Left: workspaces
      each(state.workspaces, "id", function(ws)
        return button({
          on_click = msg("ws_focus", ws.id),
          style = ws.active
            and "p-1 rounded-full bg-primary"
            or  "p-1 rounded-full hover:bg-surface",
        },
          text({ style = ws.active and "text-xs text-base" or "text-xs text-muted" },
            tostring(ws.idx))
        )
      end),

      spacer(),

      -- Center: clock
      button({ on_click = msg("toggle", "calendar"), style = "px-2 py-1 rounded hover:bg-surface" },
        text({ style = "text-sm text-fg font-bold" }, state.time)
      ),

      spacer(),

      -- Right: status icons
      icon_btn(I.bright, msg("toggle", "bright")),
      icon_btn(bat_icon(state.bat_pct, state.bat_state), msg("toggle", "battery")),
      icon_btn(vol_icon(state.vol, state.muted), msg("toggle", "audio")),
      icon_btn(I.power, msg("toggle", "power"))
    )
  ))

  -- === Audio Popup ===
  if state.popup == "audio" then
    table.insert(surfaces, popup("audio", {
      anchor = "top right", width = 260, height = 120,
      dismiss_on_outside = true,
    },
      col({ style = "bg-surface w-full h-full rounded-lg p-4 gap-3" },
        row({ style = "items-center gap-2" },
          text({ style = "text-base text-muted" }, vol_icon(state.vol, state.muted)),
          text({ style = "text-sm text-fg font-bold" }, "Volume"),
          spacer(),
          text({ style = "text-xs text-muted" }, state.vol .. "%")
        ),
        slider({ value = state.vol, min = 0, max = 100, on_change = msg("set_vol") })
      )
    ))
  end

  -- === Power Popup ===
  if state.popup == "power" then
    table.insert(surfaces, popup("power", {
      anchor = "top right", width = 200, height = 220,
      dismiss_on_outside = true,
    },
      col({ style = "bg-surface w-full h-full rounded-lg p-3 gap-1" },
        row({ style = "items-center gap-2 px-2 py-1" },
          text({ style = "text-sm text-fg font-bold" }, state.user .. "@" .. state.host)
        ),
        -- Divider
        row({ style = "w-full h-1 bg-base rounded" }),
        -- Actions
        button({ on_click = msg("lock"), style = "w-full px-2 py-2 rounded hover:bg-overlay items-center gap-3" },
          text({ style = "text-sm text-fg" }, I.lock),
          text({ style = "text-sm text-fg" }, "Lock")
        ),
        button({ on_click = msg("logout"), style = "w-full px-2 py-2 rounded hover:bg-overlay items-center gap-3" },
          text({ style = "text-sm text-fg" }, I.logout),
          text({ style = "text-sm text-fg" }, "Log Out")
        ),
        button({ on_click = msg("suspend"), style = "w-full px-2 py-2 rounded hover:bg-overlay items-center gap-3" },
          text({ style = "text-sm text-fg" }, I.suspend),
          text({ style = "text-sm text-fg" }, "Suspend")
        ),
        button({ on_click = msg("reboot"), style = "w-full px-2 py-2 rounded hover:bg-overlay items-center gap-3" },
          text({ style = "text-sm text-fg" }, I.reboot),
          text({ style = "text-sm text-fg" }, "Restart")
        ),
        button({ on_click = msg("shutdown"), style = "w-full px-2 py-2 rounded hover:bg-overlay items-center gap-3" },
          text({ style = "text-sm text-error" }, I.shut),
          text({ style = "text-sm text-error" }, "Shut Down")
        )
      )
    ))
  end

  -- === Calendar Popup ===
  if state.popup == "calendar" then
    table.insert(surfaces, popup("calendar", {
      anchor = "top", width = 240, height = 80,
      dismiss_on_outside = true,
    },
      col({ style = "bg-surface w-full h-full rounded-lg p-4 gap-2 items-center" },
        text({ style = "text-lg text-fg font-bold" }, state.time),
        text({ style = "text-sm text-muted" }, state.date)
      )
    ))
  end

  return surfaces
end

-- ============================================================================
-- Subscriptions
-- ============================================================================

function subscribe(state)
  return {
    interval(1000, "tick"),
    exec("whoami", "user"),
    exec("hostname", "host"),
    exec("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null", "audio_info"),
    exec("cat /sys/class/power_supply/BAT0/capacity 2>/dev/null || echo 100", "bat_info"),
    exec("cat /sys/class/power_supply/BAT0/status 2>/dev/null || echo Unknown", "bat_state_info"),
    exec("brightnessctl g 2>/dev/null || echo 50", "bright_info"),
    exec("niri msg -j workspaces 2>/dev/null", "ws_info"),
    ipc("ipc_cmd"),
  }
end
