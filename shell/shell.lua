-- Pulpkit Default Shell

local I = {
  vol_hi="󰕾", vol_mid="󰖀", vol_lo="󰕿", vol_mute="󰝟",
  bat_full="󰁹", bat_good="󰂀", bat_half="󰁾", bat_low="󰁻", bat_chrg="󰂄", bat_empty="󰂎",
  wifi_4="󰤨", wifi_3="󰤥", wifi_2="󰤢", wifi_1="󰤟", wifi_off="󰤭",
  bright="󰃟",
  power="󰐥", lock="󰌾", suspend="󰤄", logout="󰗼", reboot="󰜉", shutdown="󰐦",
  search="󰍉", settings="󰒓", check="󰄬",
  night="󰌵", night_off="󰌶",
  dnd="󰍶", dnd_off="󰍷",
  bt="󰂯", bt_off="󰂲",
}

-- ── System polling ─────────────────────────────────

local function rc(c)
  local f = io.popen(c); if not f then return nil end
  local o = f:read("*l"); f:close(); return o
end

local function rf(p)
  local f = io.open(p, "r"); if not f then return nil end
  local o = f:read("*l"); f:close(); return o
end

local has_bat = rf("/sys/class/power_supply/BAT0/capacity") ~= nil

local function poll_vol()
  local r = rc("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null")
  if not r then return 0, false end
  local v = r:match("Volume:%s*(%d+%.?%d*)")
  return v and math.floor(tonumber(v) * 100) or 0, r:find("%[MUTED%]") ~= nil
end

local function poll_bat()
  return tonumber(rf("/sys/class/power_supply/BAT0/capacity")) or 100,
    (rf("/sys/class/power_supply/BAT0/status") or "Unknown"):match("%S+") or "Unknown"
end

local function poll_bri()
  local r = rc("brightnessctl -m 2>/dev/null")
  return r and tonumber(r:match(",(%d+)%%")) or 50
end

local function poll_ws()
  local r = rc("niri msg -j workspaces 2>/dev/null")
  if not r then return {} end
  local l = {}
  for id, idx, foc in r:gmatch('"id":(%d+).-"idx":(%d+).-"is_focused":(.-)[,}]') do
    l[#l+1] = {id=tostring(id), idx=tonumber(idx), active=foc:find("true")~=nil}
  end
  table.sort(l, function(a, b) return a.idx < b.idx end)
  return l
end

local function poll_wifi()
  local r = rc("nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null|grep '^yes'|head -1")
  return r and r:match("^yes:(.+)") or ""
end

local function poll_profile()
  local r = rc("powerprofilesctl get 2>/dev/null")
  return r and r:match("%S+") or "balanced"
end

local function scan_wifi()
  local f = io.popen("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null")
  if not f then return {} end
  local n, s = {}, {}
  for l in f:lines() do
    local ss, sg, sc, a = l:match("^(.-):(%d+):(.-):(.-)$")
    if ss and ss ~= "" and not s[ss] then
      s[ss] = true
      n[#n+1] = {id=ss, ssid=ss, signal=tonumber(sg) or 0, secure=sc~="", active=a=="yes"}
    end
  end
  f:close()
  table.sort(n, function(a, b)
    if a.active ~= b.active then return a.active end
    return a.signal > b.signal
  end)
  return n
end

local function scan_apps()
  local apps = {}
  local dirs = {"/usr/share/applications", os.getenv("HOME").."/.local/share/applications"}
  for _, dir in ipairs(dirs) do
    local f = io.popen("ls "..dir.."/*.desktop 2>/dev/null")
    if f then
      for path in f:lines() do
        local df = io.open(path, "r")
        if df then
          local name, exec, nodisplay = nil, nil, false
          for line in df:lines() do
            if line:match("^%[") and line ~= "[Desktop Entry]" then break end
            local k, v = line:match("^(%w+)=(.+)")
            if k == "Name" and not name then name = v
            elseif k == "Exec" then exec = v:gsub(" %%[fFuUdDnNickvm]", "")
            elseif k == "NoDisplay" and v == "true" then nodisplay = true end
          end
          df:close()
          if name and exec and not nodisplay then
            apps[#apps+1] = {id=name, name=name, exec=exec}
          end
        end
      end
      f:close()
    end
  end
  table.sort(apps, function(a, b) return a.name:lower() < b.name:lower() end)
  return apps
end

-- ── Icon helpers ───────────────────────────────────

local function voli(v, m)
  if m then return I.vol_mute end
  if v > 50 then return I.vol_hi elseif v > 0 then return I.vol_mid end
  return I.vol_lo
end

local function bati(p, s)
  if s == "Charging" then return I.bat_chrg end
  if p > 80 then return I.bat_full elseif p > 60 then return I.bat_good
  elseif p > 30 then return I.bat_half elseif p > 10 then return I.bat_low end
  return I.bat_empty
end

local function wsig(s)
  if s > 75 then return I.wifi_4 elseif s > 50 then return I.wifi_3
  elseif s > 25 then return I.wifi_2 end
  return I.wifi_1
end

-- ── State ──────────────────────────────────────────

function init()
  local v, m = poll_vol()
  local b, bs = poll_bat()
  return {
    time = os.date("%H:%M"),
    vol = v, muted = m,
    bright = poll_bri(),
    bat = b, bat_st = bs,
    ws = poll_ws(),
    wifi = poll_wifi(),
    wifi_nets = {},
    night = false, dnd = false, bt = false,
    power_profile = poll_profile(),
    user = rc("whoami") or "",
    host = rc("hostname") or "",
    kernel = rc("uname -r") or "",
    uptime = (rc("uptime -p 2>/dev/null") or ""):gsub("^up ", ""),
    popup = nil,
    apps = {}, search = "",
    osd = nil, osd_val = 0, osd_timer = 0,
    tick_n = 0,
  }
end

-- ── Update ─────────────────────────────────────────

function update(state, msg)
  local t = msg.type

  -- Tick: polling cadence
  if t == "tick" then
    state.tick_n = state.tick_n + 1
    state.time = os.date("%H:%M")
    if state.tick_n % 3 == 0 then
      state.vol, state.muted = poll_vol()
      state.bright = poll_bri()
      state.wifi = poll_wifi()
    end
    if state.tick_n % 10 == 0 and has_bat then
      state.bat, state.bat_st = poll_bat()
    end
    if state.tick_n % 30 == 0 then
      state.uptime = (rc("uptime -p 2>/dev/null") or ""):gsub("^up ", "")
    end
    if state.osd then
      state.osd_timer = state.osd_timer - 1
      if state.osd_timer <= 0 then state.osd = nil end
    end
    return state
  end

  -- Niri workspace events
  if t == "niri_event" then
    if msg.data and msg.data:find("Workspaces changed:") then
      state.ws = poll_ws()
    end
    return state
  end

  -- IPC commands
  if t == "ipc" then
    local cmd = msg.data
    if cmd == "vol-up" then
      state.vol = math.min(100, state.vol + 5); state.muted = false
      os.execute(string.format("wpctl set-volume @DEFAULT_AUDIO_SINK@ %.2f &", state.vol / 100))
      state.osd = "vol"; state.osd_val = state.vol; state.osd_timer = 2
    elseif cmd == "vol-down" then
      state.vol = math.max(0, state.vol - 5); state.muted = false
      os.execute(string.format("wpctl set-volume @DEFAULT_AUDIO_SINK@ %.2f &", state.vol / 100))
      state.osd = "vol"; state.osd_val = state.vol; state.osd_timer = 2
    elseif cmd == "vol-mute" then
      state.muted = not state.muted
      os.execute("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle &")
      state.osd = "vol"; state.osd_val = state.vol; state.osd_timer = 2
    elseif cmd == "bri-up" then
      state.bright = math.min(100, state.bright + 5)
      os.execute("brightnessctl set " .. state.bright .. "% &")
      state.osd = "bri"; state.osd_val = state.bright; state.osd_timer = 2
    elseif cmd == "bri-down" then
      state.bright = math.max(0, state.bright - 5)
      os.execute("brightnessctl set " .. state.bright .. "% &")
      state.osd = "bri"; state.osd_val = state.bright; state.osd_timer = 2
    elseif cmd == "toggle-launcher" then
      if state.popup == "launcher" then state.popup = nil; state.search = ""
      else state.popup = "launcher"; state.search = ""; state.apps = scan_apps() end
    elseif cmd == "toggle-settings" then
      if state.popup == "settings" then state.popup = nil else state.popup = "settings" end
    elseif cmd == "toggle-power" then
      if state.popup == "power" then state.popup = nil else state.popup = "power" end
    elseif cmd == "toggle-wifi" then
      if state.popup == "wifi" then state.popup = nil
      else state.popup = "wifi"; state.wifi_nets = scan_wifi() end
    elseif cmd == "dismiss" then
      state.popup = nil; state.search = ""
    end
    return state
  end

  -- Popup toggles
  if t == "toggle" then
    local n = msg.data
    if type(n) == "string" then
      if state.popup == n then
        state.popup = nil; state.search = ""
      else
        state.popup = n; state.search = ""
        if n == "wifi" then state.wifi_nets = scan_wifi() end
        if n == "launcher" then state.apps = scan_apps() end
      end
    end
    return state
  end

  -- Workspace
  if t == "ws_go" then
    if msg.data then os.execute("niri msg action focus-workspace " .. msg.data .. " &") end

  -- Audio
  elseif t == "set_vol" then
    state.vol = math.floor(msg.data or state.vol); state.muted = false
    os.execute(string.format("wpctl set-volume @DEFAULT_AUDIO_SINK@ %.2f &", state.vol / 100))
  elseif t == "toggle_mute" then
    state.muted = not state.muted
    os.execute("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle &")

  -- Brightness
  elseif t == "set_bri" then
    state.bright = math.floor(msg.data or state.bright)
    os.execute("brightnessctl set " .. state.bright .. "% &")

  -- Toggles
  elseif t == "toggle_night" then
    state.night = not state.night
    os.execute(state.night and "wlsunset -T 4500 -t 3500 &" or "pkill wlsunset &")
  elseif t == "toggle_dnd" then
    state.dnd = not state.dnd
  elseif t == "toggle_bt" then
    state.bt = not state.bt
    os.execute(state.bt and "bluetoothctl power on &" or "bluetoothctl power off &")
  elseif t == "toggle_wifi_radio" then
    if state.wifi ~= "" then
      os.execute("nmcli radio wifi off &"); state.wifi = ""
    else
      os.execute("nmcli radio wifi on &")
    end

  -- Power profile
  elseif t == "set_profile" then
    if msg.data then
      state.power_profile = msg.data
      os.execute("powerprofilesctl set " .. msg.data .. " &")
    end

  -- WiFi
  elseif t == "wifi_con" then
    if msg.data then os.execute("nmcli dev wifi connect '" .. msg.data .. "' &") end
  elseif t == "wifi_dis" then
    os.execute("nmcli dev disconnect wlan0 &")

  -- Launcher
  elseif t == "launch" then
    if msg.data then os.execute(msg.data .. " &") end
    state.popup = nil; state.search = ""
  elseif t == "search" then
    if msg.data then state.search = msg.data end
  elseif t == "key" then
    if state.popup == "launcher" and msg.data then
      local key = msg.data.key or ""
      local txt = msg.data.text or ""
      if key == "BackSpace" then
        state.search = state.search:sub(1, -2)
      elseif key == "Escape" then
        state.popup = nil; state.search = ""
      elseif key == "Return" then
        local q = state.search:lower()
        for _, app in ipairs(state.apps) do
          if q == "" or app.name:lower():find(q, 1, true) then
            os.execute(app.exec .. " &"); state.popup = nil; state.search = ""; break
          end
        end
      elseif #txt == 1 and txt:byte() >= 32 then
        state.search = state.search .. txt
      end
    elseif msg.data and (msg.data.key or "") == "Escape" and state.popup then
      state.popup = nil; state.search = ""
    end

  -- Power/session
  elseif t == "lock" then os.execute("loginctl lock-session &"); state.popup = nil
  elseif t == "logout" then os.execute("niri msg action quit &")
  elseif t == "suspend" then os.execute("systemctl suspend &"); state.popup = nil
  elseif t == "reboot" then os.execute("systemctl reboot &")
  elseif t == "shutdown" then os.execute("systemctl poweroff &")

  -- Dismiss
  elseif t == "dismiss" then state.popup = nil; state.search = ""
  end

  return state
end

-- ── View helpers ───────────────────────────────────

local function ibtn(ic, action)
  return button({on_click=action, style="px-2 py-1 rounded hover:bg-surface items-center justify-center"},
    text({style="text-lg text-fg"}, ic))
end

local function sep()
  return row({style="w-full bg-overlay rounded", height=1})
end

local function mitem(ic, label, action, color)
  color = color or "text-fg"
  return button({on_click=action, style="px-3 py-2 rounded hover:bg-overlay items-center gap-3"},
    text({style="text-lg "..color}, ic),
    text({style="text-base "..color}, label))
end

local function tile(ic, label, active, action)
  return button({on_click=action,
    style=(active and "bg-primary" or "bg-overlay").." rounded-lg px-2 py-3 items-center gap-1 flex-1"},
    text({style="text-lg "..(active and "text-#0d1017" or "text-fg")}, ic),
    text({style="text-xs "..(active and "text-#0d1017" or "text-muted")}, label))
end

local function pbtn(label, value, current)
  local active = value == current
  return button({on_click=msg("set_profile", value),
    style=(active and "bg-primary" or "bg-overlay").." rounded px-3 py-1 items-center justify-center flex-1"},
    text({style="text-xs "..(active and "text-#0d1017 font-bold" or "text-muted")}, label))
end

-- ── View ───────────────────────────────────────────

function view(state)
  local S = {}

  -- BAR
  S[#S+1] = window("bar", {anchor="top", height=32, exclusive=true, monitor="all"},
    row({style="bg-base w-full h-full px-3 items-center"},

      -- Left: launcher + workspaces
      row({style="flex-1 items-center gap-1"},
        ibtn(I.search, msg("toggle", "launcher")),
        each(state.ws, "id", function(ws)
          return button({on_click=msg("ws_go", tostring(ws.idx)),
            style=ws.active
              and "px-2 py-1 rounded bg-primary items-center justify-center"
              or  "px-2 py-1 rounded hover:bg-surface items-center justify-center"},
            text({style=ws.active and "text-sm text-#0d1017 font-bold" or "text-sm text-muted"},
              tostring(ws.idx)))
        end)),

      -- Center: clock
      text({style="text-base text-fg font-medium"}, state.time),

      -- Right: status + settings + power
      row({style="flex-1 items-center justify-end gap-1"},
        ibtn(state.wifi ~= "" and I.wifi_4 or I.wifi_off, msg("toggle", "wifi")),
        ibtn(voli(state.vol, state.muted), msg("toggle", "settings")),
        has_bat and ibtn(bati(state.bat, state.bat_st), msg("toggle", "settings")) or spacer(),
        ibtn(I.settings, msg("toggle", "settings")),
        ibtn(I.power, msg("toggle", "power")))))

  -- QUICK SETTINGS
  if state.popup == "settings" then
    S[#S+1] = popup("settings", {anchor="top right", width=280, height=340, dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-3"},
        text({style="text-sm text-fg font-bold"}, "Quick Settings"),

        -- Toggle grid
        row({style="gap-2"},
          tile(state.wifi ~= "" and I.wifi_4 or I.wifi_off, "WiFi", state.wifi ~= "", msg("toggle_wifi_radio")),
          tile(state.bt and I.bt or I.bt_off, "BT", state.bt, msg("toggle_bt")),
          tile(state.dnd and I.dnd or I.dnd_off, "DND", state.dnd, msg("toggle_dnd"))),
        row({style="gap-2"},
          tile(state.night and I.night or I.night_off, "Night", state.night, msg("toggle_night")),
          tile(voli(state.vol, state.muted), "Mute", state.muted, msg("toggle_mute"))),

        sep(),

        -- Volume
        row({style="items-center gap-2"},
          text({style="text-base text-muted"}, voli(state.vol, state.muted)),
          slider({value=state.vol, min=0, max=100, on_change=msg("set_vol"), style="flex-1"}),
          text({style="text-xs text-muted w-8"}, state.vol.."%")),

        -- Brightness
        row({style="items-center gap-2"},
          text({style="text-base text-muted"}, I.bright),
          slider({value=state.bright, min=0, max=100, on_change=msg("set_bri"), style="flex-1"}),
          text({style="text-xs text-muted w-8"}, state.bright.."%")),

        sep(),

        -- Power profile
        row({style="gap-2"},
          pbtn("󰾆 Saver", "power-saver", state.power_profile),
          pbtn("󰾅 Balanced", "balanced", state.power_profile),
          pbtn("󰓅 Perf", "performance", state.power_profile))))
  end

  -- WIFI
  if state.popup == "wifi" then
    local ch = {}
    ch[#ch+1] = row({style="items-center gap-3"},
      text({style="text-lg text-fg"}, I.wifi_4),
      text({style="text-sm text-fg font-bold"}, "Networks"),
      spacer(),
      text({style="text-xs text-muted"}, state.wifi ~= "" and state.wifi or "disconnected"))
    ch[#ch+1] = sep()

    local n = 0
    for _, net in ipairs(state.wifi_nets) do
      if n >= 8 then break end; n = n + 1
      local c = net.active and "text-primary" or "text-fg"
      ch[#ch+1] = button({
        on_click=net.active and msg("wifi_dis") or msg("wifi_con", net.ssid),
        style="px-2 py-1 rounded hover:bg-overlay items-center gap-2"},
        text({style="text-base "..c}, wsig(net.signal)),
        text({style="text-sm "..c..(net.active and " font-bold" or "")},
          net.ssid..(net.secure and " 󰌾" or "")),
        spacer(),
        net.active and text({style="text-xs text-primary"}, I.check)
          or text({style="text-xs text-muted"}, net.signal.."%"))
    end

    S[#S+1] = popup("wifi", {anchor="top right", width=300, height=52+n*28, dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-1"}, unpack(ch)))
  end

  -- POWER MENU
  if state.popup == "power" then
    S[#S+1] = popup("power", {anchor="top right", width=260, height=300, dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-2"},
        col({style="gap-1 items-center py-2"},
          text({style="text-base text-fg font-bold"}, state.user.."@"..state.host),
          text({style="text-xs text-muted"}, state.kernel),
          text({style="text-xs text-muted"}, "Up "..state.uptime)),
        sep(),
        mitem(I.lock,     "Lock Screen", msg("lock")),
        mitem(I.suspend,  "Suspend",     msg("suspend")),
        mitem(I.reboot,   "Restart",     msg("reboot")),
        mitem(I.shutdown, "Shut Down",   msg("shutdown")),
        sep(),
        mitem(I.logout,   "Log Out",     msg("logout"), "text-error")))
  end

  -- LAUNCHER
  if state.popup == "launcher" then
    local query = state.search:lower()
    local filtered = {}
    for _, app in ipairs(state.apps) do
      if query == "" or app.name:lower():find(query, 1, true) then
        filtered[#filtered+1] = app
      end
      if #filtered >= 12 then break end
    end

    local ch = {}
    ch[#ch+1] = row({style="items-center gap-2"},
      text({style="text-lg text-muted"}, I.search),
      text({style="text-sm text-fg font-bold"}, "Applications"))
    ch[#ch+1] = input({value=state.search, placeholder="Search...", on_input=msg("search"),
      style="bg-overlay rounded px-2 py-1 text-sm text-fg w-full"})
    ch[#ch+1] = sep()

    for _, app in ipairs(filtered) do
      ch[#ch+1] = button({on_click=msg("launch", app.exec),
        style="px-2 py-1 rounded hover:bg-overlay items-center gap-2"},
        text({style="text-sm text-fg"}, app.name))
    end

    local h = 80 + #filtered * 24
    if h > 400 then h = 400 end
    S[#S+1] = popup("launcher", {anchor="top left", width=300, height=h, dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-3 gap-2"}, unpack(ch)))
  end

  -- OSD
  if state.osd then
    local oi = state.osd == "vol" and voli(state.osd_val, state.muted) or I.bright
    S[#S+1] = popup("osd", {anchor="center", width=200, height=48, dismiss_on_outside=false},
      row({style="bg-surface w-full h-full rounded-lg px-4 items-center gap-3"},
        text({style="text-lg text-fg"}, oi),
        slider({value=state.osd_val, min=0, max=100, style="flex-1"}),
        text({style="text-xs text-muted w-8"}, state.osd_val.."%")))
  end

  return S
end

-- ── Subscriptions ──────────────────────────────────

function subscribe(state)
  return {
    interval(1000, "tick"),
    stream("niri msg event-stream 2>/dev/null", "niri_event"),
    ipc("ipc"),
  }
end
