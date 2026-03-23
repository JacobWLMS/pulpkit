-- ╔══════════════════════════════════════════════════════════════════════════╗
-- ║  Pulpkit Shell                                                        ║
-- ╚══════════════════════════════════════════════════════════════════════════╝

local lib = dofile(shell_dir .. "/lib.lua")
local icons = lib.icons
local S = lib.spacing
local T = lib.text_styles

-- ============================================================================
-- State
-- ============================================================================

local volume      = signal(0)
local muted       = signal(false)
local bat_pct     = signal(0)
local bat_status  = signal("")
local brightness  = signal(50)
local active_ws   = signal(1)
local ws_count    = signal(5)
local time_str    = signal(os.date("%H:%M"))
local date_str    = signal(os.date("%A, %B %d"))
local has_battery = lib.read_file("/sys/class/power_supply/BAT0/capacity") ~= nil

-- WiFi networks (polled, not called during render)
local wifi_list = {}  -- plain table, updated by poll
local wifi_version = signal(0) -- increment to trigger re-render

local function poll_wifi()
  local raw = exec_output("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null")
  local seen = {}
  wifi_list = {}
  for line in raw:gmatch("[^\n]+") do
    local ssid, sig, sec, active = line:match("^(.-):(%d+):(.-):(.-)$")
    if ssid and ssid ~= "" and not seen[ssid] then
      seen[ssid] = true
      wifi_list[#wifi_list+1] = {
        ssid = ssid, signal = tonumber(sig) or 0,
        secure = sec ~= "", active = active == "yes",
      }
    end
  end
  table.sort(wifi_list, function(a, b)
    if a.active ~= b.active then return a.active end
    return a.signal > b.signal
  end)
  wifi_version:set(wifi_version:get() + 1)
end

-- Bluetooth devices (polled)
local bt_list = {}
local bt_version = signal(0)

local function poll_bluetooth()
  local raw = exec_output("bluetoothctl devices 2>/dev/null")
  bt_list = {}
  for line in raw:gmatch("[^\n]+") do
    local mac, name = line:match("Device (%S+) (.+)")
    if mac and name then
      local info = exec_output("bluetoothctl info " .. mac .. " 2>/dev/null")
      local connected = info:find("Connected: yes") ~= nil
      bt_list[#bt_list+1] = { mac = mac, name = name, connected = connected }
    end
  end
  table.sort(bt_list, function(a, b)
    if a.connected ~= b.connected then return a.connected end
    return a.name < b.name
  end)
  bt_version:set(bt_version:get() + 1)
end

-- ============================================================================
-- Event Streams (push-based, near-zero CPU)
-- ============================================================================

-- Niri workspace events — instant updates, no polling
if env("NIRI_SOCKET") then
  exec_stream("niri msg event-stream", function(line)
    if line:find("^Workspaces changed:") then
      -- Parse workspace data from rust debug format
      local count = 0
      local focused = 1
      for block in line:gmatch("Workspace {(.-)}") do
        count = count + 1
        local idx = block:match("idx: (%d+)")
        if block:find("is_focused: true") and idx then
          focused = tonumber(idx)
        end
      end
      if count > 0 then ws_count:set(count) end
      active_ws:set(focused)
    end
  end)
else
  -- Fallback: poll if not on niri
  set_interval(function()
    local a, c = lib.poll_niri_workspaces()
    active_ws:set(a); ws_count:set(c)
  end, 3000)
end

-- Initial system state
do
  local v, m = lib.poll_volume()
  volume:set(v); muted:set(m)
  if has_battery then
    local p, s = lib.poll_battery()
    if p then bat_pct:set(p); bat_status:set(s) end
  end
  -- Brightness
  local bg = tonumber(exec_output("brightnessctl g 2>/dev/null")) or 0
  local bm = tonumber(exec_output("brightnessctl m 2>/dev/null")) or 1
  if bm > 0 then brightness:set(math.floor(bg / bm * 100)) end
end

-- Slow polls for things without event streams
set_interval(function() time_str:set(os.date("%H:%M")) end, 10000)
set_interval(function()
  local v, m = lib.poll_volume()
  volume:set(v); muted:set(m)
  if has_battery then
    local p, s = lib.poll_battery()
    if p then bat_pct:set(p); bat_status:set(s) end
  end
end, 10000)
set_interval(function() date_str:set(os.date("%A, %B %d")) end, 300000)

-- ============================================================================
-- Popups
-- ============================================================================

local show_audio,    toggle_audio,    close_audio    = lib.popup_toggle()
local show_power,    toggle_power,    close_power    = lib.popup_toggle()
local show_launcher, toggle_launcher, close_launcher = lib.popup_toggle()
local show_network,  toggle_network,  close_network  = lib.popup_toggle()
local show_bluetooth, toggle_bluetooth, close_bluetooth = lib.popup_toggle()
local show_battery,  toggle_battery,  close_battery  = lib.popup_toggle()
local show_calendar, toggle_calendar, close_calendar = lib.popup_toggle()
local search_query   = signal("")
local selected_index = signal(1)
local cursor_blink   = signal(true)

-- Blink only when launcher is open — no re-render when closed.
set_interval(function()
  if show_launcher:get() then
    cursor_blink:set(not cursor_blink:get())
  else
    cursor_blink:set(true) -- reset to visible, no-op if already true (PartialEq skip)
  end
end, 530)

function close_all_popups()
  close_audio(); close_power(); close_launcher()
  close_network(); close_bluetooth(); close_battery(); close_calendar()
  search_query:set(""); selected_index:set(1)
end

function _toggle_launcher()  close_all_popups(); toggle_launcher() end
function _toggle_power()     close_all_popups(); toggle_power() end
function _toggle_audio()     close_all_popups(); toggle_audio() end
function _toggle_network()   close_all_popups(); poll_wifi(); toggle_network() end
function _toggle_bluetooth() close_all_popups(); poll_bluetooth(); toggle_bluetooth() end
function _toggle_battery()   close_all_popups(); toggle_battery() end
function _toggle_calendar()  close_all_popups(); toggle_calendar() end

-- ============================================================================
-- Bar
-- ============================================================================

window("bar", {
  monitor = "all",
  anchor  = "top",
  exclusive = true,
  height  = 40,
}, function(ctx)
  return row("w-full h-10 bg-base px-2 items-center gap-1", {

    -- Left: search + workspaces
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
            return active_ws:get() == ws.id
              and (T.icon .. " text-primary")
              or (T.icon .. " text-muted")
          end,
          on_click = function()
            lib.focus_workspace(ws.id)
            active_ws:set(ws.id)
          end,
        })
      end, function(ws) return tostring(ws.id) end, "row"),
    }),

    -- Center: clock (click for calendar)
    lib.btn(function() return time_str:get() end, {
      text_style = T.body .. " text-fg font-medium",
      on_click = function() close_all_popups(); toggle_calendar() end,
    }),

    -- Right: status icons
    row("flex-1 items-center justify-end gap-1", {
      -- WiFi
      lib.icon_btn(icons.wifi_on, {
        on_click = function() close_all_popups(); toggle_network() end,
      }),

      -- Bluetooth
      lib.icon_btn(icons.bt_on, {
        on_click = function() close_all_popups(); toggle_bluetooth() end,
      }),

      -- Battery
      (function()
        if not has_battery then return spacer() end
        return lib.icon_btn(function()
          return lib.bat_icon(bat_pct:get(), bat_status:get())
        end, {
          on_click = function() close_all_popups(); toggle_battery() end,
        })
      end)(),

      -- Volume
      lib.icon_btn(function()
        return lib.vol_icon(volume:get(), muted:get())
      end, {
        on_click = function() close_all_popups(); toggle_audio() end,
        on_scroll_up = function() volume:set(lib.set_volume(volume:get() + 5)) end,
        on_scroll_down = function() volume:set(lib.set_volume(volume:get() - 5)) end,
      }),
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
  parent = "bar", anchor = "top right",
  visible = show_audio, dismiss_on_outside = true,
  width = 300, height = 180,
}, function()
  return col("w-full h-full bg-surface " .. S.popup_pad .. " " .. S.popup_gap, {
    row("items-center " .. S.item_gap, {
      text(T.icon_large .. " text-fg", function()
        return lib.vol_icon(volume:get(), muted:get())
      end),
      col("gap-1", {
        lib.header("Volume"),
        lib.caption(function()
          return muted:get() and "Muted" or (volume:get() .. "%")
        end),
      }),
    }),
    lib.slider_row(nil, volume, {
      on_change = function(v) volume:set(lib.set_volume(v)) end,
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
  parent = "bar", anchor = "center",
  visible = show_power, dismiss_on_outside = true,
  width = 340, height = 380,
  keyboard = true,
  on_key = function(key) if key == "Escape" then close_all_popups() end end,
}, function()
  local user = exec_output("whoami")
  local host = exec_output("hostname")
  local kernel = exec_output("uname -r")
  local uptime = exec_output("uptime -p 2>/dev/null"):gsub("^up ", "")

  return col("w-full h-full bg-surface " .. S.popup_pad .. " " .. S.popup_gap, {
    -- User + system info
    col("gap-1 items-center", {
      text(T.body .. " text-fg font-bold", user .. "@" .. host),
      lib.caption(kernel),
      lib.caption("Up " .. uptime),
    }),

    lib.separator(),

    -- Actions
    lib.menu_item(icons.lock,    "Lock Screen",  { on_click = function() exec("loginctl lock-session"); close_all_popups() end }),
    lib.menu_item(icons.suspend, "Suspend",      { on_click = function() exec("systemctl suspend"); close_all_popups() end }),
    lib.menu_item(icons.reboot,  "Restart",      { on_click = function() exec("systemctl reboot") end }),
    lib.menu_item("󰐦",           "Shut Down",    { on_click = function() exec("systemctl poweroff") end }),

    lib.separator(),

    lib.menu_item(icons.logout,  "Log Out", {
      text_style = T.body .. " text-error",
      icon_style = T.icon .. " text-error",
      on_click = function()
        exec("loginctl terminate-session " .. (env("XDG_SESSION_ID") or ""))
      end,
    }),
  })
end)

-- ============================================================================
-- Network Popup
-- ============================================================================

popup("network", {
  parent = "bar", anchor = "top right",
  visible = show_network, dismiss_on_outside = true,
  width = 320, height = 400,
  keyboard = true,
  on_key = function(key) if key == "Escape" then close_all_popups() end end,
}, function()
  return col("w-full h-full bg-surface " .. S.popup_pad .. " " .. S.popup_gap, {
    row("items-center " .. S.item_gap, {
      text(T.icon_large .. " text-fg", icons.wifi_on),
      lib.header("Networks"),
      spacer(),
      lib.icon_btn("󰑓", {
        on_click = function() poll_wifi() end,
      }),
    }),

    scroll("w-full flex-1", {
      each(function()
        local _ = wifi_version:get() -- trigger re-render on poll
        local r = {}
        for i = 1, math.min(#wifi_list, 12) do r[#r+1] = wifi_list[i] end
        return r
      end, function(net)
        local sig_icon
        if net.signal > 75 then sig_icon = "󰤨"
        elseif net.signal > 50 then sig_icon = "󰤥"
        elseif net.signal > 25 then sig_icon = "󰤢"
        else sig_icon = "󰤟" end

        return lib.menu_item(
          sig_icon,
          net.ssid .. (net.secure and " 󰌾" or ""),
          {
            text_style = net.active
              and (T.body .. " text-primary font-bold")
              or (T.body .. " text-fg"),
            on_click = function()
              if net.active then
                exec("nmcli con down id '" .. net.ssid .. "'")
              else
                exec("nmcli dev wifi connect '" .. net.ssid .. "'")
              end
              set_timeout(function() poll_wifi() end, 2000)
            end,
          }
        )
      end, function(net) return net.ssid end),
    }),
  })
end)

-- ============================================================================
-- Bluetooth Popup
-- ============================================================================

popup("bluetooth", {
  parent = "bar", anchor = "top right",
  visible = show_bluetooth, dismiss_on_outside = true,
  width = 300, height = 320,
  keyboard = true,
  on_key = function(key) if key == "Escape" then close_all_popups() end end,
}, function()
  return col("w-full h-full bg-surface " .. S.popup_pad .. " " .. S.popup_gap, {
    row("items-center " .. S.item_gap, {
      text(T.icon_large .. " text-fg", icons.bt_on),
      lib.header("Bluetooth"),
      spacer(),
      lib.icon_btn("󰑓", {
        on_click = function() poll_bluetooth() end,
      }),
    }),

    scroll("w-full flex-1", {
      each(function()
        local _ = bt_version:get()
        local r = {}
        for i = 1, #bt_list do r[#r+1] = bt_list[i] end
        return r
      end, function(dev)
        return lib.menu_item(
          dev.connected and "󰂱" or icons.bt_off,
          dev.name,
          {
            text_style = dev.connected
              and (T.body .. " text-primary font-bold")
              or (T.body .. " text-fg"),
            on_click = function()
              if dev.connected then
                exec("bluetoothctl disconnect " .. dev.mac)
              else
                exec("bluetoothctl connect " .. dev.mac)
              end
              set_timeout(function() poll_bluetooth() end, 3000)
            end,
          }
        )
      end, function(dev) return dev.mac end),
    }),

    -- No devices message
    (function()
      if #bt_list == 0 then
        return text(T.small .. " text-muted", "No devices found")
      end
      return spacer()
    end)(),
  })
end)

-- ============================================================================
-- Battery Popup
-- ============================================================================

local power_profile = signal("balanced")
-- Init power profile
do
  local p = exec_output("powerprofilesctl get 2>/dev/null")
  if p ~= "" then power_profile:set(p) end
end

popup("battery-popup", {
  parent = "bar", anchor = "top right",
  visible = show_battery, dismiss_on_outside = true,
  width = 280, height = 340,
  keyboard = true,
  on_key = function(key) if key == "Escape" then close_all_popups() end end,
}, function()
  return col("w-full h-full bg-surface " .. S.popup_pad .. " " .. S.popup_gap, {
    row("items-center " .. S.item_gap, {
      text(T.icon_large .. " text-fg", function()
        return lib.bat_icon(bat_pct:get(), bat_status:get())
      end),
      col("gap-1", {
        lib.header("Battery"),
        lib.caption(function()
          return bat_pct:get() .. "% — " .. bat_status:get()
        end),
      }),
    }),

    lib.separator(),

    lib.slider_row("Brightness", brightness, {
      on_change = function(v)
        brightness:set(math.floor(v))
        exec("brightnessctl s " .. math.floor(v) .. "%")
      end,
    }),

    lib.separator(),

    lib.header("Power Profile"),
    lib.radio_group({
      { label = "Power Saver", value = "power-saver", icon = "󰌪" },
      { label = "Balanced",    value = "balanced",    icon = "󰖩" },
      { label = "Performance", value = "performance", icon = "󰓅" },
    }, power_profile, {
      on_change = function(val)
        exec("powerprofilesctl set " .. val)
      end,
    }),
  })
end)

-- ============================================================================
-- Calendar Popup
-- ============================================================================

local cal_month = signal(tonumber(os.date("%m")))
local cal_year  = signal(tonumber(os.date("%Y")))

popup("calendar", {
  parent = "bar", anchor = "center",
  visible = show_calendar, dismiss_on_outside = true,
  width = 280, height = 300,
  keyboard = true,
  on_key = function(key)
    if key == "Escape" then close_all_popups() end
  end,
}, function()
  return col("w-full h-full bg-surface " .. S.popup_pad .. " " .. S.popup_gap, {
    -- Month/year header with nav
    row("items-center", {
      lib.icon_btn(icons.chevron_l, {
        on_click = function()
          local m = cal_month:get() - 1
          if m < 1 then m = 12; cal_year:set(cal_year:get() - 1) end
          cal_month:set(m)
        end,
      }),
      spacer(),
      text(T.body .. " text-fg font-bold", function()
        local months = {"Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"}
        return months[cal_month:get()] .. " " .. cal_year:get()
      end),
      spacer(),
      lib.icon_btn(icons.chevron_r, {
        on_click = function()
          local m = cal_month:get() + 1
          if m > 12 then m = 1; cal_year:set(cal_year:get() + 1) end
          cal_month:set(m)
        end,
      }),
    }),

    -- Day headers
    row("items-center", {
      text(T.caption .. " text-muted", "Mo"),
      spacer(), text(T.caption .. " text-muted", "Tu"),
      spacer(), text(T.caption .. " text-muted", "We"),
      spacer(), text(T.caption .. " text-muted", "Th"),
      spacer(), text(T.caption .. " text-muted", "Fr"),
      spacer(), text(T.caption .. " text-muted", "Sa"),
      spacer(), text(T.caption .. " text-muted", "Su"),
    }),

    -- Calendar grid
    each(function()
      local m = cal_month:get()
      local y = cal_year:get()
      -- First day of month (1=Sun in Lua, we want 1=Mon)
      local first_wday = tonumber(os.date("%w", os.time({year=y, month=m, day=1})))
      if first_wday == 0 then first_wday = 7 end -- Sun=7
      -- Days in month
      local days_in = tonumber(os.date("%d", os.time({year=y, month=m+1, day=0})))
      local today = tonumber(os.date("%d"))
      local cur_month = tonumber(os.date("%m"))
      local cur_year = tonumber(os.date("%Y"))

      local weeks = {}
      local day = 1
      local week_num = 0
      while day <= days_in do
        week_num = week_num + 1
        local week = {}
        for wd = 1, 7 do
          if (week_num == 1 and wd < first_wday) or day > days_in then
            week[#week+1] = { day = 0, is_today = false }
          else
            local is_today = (day == today and m == cur_month and y == cur_year)
            week[#week+1] = { day = day, is_today = is_today }
            day = day + 1
          end
        end
        weeks[#weeks+1] = week
      end
      return weeks
    end, function(week)
      local cells = {}
      for _, d in ipairs(week) do
        if d.day == 0 then
          cells[#cells+1] = text(T.caption .. " text-muted", "  ")
        elseif d.is_today then
          cells[#cells+1] = text(T.caption .. " text-primary font-bold",
            string.format("%2d", d.day))
        else
          cells[#cells+1] = text(T.caption .. " text-fg",
            string.format("%2d", d.day))
        end
        cells[#cells+1] = spacer()
      end
      -- Remove trailing spacer
      if #cells > 0 then table.remove(cells) end
      return row("items-center", cells)
    end, function(week)
      return tostring(week[1].day) .. "-" .. tostring(week[7].day)
    end),
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
        local comment = c:match("Comment=([^\n]+)") or ""
        local generic = c:match("GenericName=([^\n]+)") or ""
        if name and ex and not c:match("NoDisplay=true") then
          ex = ex:gsub("%%[uUfFdDnNickvm]", ""):gsub("%s+$", "")
          local icon_name = c:match("Icon=([^\n]+)") or ""
          local icon_path = resolve_icon(icon_name)
          local desc = comment ~= "" and comment or generic
          all_apps[#all_apps+1] = {
            name = name, exec = ex, desc = desc,
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

local MAX_VISIBLE = 9

popup("launcher", {
  parent = "bar", anchor = "center",
  visible = show_launcher, dismiss_on_outside = true,
  width = 500, height = 480,
  keyboard = true,
  on_key = function(key, utf8)
    if key == "Escape" then close_all_popups()
    elseif key == "Return" then launch_selected()
    elseif key == "BackSpace" then
      local q = search_query:get()
      if #q > 0 then search_query:set(q:sub(1, #q - 1)); selected_index:set(1) end
    elseif key == "Up" or key == "Tab" then
      local i = selected_index:get()
      if i > 1 then selected_index:set(i - 1) end
    elseif key == "Down" then
      local i = selected_index:get()
      if i < #filtered_apps() and i < MAX_VISIBLE then selected_index:set(i + 1) end
    elseif utf8 and #utf8 == 1 and utf8:byte() >= 32 then
      search_query:set(search_query:get() .. utf8)
      selected_index:set(1)
    end
  end,
}, function()
  return col("w-full h-full bg-surface " .. S.popup_pad .. " " .. S.popup_gap, {
    -- Search bar
    row("bg-base p-3 " .. S.item_gap .. " items-center", {
      text(T.icon_large .. " text-muted", icons.search),
      text(T.body .. " text-fg", function()
        return search_query:get() .. (cursor_blink:get() and "│" or " ")
      end),
      spacer(),
      lib.caption(function()
        return #filtered_apps() .. " apps"
      end),
    }),

    -- Results
    each(function()
      local apps = filtered_apps()
      local sel = selected_index:get()
      local start = math.max(1, sel - MAX_VISIBLE + 1)
      if sel <= MAX_VISIBLE then start = 1 end
      local r = {}
      for i = start, math.min(start + MAX_VISIBLE - 1, #apps) do
        r[#r+1] = {
          idx = i, name = apps[i].name, exec = apps[i].exec,
          desc = apps[i].desc or "",
          icon_path = apps[i].icon_path, icon_fb = apps[i].icon_fallback,
        }
      end
      return r
    end, function(item)
      local icon_node
      if item.icon_path then
        icon_node = image(item.icon_path, 28, 28)
      else
        icon_node = text(T.icon_large .. " text-muted", item.icon_fb or icons.app)
      end

      local h = signal(false)
      local is_selected = function()
        return selected_index:get() == item.idx or h:get()
      end

      local label_nodes = {
        text(function()
          return is_selected() and (T.body .. " text-primary") or (T.body .. " text-fg")
        end, item.name),
      }
      if item.desc ~= "" then
        label_nodes[#label_nodes+1] = text(T.caption .. " text-muted", item.desc)
      end

      return button(function()
        return is_selected()
          and ("px-3 py-2 items-center " .. S.item_gap .. " bg-overlay")
          or ("px-3 py-2 items-center " .. S.item_gap)
      end, {
        on_click = function() selected_index:set(item.idx); launch_selected() end,
        on_hover = function() h:set(true) end,
        on_hover_lost = function() h:set(false) end,
      }, {
        icon_node,
        col("gap-0", label_nodes),
      })
    end, function(item) return item.name end),
  })
end)
