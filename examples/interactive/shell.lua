-- Pulpkit v3 interactive demo — workspace dots, volume popup, system info

function init()
  return {
    time = os.date("%H:%M"),
    user = "...",
    host = "...",
    vol = 50,
    popup_open = false,
    workspaces = {},
  }
end

function update(state, msg)
  if msg.type == "tick" then
    state.time = os.date("%H:%M")
  elseif msg.type == "user" then
    state.user = msg.data or "?"
  elseif msg.type == "host" then
    state.host = msg.data or "?"
  elseif msg.type == "set_vol" then
    state.vol = msg.data or state.vol
  elseif msg.type == "toggle_popup" then
    state.popup_open = not state.popup_open
  elseif msg.type == "dismiss" then
    state.popup_open = false
  elseif msg.type == "workspaces" then
    -- Parse JSON workspace list from niri
    local ws = {}
    if msg.data then
      -- Simple JSON array parsing for workspace objects
      for id, idx, active in msg.data:gmatch('"id":(%d+).-"idx":(%d+).-"is_focused":(.-)[,}]') do
        table.insert(ws, {
          id = tostring(id),
          idx = tonumber(idx),
          active = active:find("true") ~= nil,
        })
      end
    end
    if #ws > 0 then
      state.workspaces = ws
    end
  end
  return state
end

function view(state)
  local surfaces = {
    window("bar", { anchor = "top", height = 44, exclusive = true, monitor = "all" },
      row({ style = "bg-base w-full h-full items-center px-4 gap-4" },
        -- Left: workspace dots
        each(state.workspaces, "id", function(ws)
          return button({
            style = ws.active and "bg-primary rounded-full p-1" or "bg-muted rounded-full p-1",
          },
            text({ style = "text-xs text-base" }, tostring(ws.idx))
          )
        end),

        -- Branding
        text({ style = "text-base text-primary font-bold" }, "\u{f313} pulpkit"),

        spacer(),

        -- Volume
        button({ on_click = msg("toggle_popup"), style = "p-2 hover:bg-surface rounded" },
          text({ style = "text-base text-fg" }, "\u{f028} " .. math.floor(state.vol))
        ),

        -- System info
        text({ style = "text-sm text-muted" }, state.user .. "@" .. state.host),
        text({ style = "text-base text-fg" }, "\u{f017} " .. state.time)
      )
    )
  }

  if state.popup_open then
    table.insert(surfaces, popup("vol", {
      anchor = "top right", width = 280, height = 120,
      dismiss_on_outside = true,
    },
      col({ style = "bg-surface w-full h-full rounded-lg p-4 gap-3" },
        row({ style = "items-center gap-2" },
          text({ style = "text-base text-muted" }, "\u{f028}"),
          text({ style = "text-base text-fg font-bold" }, "Volume")
        ),
        slider({ value = state.vol, min = 0, max = 100, on_change = msg("set_vol") }),
        text({ style = "text-sm text-muted" }, math.floor(state.vol) .. "%")
      )
    ))
  end

  return surfaces
end

function subscribe(state)
  return {
    interval(1000, "tick"),
    exec("whoami", "user"),
    exec("hostname", "host"),
    exec("niri msg -j workspaces", "workspaces"),
    ipc("ipc_cmd"),
  }
end
