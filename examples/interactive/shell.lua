function init()
  return {
    time = os.date("%H:%M"),
    user = "...",
    host = "...",
    vol = 50,
    dark = true,
    clicks = 0,
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
  elseif msg.type == "toggle_dark" then
    state.dark = not state.dark
  elseif msg.type == "clicked" then
    state.clicks = state.clicks + 1
  end
  return state
end

function view(state)
  return {
    window("bar", { anchor = "top", height = 32, exclusive = true, monitor = "all" },
      row({ style = "bg-base w-full h-full items-center px-3 gap-3" },
        -- Left: branding
        text({ style = "text-sm text-primary font-bold" }, "\u{f313}"),
        text({ style = "text-xs text-primary" }, "pulpkit v3"),

        spacer(),

        -- Volume text
        text({ style = "text-xs text-muted" }, "vol:" .. math.floor(state.vol)),

        -- System info
        text({ style = "text-xs text-muted" }, state.user .. "@" .. state.host),
        text({ style = "text-xs text-fg" }, "\u{f017} " .. state.time)
      )
    )
  }
end

function subscribe(state)
  return {
    interval(60000, "tick"),
    exec("whoami", "user"),
    exec("hostname", "host"),
  }
end
