-- ╔══════════════════════════════════════════════════╗
-- ║  Pulpkit v3 Shell                               ║
-- ╚══════════════════════════════════════════════════╝

local I = {
  vol_hi="󰕾", vol_mid="󰖀", vol_lo="󰕿", vol_mute="󰝟",
  bat_full="󰁹", bat_good="󰂀", bat_half="󰁾", bat_low="󰁻", bat_chrg="󰂄", bat_empty="󰂎",
  wifi_4="󰤨", wifi_3="󰤥", wifi_2="󰤢", wifi_1="󰤟", wifi_off="󰤭",
  bright="󰃟",
  power="󰐥", lock="󰌾", suspend="󰤄", logout="󰗼", reboot="󰜉", shutdown="󰐦",
  dot_f="󰮍", dot="󰊠",
  check="󰄬",
  cpu="󰍛", ram="󰘚",
  settings="󰒓", night="󰌵", night_off="󰌶",
  dnd="󰍶", dnd_off="󰍷", bt="󰂯", bt_off="󰂲",
  audio_out="󰓃",
}

-- ── System polling ───────────────────────────────
local function rc(c) local f=io.popen(c);if not f then return nil end;local o=f:read("*l");f:close();return o end
local function rf(p) local f=io.open(p,"r");if not f then return nil end;local o=f:read("*l");f:close();return o end
local has_bat=rf("/sys/class/power_supply/BAT0/capacity")~=nil

local function poll_vol()
  local r=rc("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null")
  if not r then return 0,false end
  local v=r:match("Volume:%s*(%d+%.?%d*)");return v and math.floor(tonumber(v)*100) or 0,r:find("%[MUTED%]")~=nil
end
local function poll_bat() return tonumber(rf("/sys/class/power_supply/BAT0/capacity"))or 100,(rf("/sys/class/power_supply/BAT0/status")or"Unknown"):match("%S+")or"Unknown" end
local function poll_bri() local r=rc("brightnessctl -m 2>/dev/null");return r and tonumber(r:match(",(%d+)%%"))or 50 end
local function poll_cpu()
  local f=io.open("/proc/stat","r");if not f then return 0 end;local l=f:read("*l");f:close()
  local u,n,s,i=l:match("^cpu%s+(%d+)%s+(%d+)%s+(%d+)%s+(%d+)");if not u then return 0 end
  local b=tonumber(u)+tonumber(s);local t=b+tonumber(n)+tonumber(i);return t>0 and math.floor(b*100/t) or 0
end
local function poll_ram()
  local f=io.open("/proc/meminfo","r");if not f then return "0","0" end;local tot,avl
  for l in f:lines() do
    if l:find("^MemTotal:") then tot=tonumber(l:match("(%d+)")) end
    if l:find("^MemAvailable:") then avl=tonumber(l:match("(%d+)")) end
    if tot and avl then break end
  end;f:close();if not tot then return "0","0" end
  return string.format("%.1f",(tot-(avl or 0))/1048576),string.format("%.1f",tot/1048576)
end
local function poll_ws()
  local r=rc("niri msg -j workspaces 2>/dev/null");if not r then return {} end;local l={}
  for id,idx,foc in r:gmatch('"id":(%d+).-"idx":(%d+).-"is_focused":(.-)[,}]') do
    l[#l+1]={id=tostring(id),idx=tonumber(idx),active=foc:find("true")~=nil} end;return l
end
local function poll_wifi() local r=rc("nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null|grep '^yes'|head -1");return r and r:match("^yes:(.+)") or "" end
local function scan_wifi()
  local f=io.popen("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null");if not f then return {} end
  local n,s={},{};for l in f:lines() do local ss,sg,sc,a=l:match("^(.-):(%d+):(.-):(.-)$")
    if ss and ss~="" and not s[ss] then s[ss]=true;n[#n+1]={id=ss,ssid=ss,signal=tonumber(sg)or 0,secure=sc~="",active=a=="yes"} end
  end;f:close();table.sort(n,function(a,b) if a.active~=b.active then return a.active end;return a.signal>b.signal end);return n
end
local function wsig(s) if s>75 then return I.wifi_4 elseif s>50 then return I.wifi_3 elseif s>25 then return I.wifi_2 end;return I.wifi_1 end
local function voli(v,m) if m then return I.vol_mute end;if v>50 then return I.vol_hi elseif v>0 then return I.vol_mid end;return I.vol_lo end
local function bati(p,s) if s=="Charging" then return I.bat_chrg end;if p>80 then return I.bat_full elseif p>60 then return I.bat_good elseif p>30 then return I.bat_half elseif p>10 then return I.bat_low end;return I.bat_empty end

-- ── State ────────────────────────────────────────

function init()
  local v,m=poll_vol();local b,bs=poll_bat();local ru,rt=poll_ram()
  return {
    time=os.date("%H:%M"),user=rc("whoami")or"",host=rc("hostname")or"",
    kernel=rc("uname -r")or"",uptime=(rc("uptime -p 2>/dev/null")or""):gsub("^up ",""),
    vol=v,muted=m,bat=b,bat_st=bs,bright=poll_bri(),
    cpu=poll_cpu(),ram_u=ru,ram_t=rt,
    ws=poll_ws(),wifi=poll_wifi(),wifi_nets={},
    night=false,dnd=false,bt=false,
    popup=nil,tick_n=0,
  }
end

-- ── Update ───────────────────────────────────────

function update(state, msg)
  local t=msg.type
  if t=="niri_event" then
    if msg.data and msg.data:find("^Workspaces changed:") then state.ws=poll_ws() end;return state end
  if t=="tick" then
    state.tick_n=state.tick_n+1;state.time=os.date("%H:%M");state.cpu=poll_cpu()
    if state.tick_n%3==0 then state.vol,state.muted=poll_vol();state.ram_u,state.ram_t=poll_ram();state.wifi=poll_wifi();state.bright=poll_bri() end
    if state.tick_n%10==0 then state.bat,state.bat_st=poll_bat() end
    if state.tick_n%30==0 then state.uptime=(rc("uptime -p 2>/dev/null")or""):gsub("^up ","") end
    return state end
  if t=="set_vol" then state.vol=math.floor(msg.data or state.vol);state.muted=false;os.execute(string.format("wpctl set-volume @DEFAULT_AUDIO_SINK@ %.2f &",state.vol/100))
  elseif t=="toggle_mute" then state.muted=not state.muted;os.execute("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle &")
  elseif t=="set_bri" then state.bright=math.floor(msg.data or state.bright);os.execute("brightnessctl set "..state.bright.."% &")
  elseif t=="ws_go" then if msg.data then os.execute("niri msg action focus-workspace "..msg.data.." &") end
  elseif t=="wifi_con" then if msg.data then os.execute("nmcli dev wifi connect '"..msg.data.."' &") end
  elseif t=="wifi_dis" then os.execute("nmcli dev disconnect wlan0 &")
  elseif t=="toggle_night" then state.night=not state.night;os.execute(state.night and "wlsunset -T 4500 -t 3500 &" or "pkill wlsunset &")
  elseif t=="toggle_dnd" then state.dnd=not state.dnd
  elseif t=="toggle_bt" then state.bt=not state.bt;os.execute(state.bt and "bluetoothctl power on &" or "bluetoothctl power off &")
  elseif t=="toggle_wifi_radio" then
    if state.wifi~="" then os.execute("nmcli radio wifi off &");state.wifi="" else os.execute("nmcli radio wifi on &") end
  elseif t=="toggle" then
    local n=msg.data;if type(n)=="string" then
      if state.popup==n then state.popup=nil else state.popup=n
        if n=="wifi" then state.wifi_nets=scan_wifi() end
        if n=="bat" then state.bat,state.bat_st=poll_bat() end end end
  elseif t=="dismiss" then state.popup=nil
  elseif t=="lock" then os.execute("loginctl lock-session &");state.popup=nil
  elseif t=="logout" then os.execute("niri msg action quit &")
  elseif t=="suspend" then os.execute("systemctl suspend &");state.popup=nil
  elseif t=="reboot" then os.execute("systemctl reboot &")
  elseif t=="shutdown" then os.execute("systemctl poweroff &")
  end;return state
end

-- ── View components ──────────────────────────────

-- Bar icon button: consistent 28px square hit target
local function ibtn(icon, action)
  return button({on_click=action, style="px-2 py-1 rounded hover:bg-surface items-center justify-center"},
    text({style="text-lg text-fg"}, icon))
end

-- Popup section separator
local function sep()
  return row({style="w-full bg-overlay rounded", height=1})
end

-- Popup menu item
local function mitem(icon, label, action, color)
  color = color or "text-fg"
  return button({on_click=action, style="px-3 py-2 rounded hover:bg-overlay items-center gap-3"},
    text({style="text-lg "..color}, icon),
    text({style="text-base "..color}, label))
end

-- ── View ─────────────────────────────────────────

function view(state)
  local S = {}

  -- ═══════════════════════════════════════════════
  -- BAR — 32px, three sections
  -- ═══════════════════════════════════════════════
  S[#S+1] = window("bar", {anchor="top", height=32, exclusive=true, monitor="all"},
    row({style="bg-base w-full h-full px-3 items-center"},

      -- LEFT: workspaces
      row({style="flex-1 items-center gap-1"},
        each(state.ws, "id", function(ws)
          return button({on_click=msg("ws_go",tostring(ws.idx)),
            style=ws.active
              and "px-2 py-1 rounded bg-primary items-center justify-center"
              or  "px-2 py-1 rounded hover:bg-surface items-center justify-center"},
            text({style=ws.active and "text-sm text-base font-bold" or "text-sm text-muted"}, tostring(ws.idx)))
        end)),

      -- CENTER: clock
      text({style="text-base text-fg font-medium"}, state.time),

      -- RIGHT: status
      row({style="flex-1 items-center justify-end gap-2"},
        text({style="text-xs text-muted"}, I.cpu.." "..state.cpu.."%"),
        text({style="text-xs text-muted"}, I.ram.." "..state.ram_u.."/"..state.ram_t.."G"),
        ibtn(state.wifi~="" and I.wifi_4 or I.wifi_off, msg("toggle","wifi")),
        has_bat and ibtn(bati(state.bat,state.bat_st), msg("toggle","bat")) or spacer(),
        ibtn(I.settings, msg("toggle","settings")),
        ibtn(I.power, msg("toggle","power")))))

  -- ═══════════════════════════════════════════════
  -- SETTINGS — quick settings panel
  -- ═══════════════════════════════════════════════
  if state.popup=="settings" then
    local function tile(icon, label, active, action)
      return button({on_click=action,
        style=(active and "bg-primary" or "bg-overlay")
          .." rounded-lg px-2 py-3 items-center gap-1 flex-1"},
        text({style="text-lg "..(active and "text-base" or "text-fg")}, icon),
        text({style="text-xs "..(active and "text-base" or "text-muted")}, label))
    end

    S[#S+1] = popup("settings", {anchor="top right", width=280, height=300, dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-3"},
        text({style="text-sm text-fg font-bold"}, "Quick Settings"),

        -- Toggle grid
        row({style="gap-2"},
          tile(state.wifi~="" and I.wifi_4 or I.wifi_off, "WiFi", state.wifi~="", msg("toggle_wifi_radio")),
          tile(state.bt and I.bt or I.bt_off, "Bluetooth", state.bt, msg("toggle_bt")),
          tile(state.dnd and I.dnd or I.dnd_off, "DND", state.dnd, msg("toggle_dnd"))),
        row({style="gap-2"},
          tile(state.night and I.night or I.night_off, "Night", state.night, msg("toggle_night")),
          tile(voli(state.vol,state.muted), "Mute", state.muted, msg("toggle_mute")),
          tile(I.audio_out, "Audio", false, msg("dismiss"))),

        sep(),

        -- Volume
        row({style="items-center gap-2"},
          text({style="text-base text-muted"}, voli(state.vol,state.muted)),
          slider({value=state.vol, min=0, max=100, on_change=msg("set_vol"), style="flex-1"}),
          text({style="text-xs text-muted w-8"}, state.vol.."%")),

        -- Brightness
        row({style="items-center gap-2"},
          text({style="text-base text-muted"}, I.bright),
          slider({value=state.bright, min=0, max=100, on_change=msg("set_bri"), style="flex-1"}),
          text({style="text-xs text-muted w-8"}, state.bright.."%"))))
  end

  -- ═══════════════════════════════════════════════
  -- BATTERY
  -- ═══════════════════════════════════════════════
  if state.popup=="bat" then
    S[#S+1] = popup("bat", {anchor="top right", width=220, height=80, dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-2"},
        row({style="items-center gap-3"},
          text({style="text-xl text-fg"}, bati(state.bat,state.bat_st)),
          col({style="gap-0"},
            text({style="text-base text-fg font-bold"}, state.bat.."%"),
            text({style="text-xs text-muted"}, state.bat_st)))))
  end

  -- ═══════════════════════════════════════════════
  -- WIFI
  -- ═══════════════════════════════════════════════
  if state.popup=="wifi" then
    local ch = {}
    ch[#ch+1] = row({style="items-center gap-3"},
      text({style="text-lg text-fg"}, I.wifi_4),
      text({style="text-sm text-fg font-bold"}, "Networks"),
      spacer(),
      text({style="text-xs text-muted"}, state.wifi~="" and state.wifi or "disconnected"))
    ch[#ch+1] = sep()

    local n = 0
    for _, net in ipairs(state.wifi_nets) do
      if n >= 8 then break end; n = n + 1
      local c = net.active and "text-primary" or "text-fg"
      ch[#ch+1] = button({
        on_click = net.active and msg("wifi_dis") or msg("wifi_con", net.ssid),
        style = "px-2 py-1 rounded hover:bg-overlay items-center gap-2"},
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

  -- ═══════════════════════════════════════════════
  -- POWER
  -- ═══════════════════════════════════════════════
  if state.popup=="power" then
    S[#S+1] = popup("power", {anchor="top right", width=260, height=300, dismiss_on_outside=true},
      col({style="bg-surface w-full h-full rounded-lg p-4 gap-2"},
        -- User info header
        col({style="gap-1 items-center py-2"},
          text({style="text-base text-fg font-bold"}, state.user.."@"..state.host),
          text({style="text-xs text-muted"}, state.kernel),
          text({style="text-xs text-muted"}, "Up "..state.uptime)),
        sep(),
        -- Actions
        mitem(I.lock,     "Lock Screen", msg("lock")),
        mitem(I.suspend,  "Suspend",     msg("suspend")),
        mitem(I.reboot,   "Restart",     msg("reboot")),
        mitem(I.shutdown, "Shut Down",   msg("shutdown")),
        sep(),
        mitem(I.logout,   "Log Out",     msg("logout"), "text-error")))
  end

  return S
end

-- ── Subscriptions ────────────────────────────────

function subscribe(state)
  return {
    interval(1000, "tick"),
    stream("niri msg event-stream 2>/dev/null", "niri_event"),
    ipc("ipc"),
  }
end
