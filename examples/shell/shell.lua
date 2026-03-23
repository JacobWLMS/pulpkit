-- ╔══════════════════════════════════════════════════╗
-- ║  Pulpkit v3 Shell                               ║
-- ╚══════════════════════════════════════════════════╝

-- Nerd Font icons (material design variants)
local I = {
  vol_hi   = "󰕾",  vol_mid  = "󰖀",  vol_lo   = "󰕿",  vol_mute = "󰝟",
  bat_full = "󰁹",  bat_good = "󰂀",  bat_half = "󰁾",  bat_low  = "󰁻",
  bat_chrg = "󰂄",  bat_empty = "󰂎",
  wifi_4   = "󰤨",  wifi_3   = "󰤥",  wifi_2   = "󰤢",  wifi_1   = "󰤟",
  wifi_off = "󰤭",
  bright   = "󰃟",
  power    = "󰐥",  lock     = "󰌾",  suspend  = "󰤄",
  logout   = "󰗼",  reboot   = "󰜉",  shutdown = "󰐦",
  search   = "󰍉",
  circle_f = "󰮍",  circle   = "󰊠",
  check    = "󰄬",  refresh  = "󰑓",
  cpu      = "󰍛",  ram      = "󰘚",
  settings = "󰒓",  night    = "󰌵",  night_off = "󰌶",
  dnd      = "󰍶",  dnd_off  = "󰍷",  bt       = "󰂯",  bt_off = "󰂲",
  display  = "󰍹",  audio_out = "󰓃",
}

-- ============================================================================
-- System polling
-- ============================================================================

local function rcmd(c) local f=io.popen(c); if not f then return nil end; local o=f:read("*l"); f:close(); return o end
local function rfile(p) local f=io.open(p,"r"); if not f then return nil end; local o=f:read("*l"); f:close(); return o end

local has_bat = rfile("/sys/class/power_supply/BAT0/capacity") ~= nil

local function poll_vol()
  local r=rcmd("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null")
  if not r then return 0,false end
  local v=r:match("Volume:%s*(%d+%.?%d*)")
  return v and math.floor(tonumber(v)*100) or 0, r:find("%[MUTED%]")~=nil
end

local function poll_bat()
  return tonumber(rfile("/sys/class/power_supply/BAT0/capacity")) or 100,
         (rfile("/sys/class/power_supply/BAT0/status") or "Unknown"):match("%S+") or "Unknown"
end

local function poll_bri()
  local r=rcmd("brightnessctl -m 2>/dev/null")
  return r and tonumber(r:match(",(%d+)%%")) or 50
end

local function poll_cpu()
  local f=io.open("/proc/stat","r"); if not f then return 0 end
  local l=f:read("*l"); f:close()
  local u,n,s,i=l:match("^cpu%s+(%d+)%s+(%d+)%s+(%d+)%s+(%d+)")
  if not u then return 0 end
  local busy=tonumber(u)+tonumber(s); local tot=busy+tonumber(n)+tonumber(i)
  return tot>0 and math.floor(busy*100/tot) or 0
end

local function poll_ram()
  local f=io.open("/proc/meminfo","r"); if not f then return "0","0" end
  local tot,avail
  for l in f:lines() do
    if l:find("^MemTotal:") then tot=tonumber(l:match("(%d+)")) end
    if l:find("^MemAvailable:") then avail=tonumber(l:match("(%d+)")) end
    if tot and avail then break end
  end
  f:close()
  if not tot then return "0","0" end
  return string.format("%.1f",(tot-(avail or 0))/1048576), string.format("%.1f",tot/1048576)
end

local function poll_ws()
  local r=rcmd("niri msg -j workspaces 2>/dev/null")
  if not r then return {} end
  local l={}
  for id,idx,foc in r:gmatch('"id":(%d+).-"idx":(%d+).-"is_focused":(.-)[,}]') do
    l[#l+1]={id=tostring(id),idx=tonumber(idx),active=foc:find("true")~=nil}
  end
  return l
end

local function poll_wifi()
  local r=rcmd("nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null | grep '^yes' | head -1")
  return r and r:match("^yes:(.+)") or ""
end

local function scan_wifi()
  local f=io.popen("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null")
  if not f then return {} end
  local nets,seen={},{}
  for line in f:lines() do
    local ssid,sig,sec,act=line:match("^(.-):(%d+):(.-):(.-)$")
    if ssid and ssid~="" and not seen[ssid] then
      seen[ssid]=true
      nets[#nets+1]={id=ssid,ssid=ssid,signal=tonumber(sig)or 0,secure=sec~="",active=act=="yes"}
    end
  end
  f:close()
  table.sort(nets,function(a,b) if a.active~=b.active then return a.active end; return a.signal>b.signal end)
  return nets
end

local function wifi_sig_icon(sig)
  if sig>75 then return I.wifi_4 elseif sig>50 then return I.wifi_3
  elseif sig>25 then return I.wifi_2 end; return I.wifi_1
end

-- ============================================================================
-- State
-- ============================================================================

function init()
  local v,m=poll_vol(); local b,bs=poll_bat(); local ru,rt=poll_ram()
  return {
    time=os.date("%H:%M"), user=rcmd("whoami")or"", host=rcmd("hostname")or"",
    kernel=rcmd("uname -r")or"", uptime=(rcmd("uptime -p 2>/dev/null")or""):gsub("^up ",""),
    vol=v, muted=m, bat=b, bat_st=bs, bright=poll_bri(),
    cpu=poll_cpu(), ram_u=ru, ram_t=rt,
    ws=poll_ws(), wifi=poll_wifi(), wifi_nets={},
    night_light=false, dnd=false, bt=false,
    popup=nil, tick_n=0,
  }
end

-- ============================================================================
-- Update
-- ============================================================================

function update(state, msg)
  local t=msg.type

  if t=="niri_event" then
    if msg.data and msg.data:find("^Workspaces changed:") then state.ws=poll_ws() end
    return state
  end

  if t=="tick" then
    state.tick_n=state.tick_n+1
    state.time=os.date("%H:%M")
    state.cpu=poll_cpu()
    if state.tick_n%3==0 then
      state.vol,state.muted=poll_vol()
      state.ram_u,state.ram_t=poll_ram()
      state.wifi=poll_wifi()
      state.bright=poll_bri()
    end
    if state.tick_n%10==0 then state.bat,state.bat_st=poll_bat() end
    if state.tick_n%30==0 then state.uptime=(rcmd("uptime -p 2>/dev/null")or""):gsub("^up ","") end
    return state
  end

  if t=="set_vol" then
    state.vol=math.floor(msg.data or state.vol); state.muted=false
    os.execute(string.format("wpctl set-volume @DEFAULT_AUDIO_SINK@ %.2f &",state.vol/100))
  elseif t=="toggle_mute" then
    state.muted=not state.muted
    os.execute("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle &")
  elseif t=="set_bri" then
    state.bright=math.floor(msg.data or state.bright)
    os.execute("brightnessctl set "..state.bright.."% &")
  elseif t=="ws_go" then
    if msg.data then os.execute("niri msg action focus-workspace "..msg.data.." &") end
  elseif t=="wifi_con" then
    if msg.data then os.execute("nmcli dev wifi connect '"..msg.data.."' &") end
  elseif t=="wifi_dis" then os.execute("nmcli dev disconnect wlan0 &")
  elseif t=="toggle_night" then
    state.night_light = not state.night_light
    -- Toggle gammastep/wlsunset if available
    if state.night_light then
      os.execute("wlsunset -T 4500 -t 3500 &")
    else
      os.execute("pkill wlsunset &")
    end
  elseif t=="toggle_dnd" then state.dnd = not state.dnd
  elseif t=="toggle_bt" then
    state.bt = not state.bt
    os.execute(state.bt and "bluetoothctl power on &" or "bluetoothctl power off &")
  elseif t=="toggle" then
    local name=msg.data
    if type(name)=="string" then
      if state.popup==name then state.popup=nil
      else
        state.popup=name
        if name=="wifi" then state.wifi_nets=scan_wifi() end
        if name=="audio" then state.vol,state.muted=poll_vol() end
        if name=="bright" then state.bright=poll_bri() end
        if name=="bat" then state.bat,state.bat_st=poll_bat() end
      end
    end
  elseif t=="dismiss" then state.popup=nil
  elseif t=="lock"     then os.execute("loginctl lock-session &"); state.popup=nil
  elseif t=="logout"   then os.execute("niri msg action quit &")
  elseif t=="suspend"  then os.execute("systemctl suspend &"); state.popup=nil
  elseif t=="reboot"   then os.execute("systemctl reboot &")
  elseif t=="shutdown" then os.execute("systemctl poweroff &")
  end
  return state
end

-- ============================================================================
-- View helpers
-- ============================================================================

local function ibtn(icon, action)
  return button({on_click=action, style="p-2 rounded-full hover:bg-surface"},
    text({style="text-lg text-fg"}, icon))
end

local function menu_item(icon, label, action, color)
  return button({on_click=action, style="px-3 py-2 rounded hover:bg-overlay items-center gap-3"},
    text({style="text-base "..(color or "text-fg")}, icon),
    text({style="text-base "..(color or "text-fg")}, label))
end

local function header(icon, title)
  return row({style="items-center gap-2"},
    text({style="text-xl text-fg"}, icon),
    text({style="text-base text-fg font-bold"}, title))
end

local function caption(txt)
  return text({style="text-xs text-muted"}, txt)
end

local function separator()
  return row({style="w-full h-1 bg-base rounded my-1"})
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

-- ============================================================================
-- View
-- ============================================================================

function view(state)
  local S = {}

  -- ── Bar ────────────────────────────────────────
  S[#S+1] = window("bar", {anchor="top", height=48, exclusive=true, monitor="all"},
    row({style="bg-base w-full h-full px-2 items-center gap-1"},

      -- Left section: workspaces (flex-1)
      row({style="flex-1 items-center gap-1"},
        each(state.ws, "id", function(ws)
          return ibtn(ws.active and I.circle_f or I.circle,
            msg("ws_go", tostring(ws.idx)))
        end)),

      -- Center: clock
      text({style="text-base text-fg font-bold"}, state.time),

      -- Right section: stats + icons (flex-1, right-aligned)
      row({style="flex-1 items-center justify-end gap-1"},
        text({style="text-sm text-muted"}, I.cpu.." "..state.cpu.."%"),
        text({style="text-sm text-muted"}, I.ram.." "..state.ram_u.."/"..state.ram_t.."G"),
        ibtn(state.wifi~="" and I.wifi_4 or I.wifi_off, msg("toggle","wifi")),
        ibtn(I.bright, msg("toggle","bright")),
        has_bat and ibtn(bat_i(state.bat,state.bat_st), msg("toggle","bat")) or spacer(),
        ibtn(vol_i(state.vol,state.muted), msg("toggle","audio")),
        ibtn(I.settings, msg("toggle","settings")),
        ibtn(I.power, msg("toggle","power")))))

  -- ── Audio popup ────────────────────────────────
  if state.popup=="audio" then
    S[#S+1] = popup("audio", {anchor="top right",width=300,height=180,dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-3"},
        row({style="items-center gap-3"},
          text({style="text-xl text-fg"}, vol_i(state.vol,state.muted)),
          col({style="gap-1"},
            text({style="text-base text-fg font-bold"}, "Volume"),
            caption(state.muted and "Muted" or (state.vol.."%")))),
        slider({value=state.vol, min=0, max=100, on_change=msg("set_vol")}),
        button({on_click=msg("toggle_mute"), style="px-3 py-1 rounded hover:bg-overlay items-center gap-2"},
          text({style="text-sm text-fg"}, state.muted and "Unmute" or "Mute"))))
  end

  -- ── Brightness popup ───────────────────────────
  if state.popup=="bright" then
    S[#S+1] = popup("bright", {anchor="top right",width=300,height=140,dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-3"},
        row({style="items-center gap-3"},
          text({style="text-xl text-fg"}, I.bright),
          col({style="gap-1"},
            text({style="text-base text-fg font-bold"}, "Brightness"),
            caption(state.bright.."%"))),
        slider({value=state.bright, min=0, max=100, on_change=msg("set_bri")})))
  end

  -- ── Battery popup ──────────────────────────────
  if state.popup=="bat" then
    S[#S+1] = popup("bat", {anchor="top right",width=280,height=120,dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-2"},
        row({style="items-center gap-3"},
          text({style="text-xl text-fg"}, bat_i(state.bat,state.bat_st)),
          col({style="gap-1"},
            text({style="text-base text-fg font-bold"}, "Battery"),
            caption(state.bat_st))),
        row({style="items-center gap-2"},
          text({style="text-2xl text-fg font-bold"}, state.bat.."%"))))
  end

  -- ── WiFi popup ─────────────────────────────────
  if state.popup=="wifi" then
    local ch = {}
    ch[#ch+1] = row({style="items-center gap-3"},
      text({style="text-xl text-fg"}, I.wifi_4),
      text({style="text-base text-fg font-bold"}, "Networks"),
      spacer(),
      caption(state.wifi~="" and state.wifi or "disconnected"))
    ch[#ch+1] = separator()

    local n=0
    for _,net in ipairs(state.wifi_nets) do
      if n>=10 then break end; n=n+1
      local color = net.active and "text-primary" or "text-fg"
      ch[#ch+1] = button({
        on_click=net.active and msg("wifi_dis") or msg("wifi_con",net.ssid),
        style="px-2 py-1 rounded hover:bg-overlay items-center gap-2"},
        text({style="text-base "..color}, wifi_sig_icon(net.signal)),
        text({style="text-sm "..color..(net.active and " font-bold" or "")},
          net.ssid..(net.secure and " 󰌾" or "")),
        spacer(),
        net.active and text({style="text-xs text-primary"}, I.check) or
          text({style="text-xs text-muted"}, net.signal.."%"))
    end

    S[#S+1] = popup("wifi", {anchor="top right",width=320,height=60+n*32,dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-1"}, unpack(ch)))
  end

  -- ── Settings popup (COSMIC-style quick settings) ─
  if state.popup=="settings" then
    local function tile(icon, label, active, action)
      local bg = active and "bg-primary" or "bg-overlay"
      local fg = active and "text-base" or "text-fg"
      return button({on_click=action, style=bg.." rounded-lg p-3 items-center gap-1 flex-1"},
        text({style="text-lg "..fg}, icon),
        text({style="text-xs "..fg}, label))
    end

    S[#S+1] = popup("settings", {anchor="top right",width=340,height=380,dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-3"},
        text({style="text-base text-fg font-bold"}, "Quick Settings"),
        separator(),
        -- Toggle tiles row 1
        row({style="gap-2"},
          tile(state.wifi~="" and I.wifi_4 or I.wifi_off, "WiFi",
            state.wifi~="", msg("toggle","wifi")),
          tile(state.bt and I.bt or I.bt_off, "Bluetooth",
            state.bt, msg("toggle_bt")),
          tile(state.dnd and I.dnd or I.dnd_off, "Do Not Disturb",
            state.dnd, msg("toggle_dnd"))),
        -- Toggle tiles row 2
        row({style="gap-2"},
          tile(state.night_light and I.night or I.night_off, "Night Light",
            state.night_light, msg("toggle_night")),
          tile(I.display, "Display", false, msg("dismiss")),
          tile(I.audio_out, "Audio", false, msg("toggle","audio"))),
        separator(),
        -- Volume slider
        row({style="items-center gap-2"},
          text({style="text-lg text-muted"}, vol_i(state.vol,state.muted)),
          text({style="text-xs text-muted"}, state.vol.."%")),
        slider({value=state.vol, min=0, max=100, on_change=msg("set_vol")}),
        -- Brightness slider
        row({style="items-center gap-2"},
          text({style="text-lg text-muted"}, I.bright),
          text({style="text-xs text-muted"}, state.bright.."%")),
        slider({value=state.bright, min=0, max=100, on_change=msg("set_bri")})))
  end

  -- ── Power popup ────────────────────────────────
  if state.popup=="power" then
    S[#S+1] = popup("power", {anchor="top right",width=300,height=340,dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-3"},
        -- User info
        col({style="gap-1 items-center"},
          text({style="text-base text-fg font-bold"}, state.user.."@"..state.host),
          caption(state.kernel),
          caption("Up "..state.uptime)),
        separator(),
        -- Actions
        menu_item(I.lock,     "Lock Screen", msg("lock")),
        menu_item(I.suspend,  "Suspend",     msg("suspend")),
        menu_item(I.reboot,   "Restart",     msg("reboot")),
        menu_item(I.shutdown, "Shut Down",   msg("shutdown")),
        separator(),
        menu_item(I.logout,   "Log Out",     msg("logout"), "text-error")))
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
