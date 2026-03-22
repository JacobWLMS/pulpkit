window("bar", {
  monitor = "all",
  anchor = "top",
  exclusive = true,
  height = 36,
}, function(ctx)
  return row("w-full h-9 bg-base px-2 items-center", {
    text("text-sm text-primary font-bold", "Pulpkit"),
    spacer(),
    text("text-sm text-fg", os.date("%H:%M")),
    spacer(),
    text("text-xs text-muted", "Hello World"),
  })
end)
