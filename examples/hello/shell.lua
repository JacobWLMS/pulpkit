-- Signals for state
local show_volume = signal(false)
local volume = signal(75)
local muted = signal(false)

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

    -- Center: time
    text("text-sm text-fg", os.date("%H:%M")),

    spacer(),

    -- Right: volume button + mute toggle
    button("px-2 py-1 rounded hover:bg-surface", {
      on_click = function()
        show_volume:set(not show_volume:get())
      end,
    }, {
      text("text-sm text-fg", "Vol"),
    }),

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
