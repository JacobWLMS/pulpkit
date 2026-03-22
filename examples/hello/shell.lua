-- Signals for state
local show_volume = signal(false)
local volume = signal(75)
local muted = signal(false)
local time_str = signal(os.date("%H:%M:%S"))

-- Update the clock every second
set_interval(function()
  time_str:set(os.date("%H:%M:%S"))
end, 1000)

-- Main bar
window("bar", {
  monitor = "all",
  anchor = "top",
  exclusive = true,
  height = 36,
}, function(ctx)
  return row("w-full h-9 bg-base px-2 items-center gap-4", {
    -- Left: branding
    text("text-sm text-primary font-bold", "Pulpkit"),

    spacer(),

    -- Center: live clock (reactive — function re-evaluated each render)
    text("text-sm text-fg", function()
      return time_str:get()
    end),

    spacer(),

    -- Right: volume button (reactive hover via signal)
    (function()
      local hovered = signal(false)
      return button(function()
        return hovered:get() and "px-2 py-1 rounded bg-surface" or "px-2 py-1 rounded"
      end, {
        on_click = function()
          show_volume:set(not show_volume:get())
        end,
        on_hover = function() hovered:set(true) end,
        on_hover_lost = function() hovered:set(false) end,
      }, {
        text("text-sm text-fg", "Vol"),
      })
    end)(),

    text("text-xs text-muted", "Hello World"),
  })
end)

-- Volume popup
popup("volume-popup", {
  parent = "bar",
  anchor = "top right",
  offset = { x = -8, y = 4 },
  visible = show_volume,
  dismiss_on_outside = true,
  width = 260,
  height = 140,
}, function()
  return col("bg-surface rounded-lg p-4 gap-4", {
    text("text-sm font-bold text-fg", "Volume"),
    slider("w-full accent-primary", {
      value = volume,
      on_change = function(v) volume:set(v) end,
      min = 0,
      max = 100,
    }),
    row("items-center gap-2", {
      text("text-xs text-dim", "Mute"),
      toggle("accent-primary", {
        checked = muted,
        on_change = function(v) muted:set(v) end,
      }),
    }),
  })
end)
