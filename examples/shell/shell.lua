-- ╔══════════════════════════════════════════════════╗
-- ║  Pulpkit v3 Shell — Niri desktop bar            ║
-- ╚══════════════════════════════════════════════════╝

local I = {
  circle   = "\u{f111}",
  circle_o = "\u{f10c}",
  clock    = "\u{f017}",
  wifi     = "\u{f1eb}",
  wifi_off = "\u{f6ac}",
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
  check    = "\u{f00c}",
  signal   = "\u{f012}",
}

-- ============================================================================
-- State
-- ============================================================================

function init()
  return {
    time       = os.date("%H:%M"),
    user       = "...",
    host       = "...",
    vol        = 0,
    muted      = false,
    bat_pct    = 0,
    bat_state  = "unknown",
    bright     = 50,
    workspaces = {},
    wifi_ssid  = "",
    wifi_list  = {},
    cpu        = 0,
    ram_used   = 0,
    ram_total  = 0,
    popup      = nil,
  }
end

-- ============================================================================
-- Update
-- ============================================================================

function update(state, msg)
  local t = msg.type

  if t == "tick" then
    state.time = os.date("%H:%M")
    -- Poll CPU/RAM on each tick via Lua io
    -- CPU: read /proc/stat for instant measurement (no top overhead)
    local cpu_f = io.popen("awk '/^cpu /{u=$2+$4; t=$2+$4+$5; print int(u*100/t)}' /proc/stat")
    if cpu_f then
      local val = cpu_f:read("*l")
      if val then state.cpu = tonumber(val) or 0 end
      cpu_f:close()
    end
    -- RAM: show in GB with one decimal
    local ram_f = io.popen("free -m | awk '/Mem:/{printf \"%.1f %.1f\", $3/1024, $2/1024}'")
    if ram_f then
      local line = ram_f:read("*l")
      if line then
        local used, total = line:match("(%S+)%s+(%S+)")
        if used then
          state.ram_used = used
          state.ram_total = total
        end
      end
      ram_f:close()
    end
  elseif t == "user" then state.user = msg.data or "?"
  elseif t == "host" then state.host = msg.data or "?"

  -- Audio (wpctl returns "Volume: 0.42" or "Volume: 0.42 [MUTED]")
  elseif t == "audio_info" then
    if msg.data then
      local vol = msg.data:match("Volume:%s*(%d+%.?%d*)")
      if vol then state.vol = math.floor(tonumber(vol) * 100) end
      state.muted = msg.data:find("%[MUTED%]") ~= nil
    end
  elseif t == "set_vol" then
    state.vol = math.floor(msg.data or state.vol)
    os.execute("wpctl set-volume @DEFAULT_AUDIO_SINK@ " .. string.format("%.2f", state.vol / 100))

  -- Battery
  elseif t == "bat_info" then
    if msg.data then
      local pct = msg.data:match("(%d+)")
      if pct then state.bat_pct = tonumber(pct) end
    end
  elseif t == "bat_state_info" then
    if msg.data then state.bat_state = msg.data:match("%S+") or "unknown" end

  -- Brightness
  elseif t == "bright_info" then
    if msg.data then
      local pct = msg.data:match("(%d+)")
      if pct then state.bright = tonumber(pct) end
    end
  elseif t == "set_bright" then
    local pct = math.floor(msg.data or state.bright)
    state.bright = pct
    os.execute("brightnessctl set " .. pct .. "% &")

  -- Workspaces
  elseif t == "ws_info" then
    if msg.data then
      local ws = {}
      for id, idx, focused in msg.data:gmatch('"id":(%d+).-"idx":(%d+).-"is_focused":(.-)[,}]') do
        table.insert(ws, {
          id = tostring(id), idx = tonumber(idx),
          active = focused:find("true") ~= nil,
        })
      end
      if #ws > 0 then state.workspaces = ws end
    end
  elseif t == "ws_focus" then
    if msg.data then
      os.execute("niri msg action focus-workspace " .. msg.data .. " &")
    end

  -- WiFi
  elseif t == "wifi_info" then
    if msg.data then
      local ssid = msg.data:match("^(.-):")
      if ssid and ssid ~= "" then state.wifi_ssid = ssid
      else state.wifi_ssid = "" end
    end
  elseif t == "wifi_connect" then
    if msg.data then
      os.execute("nmcli dev wifi connect '" .. msg.data .. "' &")
    end
  elseif t == "wifi_disconnect" then
    os.execute("nmcli dev disconnect wlan0 &")

  -- Popups
  elseif t == "toggle" then
    local name = msg.data
    if type(name) == "string" then
      state.popup = state.popup == name and nil or name
      -- Scan wifi when opening wifi popup
      if state.popup == "wifi" then
        local f = io.popen("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null")
        if f then
          local raw = f:read("*a")
          f:close()
          local list = {}
          local seen = {}
          for line in raw:gmatch("[^\n]+") do
            local ssid, sig, sec, active = line:match("^(.-):(%d+):(.-):(.-)$")
            if ssid and ssid ~= "" and not seen[ssid] then
              seen[ssid] = true
              table.insert(list, {
                id = ssid, ssid = ssid,
                signal = tonumber(sig) or 0,
                secure = sec ~= "",
                active = active == "yes",
              })
            end
          end
          table.sort(list, function(a, b)
            if a.active ~= b.active then return a.active end
            return a.signal > b.signal
          end)
          state.wifi_list = list
        end
      end
    end
  elseif t == "dismiss" then
    state.popup = nil

  -- Power
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

local function icon_btn(icon, action)
  return button({ on_click = action, style = "p-2 rounded-full hover:bg-surface" },
    text({ style = "text-base text-fg" }, icon)
  )
end

local function vol_icon(vol, muted)
  if muted then return I.vol_mute end
  if vol > 50 then return I.vol_hi end
  if vol > 0  then return I.vol_lo end
  return I.vol_off
end

local function bat_icon(pct, st)
  if st == "Charging" then return I.bat_chrg end
  if pct > 60 then return I.bat_full end
  if pct > 20 then return I.bat_half end
  return I.bat_low
end

local function menu_btn(icon, label, action, style_extra)
  return button({ on_click = action, style = "px-3 py-2 rounded hover:bg-overlay items-center gap-3 " .. (style_extra or "") },
    text({ style = "text-sm text-fg" }, icon),
    text({ style = "text-sm text-fg" }, label)
  )
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
          on_click = msg("ws_focus", tostring(ws.idx)),
          style = ws.active and "p-1 rounded-full bg-primary" or "p-1 rounded-full hover:bg-surface",
        },
          text({ style = ws.active and "text-xs text-base" or "text-xs text-muted" }, tostring(ws.idx))
        )
      end),

      spacer(),
      text({ style = "text-sm text-fg font-bold" }, state.time),
      spacer(),

      -- Resource monitor
      text({ style = "text-xs text-muted" }, "CPU " .. state.cpu .. "%"),
      text({ style = "text-xs text-muted" }, "RAM " .. state.ram_used .. "/" .. state.ram_total .. "G"),

      -- Right: status icons
      icon_btn(I.wifi, msg("toggle", "wifi")),
      icon_btn(I.bright, msg("toggle", "bright")),
      icon_btn(bat_icon(state.bat_pct, state.bat_state), msg("toggle", "battery")),
      icon_btn(vol_icon(state.vol, state.muted), msg("toggle", "audio")),
      icon_btn(I.power, msg("toggle", "power"))
    )
  ))

  -- === Audio Popup ===
  if state.popup == "audio" then
    table.insert(surfaces, popup("audio", {
      anchor = "top right", width = 260, height = 120, dismiss_on_outside = true,
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

  -- === Brightness Popup ===
  if state.popup == "bright" then
    table.insert(surfaces, popup("bright", {
      anchor = "top right", width = 260, height = 120, dismiss_on_outside = true,
    },
      col({ style = "bg-surface w-full h-full rounded-lg p-4 gap-3" },
        row({ style = "items-center gap-2" },
          text({ style = "text-base text-muted" }, I.bright),
          text({ style = "text-sm text-fg font-bold" }, "Brightness"),
          spacer(),
          text({ style = "text-xs text-muted" }, state.bright .. "%")
        ),
        slider({ value = state.bright, min = 0, max = 100, on_change = msg("set_bright") })
      )
    ))
  end

  -- === Battery Popup ===
  if state.popup == "battery" then
    table.insert(surfaces, popup("battery", {
      anchor = "top right", width = 220, height = 100, dismiss_on_outside = true,
    },
      col({ style = "bg-surface w-full h-full rounded-lg p-4 gap-2" },
        row({ style = "items-center gap-2" },
          text({ style = "text-lg text-fg" }, bat_icon(state.bat_pct, state.bat_state)),
          text({ style = "text-sm text-fg font-bold" }, "Battery")
        ),
        row({ style = "items-center gap-3" },
          text({ style = "text-2xl text-fg font-bold" }, state.bat_pct .. "%"),
          text({ style = "text-sm text-muted" }, state.bat_state)
        )
      )
    ))
  end

  -- === WiFi Popup ===
  if state.popup == "wifi" then
    local wifi_children = {
      row({ style = "items-center gap-2 px-1" },
        text({ style = "text-base text-muted" }, I.wifi),
        text({ style = "text-sm text-fg font-bold" }, "WiFi"),
        spacer(),
        text({ style = "text-xs text-muted" }, state.wifi_ssid ~= "" and state.wifi_ssid or "disconnected")
      ),
      row({ style = "w-full h-1 bg-base rounded" }),
    }
    -- Network list (up to 8)
    local count = 0
    for _, net in ipairs(state.wifi_list) do
      if count >= 8 then break end
      count = count + 1
      local label = net.ssid
      if net.active then label = label .. "  " .. I.check end
      local action = net.active and msg("wifi_disconnect") or msg("wifi_connect", net.ssid)
      table.insert(wifi_children,
        button({ on_click = action, style = "px-2 py-1 rounded hover:bg-overlay items-center gap-2" },
          text({ style = "text-xs text-muted" }, I.signal),
          text({ style = "text-xs text-fg" }, label),
          spacer(),
          text({ style = "text-xs text-muted" }, net.signal .. "%")
        )
      )
    end

    table.insert(surfaces, popup("wifi", {
      anchor = "top right", width = 280, height = 40 + count * 28, dismiss_on_outside = true,
    },
      col({ style = "bg-surface w-full h-full rounded-lg p-3 gap-1" },
        unpack(wifi_children)
      )
    ))
  end

  -- === Power Popup ===
  if state.popup == "power" then
    table.insert(surfaces, popup("power", {
      anchor = "top right", width = 200, height = 220, dismiss_on_outside = true,
    },
      col({ style = "bg-surface w-full h-full rounded-lg p-3 gap-1" },
        row({ style = "items-center gap-2 px-2 py-1" },
          text({ style = "text-sm text-fg font-bold" }, state.user .. "@" .. state.host)
        ),
        row({ style = "w-full h-1 bg-base rounded" }),
        menu_btn(I.lock,    "Lock",      msg("lock")),
        menu_btn(I.logout,  "Log Out",   msg("logout")),
        menu_btn(I.suspend, "Suspend",   msg("suspend")),
        menu_btn(I.reboot,  "Restart",   msg("reboot")),
        menu_btn(I.shut,    "Shut Down", msg("shutdown"), "text-error")
      )
    ))
  end

  return surfaces
end

-- ============================================================================
-- Subscriptions
-- ============================================================================

function subscribe(state)
  local subs = {
    interval(1000, "tick"),
    exec("whoami", "user"),
    exec("hostname", "host"),
    exec("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null", "audio_info"),
    exec("cat /sys/class/power_supply/BAT0/capacity 2>/dev/null || echo 100", "bat_info"),
    exec("cat /sys/class/power_supply/BAT0/status 2>/dev/null || echo Unknown", "bat_state_info"),
    exec("brightnessctl -m 2>/dev/null | cut -d, -f4 | tr -d '%' || echo 50", "bright_info"),
    exec("niri msg -j workspaces 2>/dev/null", "ws_info"),
    exec("nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null | grep '^yes' | head -1", "wifi_info"),
    ipc("ipc_cmd"),
  }
  return subs
end
