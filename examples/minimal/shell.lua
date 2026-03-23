function init()
  return {
    count = 0,
    time = os.date("%H:%M:%S"),
  }
end

function update(state, msg)
  if msg.type == "tick" then
    state.count = state.count + 1
    state.time = os.date("%H:%M:%S")
  end
  return state
end

function view(state)
  return {
    window("bar", { anchor = "top", height = 36, exclusive = true, monitor = "all" },
      row({ style = "bg-base h-full items-center px-3 gap-2" },
        text({ style = "text-sm text-primary font-bold" }, "pulpkit v3"),
        spacer(),
        text({ style = "text-xs text-muted" }, state.time),
        text({ style = "text-xs text-fg" }, "ticks: " .. state.count)
      )
    )
  }
end

function subscribe(state)
  return {
    interval(1000, "tick"),
  }
end
