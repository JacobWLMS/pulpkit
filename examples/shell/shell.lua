-- ╔══════════════════════════════════════════════════╗
-- ║  Pulpkit v3 Shell                               ║
-- ╚══════════════════════════════════════════════════╝

-- Nerd Font Material Design icons (JetBrainsMono Nerd Font)
local I = {
  vol_hi   = "󰕾",  vol_mid  = "󰖀",  vol_lo   = "󰕿",  vol_mute = "󰝟",
  bat_full = "󰁹",  bat_good = "󰂀",  bat_half = "󰁾",  bat_low  = "󰁻",
  bat_chrg = "󰂄",  bat_empty = "󰂎",
  wifi_on  = "󰤨",  wifi_off = "󰤭",
  bright   = "󰃟",
  power    = "󰐥",  lock     = "󰌾",  suspend  = "󰤄",
  logout   = "󰗼",  reboot   = "󰜉",
  circle_f = "󰮍",  circle   = "󰊠",
  check    = "󰄬",  signal   = "󰤥",
  cpu      = "󰍛",  ram      = "󰘚",
}

-- ============================================================================
-- Helpers: read system state
-- ============================================================================

local function read_cmd(cmd)
  local f = io.popen(cmd)
  if not f then return nil end
  local out = f:read("*l")
  f:close()
  return out
end

local function read_file(path)
  local f = io.open(path, "r")
  if not f then return nil end
  local out = f:read("*l")
  f:close()
  return out
end

local function poll_volume()
  local raw = read_cmd("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null")
  if not raw then return 0, false end
  local v = raw:match("Volume:%s*(%d+%.?%d*)")
  local vol = v and math.floor(tonumber(v) * 100) or 0
  local muted = raw:find("%[MUTED%]") ~= nil
  return vol, muted
end

local function poll_battery()
  local cap = read_file("/sys/class/power_supply/BAT0/capacity")
  local st  = read_file("/sys/class/power_supply/BAT0/status")
  return tonumber(cap) or 100, (st or "Unknown"):match("%S+") or "Unknown"
end

local function poll_brightness()
  local raw = read_cmd("brightnessctl -m 2>/dev/null")
  if not raw then return 50 end
  local pct = raw:match(",(%d+)%%")
  return tonumber(pct) or 50
end

local function poll_cpu()
  local raw = read_cmd("awk '/^cpu /{u=$2+$4;t=$2+$4+$5;print int(u*100/t)}' /proc/stat")
  return tonumber(raw) or 0
end

local function poll_ram()
  local raw = read_cmd("free -m | awk '/Mem:/{printf \"%.1f %.1f\",$3/1024,$2/1024}'")
  if not raw then return "0", "0" end
  local u, t = raw:match("(%S+)%s+(%S+)")
  return u or "0", t or "0"
end

local function poll_workspaces()
  local raw = read_cmd("niri msg -j workspaces 2>/dev/null")
  if not raw then return {} end
  local list = {}
  for id, idx, foc in raw:gmatch('"id":(%d+).-"idx":(%d+).-"is_focused":(.-)[,}]') do
    list[#list+1] = { id=tostring(id), idx=tonumber(idx), active=foc:find("true")~=nil }
  end
  return list
end

local function poll_wifi_ssid()
  local raw = read_cmd("nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null | grep '^yes' | head -1")
  if not raw then return "" end
  return raw:match("^yes:(.+)") or ""
end

local function scan_wifi()
  local f = io.popen("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null")
  if not f then return {} end
  local nets, seen = {}, {}
  for line in f:lines() do
    local ssid, sig, sec, act = line:match("^(.-):(%d+):(.-):(.-)$")
    if ssid and ssid ~= "" and not seen[ssid] then
      seen[ssid] = true
      nets[#nets+1] = { id=ssid, ssid=ssid, signal=tonumber(sig)or 0, active=act=="yes" }
    end
  end
  f:close()
  table.sort(nets, function(a,b) if a.active~=b.active then return a.active end; return a.signal>b.signal end)
  return nets
end

-- ============================================================================
-- State
-- ============================================================================

local has_bat = read_file("/sys/class/power_supply/BAT0/capacity") ~= nil

function init()
  local vol, muted = poll_volume()
  local bat, bat_st = poll_battery()
  local ram_u, ram_t = poll_ram()
  return {
    time    = os.date("%H:%M"),
    user    = read_cmd("whoami") or "",
    host    = read_cmd("hostname") or "",
    vol     = vol,
    muted   = muted,
    bat     = bat,
    bat_st  = bat_st,
    bright  = poll_brightness(),
    cpu     = poll_cpu(),
    ram_u   = ram_u,
    ram_t   = ram_t,
    ws      = poll_workspaces(),
    wifi    = poll_wifi_ssid(),
    wifi_nets = {},
    popup   = nil,
  }
end

-- ============================================================================
-- Update
-- ============================================================================

function update(state, msg)
  local t = msg.type

  -- Periodic refresh (every second)
  if t == "tick" then
    state.time   = os.date("%H:%M")
    state.vol, state.muted = poll_volume()
    state.bat, state.bat_st = poll_battery()
    state.bright = poll_brightness()
    state.cpu    = poll_cpu()
    state.ram_u, state.ram_t = poll_ram()
    state.ws     = poll_workspaces()
    state.wifi   = poll_wifi_ssid()
    return state
  end

  -- Volume control
  if t == "set_vol" then
    state.vol = math.floor(msg.data or state.vol)
    os.execute(string.format("wpctl set-volume @DEFAULT_AUDIO_SINK@ %.2f &", state.vol/100))
  -- Brightness control
  elseif t == "set_bri" then
    state.bright = math.floor(msg.data or state.bright)
    os.execute("brightnessctl set " .. state.bright .. "% &")
  -- Workspace focus
  elseif t == "ws_go" then
    if msg.data then os.execute("niri msg action focus-workspace " .. msg.data .. " &") end
  -- WiFi
  elseif t == "wifi_con" then
    if msg.data then os.execute("nmcli dev wifi connect '" .. msg.data .. "' &") end
  elseif t == "wifi_dis" then
    os.execute("nmcli dev disconnect wlan0 &")
  -- Popup toggle
  elseif t == "toggle" then
    local name = msg.data
    if type(name) == "string" then
      if state.popup == name then
        state.popup = nil
      else
        state.popup = name
        if name == "wifi" then state.wifi_nets = scan_wifi() end
      end
    end
  elseif t == "dismiss" then
    state.popup = nil
  -- Power actions
  elseif t == "lock"     then os.execute("loginctl lock-session &"); state.popup = nil
  elseif t == "logout"   then os.execute("niri msg action quit &")
  elseif t == "suspend"  then os.execute("systemctl suspend &"); state.popup = nil
  elseif t == "reboot"   then os.execute("systemctl reboot &")
  elseif t == "shutdown" then os.execute("systemctl poweroff &")
  end
  return state
end

-- ============================================================================
-- View components
-- ============================================================================

local function ibtn(icon, action)
  return button({ on_click=action, style="p-2 rounded-full hover:bg-surface" },
    text({ style="text-lg text-fg" }, icon))
end

local function mbtn(icon, label, action, color)
  color = color or "text-fg"
  return button({ on_click=action, style="px-3 py-2 rounded hover:bg-overlay items-center gap-3" },
    text({ style="text-sm " .. color }, icon),
    text({ style="text-sm " .. color }, label))
end

local function vol_i(v, m)
  if m then return I.vol_mute end
  if v > 50 then return I.vol_hi elseif v > 0 then return I.vol_mid end
  return I.vol_lo
end

local function bat_i(p, s)
  if s == "Charging" then return I.bat_chrg end
  if p > 80 then return I.bat_full elseif p > 60 then return I.bat_good
  elseif p > 30 then return I.bat_half elseif p > 10 then return I.bat_low end
  return I.bat_empty
end

-- ============================================================================
-- View
-- ============================================================================

function view(state)
  local S = {}

  -- ── Bar ────────────────────────────────────────
  S[#S+1] = window("bar", { anchor="top", height=40, exclusive=true, monitor="all" },
    row({ style="bg-base w-full h-full px-2 items-center gap-1" },

      -- Workspaces
      each(state.ws, "id", function(ws)
        return button({ on_click=msg("ws_go", tostring(ws.idx)),
          style=ws.active and "px-2 py-1 rounded bg-primary" or "px-2 py-1 rounded hover:bg-surface" },
          text({ style=ws.active and "text-sm text-base" or "text-sm text-muted" }, tostring(ws.idx)))
      end),

      spacer(),
      text({ style="text-sm text-fg font-bold" }, state.time),
      spacer(),

      -- System stats
      row({ style="items-center gap-3 px-2" },
        text({ style="text-xs text-muted" }, I.cpu .. " " .. state.cpu .. "%"),
        text({ style="text-xs text-muted" }, I.ram .. " " .. state.ram_u .. "/" .. state.ram_t .. "G")),

      -- Status icons
      ibtn(state.wifi ~= "" and I.wifi_on or I.wifi_off, msg("toggle", "wifi")),
      ibtn(I.bright, msg("toggle", "bright")),
      has_bat and ibtn(bat_i(state.bat, state.bat_st), msg("toggle", "bat")) or spacer(),
      ibtn(vol_i(state.vol, state.muted), msg("toggle", "audio")),
      ibtn(I.power, msg("toggle", "power"))))

  -- ── Audio popup ────────────────────────────────
  if state.popup == "audio" then
    S[#S+1] = popup("audio", { anchor="top right", width=260, height=120, dismiss_on_outside=true },
      col({ style="bg-surface w-full h-full rounded-lg p-4 gap-3" },
        row({ style="items-center gap-2" },
          text({ style="text-lg text-muted" }, vol_i(state.vol, state.muted)),
          text({ style="text-sm text-fg font-bold" }, "Volume"),
          spacer(),
          text({ style="text-xs text-muted" }, state.vol .. "%")),
        slider({ value=state.vol, min=0, max=100, on_change=msg("set_vol") })))
  end

  -- ── Brightness popup ───────────────────────────
  if state.popup == "bright" then
    S[#S+1] = popup("bright", { anchor="top right", width=260, height=120, dismiss_on_outside=true },
      col({ style="bg-surface w-full h-full rounded-lg p-4 gap-3" },
        row({ style="items-center gap-2" },
          text({ style="text-lg text-muted" }, I.bright),
          text({ style="text-sm text-fg font-bold" }, "Brightness"),
          spacer(),
          text({ style="text-xs text-muted" }, state.bright .. "%")),
        slider({ value=state.bright, min=0, max=100, on_change=msg("set_bri") })))
  end

  -- ── Battery popup ──────────────────────────────
  if state.popup == "bat" then
    S[#S+1] = popup("bat", { anchor="top right", width=200, height=90, dismiss_on_outside=true },
      col({ style="bg-surface w-full h-full rounded-lg p-4 gap-2" },
        row({ style="items-center gap-2" },
          text({ style="text-lg text-fg" }, bat_i(state.bat, state.bat_st)),
          text({ style="text-sm text-fg font-bold" }, "Battery")),
        row({ style="items-center gap-3" },
          text({ style="text-xl text-fg font-bold" }, state.bat .. "%"),
          text({ style="text-sm text-muted" }, state.bat_st))))
  end

  -- ── WiFi popup ─────────────────────────────────
  if state.popup == "wifi" then
    local ch = {
      row({ style="items-center gap-2 px-1" },
        text({ style="text-lg text-muted" }, I.wifi_on),
        text({ style="text-sm text-fg font-bold" }, "WiFi"),
        spacer(),
        text({ style="text-xs text-muted" }, state.wifi ~= "" and state.wifi or "disconnected")),
      row({ style="w-full h-1 bg-base rounded" }),
    }
    local n = 0
    for _, net in ipairs(state.wifi_nets) do
      if n >= 8 then break end; n = n + 1
      local lbl = net.ssid .. (net.active and (" " .. I.check) or "")
      ch[#ch+1] = button({
        on_click = net.active and msg("wifi_dis") or msg("wifi_con", net.ssid),
        style = "px-2 py-1 rounded hover:bg-overlay items-center gap-2" },
        text({ style="text-xs text-muted" }, I.signal),
        text({ style="text-xs text-fg" }, lbl),
        spacer(),
        text({ style="text-xs text-muted" }, net.signal .. "%"))
    end
    S[#S+1] = popup("wifi", { anchor="top right", width=280, height=50+n*28, dismiss_on_outside=true },
      col({ style="bg-surface w-full h-full rounded-lg p-3 gap-1" }, unpack(ch)))
  end

  -- ── Power popup ────────────────────────────────
  if state.popup == "power" then
    S[#S+1] = popup("power", { anchor="top right", width=200, height=230, dismiss_on_outside=true },
      col({ style="bg-surface w-full h-full rounded-lg p-3 gap-1" },
        row({ style="items-center gap-2 px-2 py-1" },
          text({ style="text-sm text-fg font-bold" }, state.user .. "@" .. state.host)),
        row({ style="w-full h-1 bg-base rounded" }),
        mbtn(I.lock,    "Lock",      msg("lock")),
        mbtn(I.logout,  "Log Out",   msg("logout")),
        mbtn(I.suspend, "Suspend",   msg("suspend")),
        mbtn(I.reboot,  "Restart",   msg("reboot")),
        mbtn(I.power,   "Shut Down", msg("shutdown"), "text-error")))
  end

  return S
end

-- ============================================================================
-- Subscriptions
-- ============================================================================

function subscribe(state)
  return {
    interval(1000, "tick"),
    ipc("ipc"),
  }
end
