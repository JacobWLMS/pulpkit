function init()
  return {
    time = os.date("%H:%M"),
    user = "...",
    host = "...",
    vol = 50,
    popup_open = false,
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
  elseif msg.type == "ipc_cmd" then
    -- IPC commands can be anything — for now just log
  end
  return state
end

function view(state)
  local surfaces = {
    window("bar", { anchor = "top", height = 44, exclusive = true, monitor = "all" },
      row({ style = "bg-base w-full h-full items-center px-4 gap-4" },
        -- Left: branding
        text({ style = "text-lg text-primary font-bold" }, "\u{f313}"),
        text({ style = "text-base text-primary font-bold" }, "pulpkit v3"),

        spacer(),

        -- Volume button (toggles popup)
        button({ on_click = msg("toggle_popup"), style = "p-2 hover:bg-surface rounded" },
          text({ style = "text-base text-fg" }, "\u{f028} " .. math.floor(state.vol))
        ),

        -- System info
        text({ style = "text-sm text-muted" }, state.user .. "@" .. state.host),
        text({ style = "text-base text-fg" }, "\u{f017} " .. state.time)
      )
    )
  }

  -- Popup: volume control
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
    interval(60000, "tick"),
    exec("whoami", "user"),
    exec("hostname", "host"),
    ipc("ipc_cmd"),
  }
end
