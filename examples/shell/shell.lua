-- ╔══════════════════════════════════════════════════╗
-- ║  Pulpkit v3 Shell                               ║
-- ╚══════════════════════════════════════════════════╝

-- Nerd Font Material Design icons
local I = {
  vol_hi   = "󰕾",  vol_mid  = "󰖀",  vol_lo   = "󰕿",  vol_mute = "󰝟",
  bat_full = "󰁹",  bat_good = "󰂀",  bat_half = "󰁾",  bat_low  = "󰁻",
  bat_chrg = "󰂄",  bat_empty = "󰂎",
  wifi_on  = "󰤨",  wifi_off = "󰤭",
  bright   = "󰃟",
  power    = "󰐥",  lock     = "󰌾",  suspend  = "󰤄",
  logout   = "󰗼",  reboot   = "󰜉",
  cpu      = "󰍛",  ram      = "󰘚",
  check    = "󰄬",  signal   = "󰤥",
}

-- ============================================================================
-- System polling helpers
-- ============================================================================

local function rcmd(cmd)
  local f = io.popen(cmd); if not f then return nil end
  local o = f:read("*l"); f:close(); return o
end

local function rfile(path)
  local f = io.open(path,"r"); if not f then return nil end
  local o = f:read("*l"); f:close(); return o
end

local has_bat = rfile("/sys/class/power_supply/BAT0/capacity") ~= nil

local function poll_vol()
  local r = rcmd("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null")
  if not r then return 0, false end
  local v = r:match("Volume:%s*(%d+%.?%d*)")
  return v and math.floor(tonumber(v)*100) or 0, r:find("%[MUTED%]") ~= nil
end

local function poll_bat()
  local c = rfile("/sys/class/power_supply/BAT0/capacity")
  local s = rfile("/sys/class/power_supply/BAT0/status")
  return tonumber(c) or 100, (s or "Unknown"):match("%S+") or "Unknown"
end

local function poll_bri()
  local r = rcmd("brightnessctl -m 2>/dev/null")
  if not r then return 50 end
  return tonumber(r:match(",(%d+)%%")) or 50
end

local function poll_cpu()
  local f = io.open("/proc/stat","r")
  if not f then return 0 end
  local line = f:read("*l"); f:close()
  if not line then return 0 end
  local user,nice,sys,idle = line:match("^cpu%s+(%d+)%s+(%d+)%s+(%d+)%s+(%d+)")
  if not user then return 0 end
  local busy = tonumber(user)+tonumber(sys)
  local total = busy+tonumber(nice)+tonumber(idle)
  if total == 0 then return 0 end
  return math.floor(busy*100/total)
end

local function poll_ram()
  local f = io.open("/proc/meminfo","r")
  if not f then return "0","0" end
  local total, avail
  for line in f:lines() do
    if line:find("^MemTotal:") then total = tonumber(line:match("(%d+)")) end
    if line:find("^MemAvailable:") then avail = tonumber(line:match("(%d+)")) end
    if total and avail then break end
  end
  f:close()
  if not total then return "0","0" end
  local used = total - (avail or 0)
  return string.format("%.1f", used/1048576), string.format("%.1f", total/1048576)
end

local function poll_ws()
  local r = rcmd("niri msg -j workspaces 2>/dev/null")
  if not r then return {} end
  local l = {}
  for id,idx,foc in r:gmatch('"id":(%d+).-"idx":(%d+).-"is_focused":(.-)[,}]') do
    l[#l+1] = { id=tostring(id), idx=tonumber(idx), active=foc:find("true")~=nil }
  end
  return l
end

local function poll_wifi()
  local r = rcmd("nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null | grep '^yes' | head -1")
  if not r then return "" end
  return r:match("^yes:(.+)") or ""
end

local function scan_wifi()
  local f = io.popen("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null")
  if not f then return {} end
  local nets, seen = {}, {}
  for line in f:lines() do
    local ssid,sig,sec,act = line:match("^(.-):(%d+):(.-):(.-)$")
    if ssid and ssid~="" and not seen[ssid] then
      seen[ssid]=true
      nets[#nets+1]={id=ssid,ssid=ssid,signal=tonumber(sig)or 0,active=act=="yes"}
    end
  end
  f:close()
  table.sort(nets,function(a,b) if a.active~=b.active then return a.active end; return a.signal>b.signal end)
  return nets
end

-- ============================================================================
-- State
-- ============================================================================

function init()
  local v, m = poll_vol()
  local b, bs = poll_bat()
  local ru, rt = poll_ram()
  return {
    time=os.date("%H:%M"), user=rcmd("whoami")or"", host=rcmd("hostname")or"",
    vol=v, muted=m, bat=b, bat_st=bs, bright=poll_bri(),
    cpu=poll_cpu(), ram_u=ru, ram_t=rt,
    ws=poll_ws(), wifi=poll_wifi(), wifi_nets={},
    popup=nil, tick_n=0,
  }
end

-- ============================================================================
-- Update
-- ============================================================================

function update(state, msg)
  local t = msg.type

  if t == "niri_event" then
    -- Instant workspace updates from niri event stream
    if msg.data and msg.data:find("^Workspaces changed:") then
      state.ws = poll_ws()  -- re-read full workspace state
    end
    return state
  end

  if t == "tick" then
    state.tick_n = state.tick_n + 1
    state.time = os.date("%H:%M")
    -- Workspaces: skip polling since niri event stream handles this
    state.cpu = poll_cpu()       -- cpu: every tick
    if state.tick_n % 3 == 0 then  -- every 3s: volume, ram, wifi, brightness
      state.vol, state.muted = poll_vol()
      state.ram_u, state.ram_t = poll_ram()
      state.wifi = poll_wifi()
      state.bright = poll_bri()
    end
    if state.tick_n % 10 == 0 then -- every 10s: battery
      state.bat, state.bat_st = poll_bat()
    end
    return state
  end

  if t == "set_vol" then
    state.vol = math.floor(msg.data or state.vol)
    os.execute(string.format("wpctl set-volume @DEFAULT_AUDIO_SINK@ %.2f &", state.vol/100))
    state.muted = false
  elseif t == "set_bri" then
    state.bright = math.floor(msg.data or state.bright)
    os.execute("brightnessctl set "..state.bright.."% &")
  elseif t == "ws_go" then
    if msg.data then os.execute("niri msg action focus-workspace "..msg.data.." &") end
  elseif t == "wifi_con" then
    if msg.data then os.execute("nmcli dev wifi connect '"..msg.data.."' &") end
  elseif t == "wifi_dis" then
    os.execute("nmcli dev disconnect wlan0 &")
  elseif t == "toggle" then
    local name = msg.data
    if type(name)=="string" then
      if state.popup==name then state.popup=nil
      else
        state.popup=name
        if name=="wifi" then state.wifi_nets=scan_wifi() end
        -- Refresh relevant data when opening popup
        if name=="audio" then state.vol,state.muted=poll_vol() end
        if name=="bright" then state.bright=poll_bri() end
        if name=="bat" then state.bat,state.bat_st=poll_bat() end
      end
    end
  elseif t == "dismiss" then state.popup=nil
  elseif t == "lock"     then os.execute("loginctl lock-session &"); state.popup=nil
  elseif t == "logout"   then os.execute("niri msg action quit &")
  elseif t == "suspend"  then os.execute("systemctl suspend &"); state.popup=nil
  elseif t == "reboot"   then os.execute("systemctl reboot &")
  elseif t == "shutdown" then os.execute("systemctl poweroff &")
  end
  return state
end

-- ============================================================================
-- View components
-- ============================================================================

local function ibtn(icon, action)
  return button({on_click=action, style="p-2 rounded-full hover:bg-surface"},
    text({style="text-lg text-fg"}, icon))
end

local function mbtn(icon, label, action, color)
  return button({on_click=action, style="px-3 py-2 rounded hover:bg-overlay items-center gap-3"},
    text({style="text-sm "..(color or "text-fg")}, icon),
    text({style="text-sm "..(color or "text-fg")}, label))
end

local function vol_i(v,m)
  if m then return I.vol_mute end
  if v>50 then return I.vol_hi elseif v>0 then return I.vol_mid end
  return I.vol_lo
end

local function bat_i(p,s)
  if s=="Charging" then return I.bat_chrg end
  if p>80 then return I.bat_full elseif p>60 then return I.bat_good
  elseif p>30 then return I.bat_half elseif p>10 then return I.bat_low end
  return I.bat_empty
end

local function slider_row(icon, label, value, pct_text, on_change)
  return col({style="bg-surface w-full h-full rounded-lg p-4 gap-3"},
    row({style="items-center gap-2"},
      text({style="text-lg text-muted"}, icon),
      text({style="text-sm text-fg font-bold"}, label),
      spacer(),
      text({style="text-xs text-muted"}, pct_text)),
    slider({value=value, min=0, max=100, on_change=on_change}))
end

-- ============================================================================
-- View
-- ============================================================================

function view(state)
  local S = {}

  -- ── Bar ────────────────────────────────────────
  S[#S+1] = window("bar", {anchor="top", height=40, exclusive=true, monitor="all"},
    row({style="bg-base w-full h-full px-2 items-center gap-1"},

      -- Workspaces
      each(state.ws, "id", function(ws)
        local active = ws.active
        return button({on_click=msg("ws_go",tostring(ws.idx)),
          style=active and "px-2 py-1 rounded bg-primary" or "px-2 py-1 rounded hover:bg-surface"},
          text({style=active and "text-sm text-base" or "text-sm text-muted"}, tostring(ws.idx)))
      end),

      spacer(),
      text({style="text-sm text-fg font-bold"}, state.time),
      spacer(),

      -- Stats
      row({style="items-center gap-3 px-2"},
        text({style="text-sm text-muted"}, I.cpu.." "..state.cpu.."%"),
        text({style="text-sm text-muted"}, I.ram.." "..state.ram_u.."/"..state.ram_t.."G")),

      -- Status bar icons
      ibtn(state.wifi~="" and I.wifi_on or I.wifi_off, msg("toggle","wifi")),
      ibtn(I.bright, msg("toggle","bright")),
      has_bat and ibtn(bat_i(state.bat,state.bat_st), msg("toggle","bat")) or spacer(),
      ibtn(vol_i(state.vol,state.muted), msg("toggle","audio")),
      ibtn(I.power, msg("toggle","power"))))

  -- ── Audio popup ────────────────────────────────
  if state.popup=="audio" then
    S[#S+1] = popup("audio", {anchor="top right",width=260,height=120,dismiss_on_outside=true},
      slider_row(vol_i(state.vol,state.muted), "Volume", state.vol, state.vol.."%", msg("set_vol")))
  end

  -- ── Brightness popup ───────────────────────────
  if state.popup=="bright" then
    S[#S+1] = popup("bright", {anchor="top right",width=260,height=120,dismiss_on_outside=true},
      slider_row(I.bright, "Brightness", state.bright, state.bright.."%", msg("set_bri")))
  end

  -- ── Battery popup ──────────────────────────────
  if state.popup=="bat" then
    S[#S+1] = popup("bat", {anchor="top right",width=200,height=90,dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-2"},
        row({style="items-center gap-2"},
          text({style="text-lg text-fg"}, bat_i(state.bat,state.bat_st)),
          text({style="text-sm text-fg font-bold"}, "Battery")),
        row({style="items-center gap-3"},
          text({style="text-xl text-fg font-bold"}, state.bat.."%"),
          text({style="text-sm text-muted"}, state.bat_st))))
  end

  -- ── WiFi popup ─────────────────────────────────
  if state.popup=="wifi" then
    local ch = {
      row({style="items-center gap-2 px-1"},
        text({style="text-lg text-muted"}, I.wifi_on),
        text({style="text-sm text-fg font-bold"}, "WiFi"),
        spacer(),
        text({style="text-xs text-muted"}, state.wifi~="" and state.wifi or "disconnected")),
      row({style="w-full h-1 bg-base rounded"}),
    }
    local n=0
    for _,net in ipairs(state.wifi_nets) do
      if n>=8 then break end; n=n+1
      ch[#ch+1] = button({
        on_click=net.active and msg("wifi_dis") or msg("wifi_con",net.ssid),
        style="px-2 py-1 rounded hover:bg-overlay items-center gap-2"},
        text({style="text-xs text-muted"}, I.signal),
        text({style="text-xs text-fg"}, net.ssid..(net.active and (" "..I.check) or "")),
        spacer(),
        text({style="text-xs text-muted"}, net.signal.."%"))
    end
    S[#S+1] = popup("wifi", {anchor="top right",width=280,height=50+n*28,dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-3 gap-1"}, unpack(ch)))
  end

  -- ── Power popup ────────────────────────────────
  if state.popup=="power" then
    S[#S+1] = popup("power", {anchor="top right",width=200,height=230,dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-3 gap-1"},
        row({style="items-center gap-2 px-2 py-1"},
          text({style="text-sm text-fg font-bold"}, state.user.."@"..state.host)),
        row({style="w-full h-1 bg-base rounded"}),
        mbtn(I.lock,   "Lock",     msg("lock")),
        mbtn(I.logout, "Log Out",  msg("logout")),
        mbtn(I.suspend,"Suspend",  msg("suspend")),
        mbtn(I.reboot, "Restart",  msg("reboot")),
        mbtn(I.power,  "Shut Down",msg("shutdown"),"text-error")))
  end

  return S
end

-- ============================================================================
-- Subscriptions
-- ============================================================================

function subscribe(state)
  return {
    interval(1000, "tick"),
    stream("niri msg event-stream 2>/dev/null", "niri_event"),
    ipc("ipc"),
  }
end
