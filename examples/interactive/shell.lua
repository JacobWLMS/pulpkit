function init()
  return {
    time = os.date("%H:%M"),
    user = "...",
    host = "...",
    ticks = 0,
  }
end

function update(state, msg)
  if msg.type == "tick" then
    state.time = os.date("%H:%M")
    state.ticks = state.ticks + 1
  elseif msg.type == "user" then
    state.user = msg.data or "?"
  elseif msg.type == "host" then
    state.host = msg.data or "?"
  end
  return state
end

function view(state)
  return {
    window("bar", { anchor = "top", height = 32, exclusive = true, monitor = "all" },
      row({ style = "bg-base w-full h-full items-center px-3 gap-3" },
        -- Left: branding
        text({ style = "text-sm text-primary font-bold" }, ""),
        text({ style = "text-xs text-primary" }, "pulpkit"),
        -- Center spacer
        spacer(),
        -- Right: system info
        text({ style = "text-xs text-muted" }, state.user .. "@" .. state.host),
        text({ style = "text-xs text-fg" }, "  " .. state.time)
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
