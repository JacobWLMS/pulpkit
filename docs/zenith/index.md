# Zenith

Zenith is the reference shell for Pulpkit — a fully-featured desktop shell that showcases every capability of the framework. It ships as the default when you run `pulpkit-webshell-poc zenith` and serves as both a daily-driver shell and a worked example for shell developers.

## Design Philosophy

Zenith follows a **translucent, minimal, icon-first** design language:

- **Nerd Font driven** — Every indicator, tile, and button uses Nerd Font glyphs instead of text labels. The bar reads as a row of icons, not a wall of words.
- **Frosted glass** — The bar uses `backdrop-filter: blur(12px)` over an 85% opacity background, giving a translucent frosted-glass look that lets your wallpaper bleed through.
- **Contextual visibility** — Indicators only appear when relevant. No VPN icon unless a VPN is active. No gaming indicator unless GameMode is on. No media controls unless something is playing.
- **Catppuccin palette** — Default theme is Catppuccin Mocha. All 12 themes use the same CSS variable system, so switching is instant.

## Feature Highlights

### Bar

- Frosted glass bar with backdrop blur, positioned at the top of the screen
- Workspace switcher with numbered dots (active workspace highlighted in accent color)
- Taskbar with app icons and focus indicators
- Active window title (truncated, hover for full)
- Now-playing media ticker with inline transport controls (prev/play/next)
- Centered clock (click to open calendar)
- CPU and memory mini-stats
- Notification bell with unread badge
- System tray (StatusNotifierItem icons)
- Contextual indicators: VPN, DND, gaming/GPU temp, caffeine, mic
- Volume, WiFi, battery, and power icons in the right status area

### Popups

10 popup panels, each opened by clicking the relevant bar element. All popups appear with a dim transparent backdrop (`rgba(0,0,0,0.25)`) that dismisses on click.

| Panel | Opens from | Purpose |
|-------|-----------|---------|
| [Quick Settings](#) | Volume / settings icon | Toggle tiles, sliders, battery |
| [WiFi](#) | Network icon | Scan and connect to networks |
| [Power](#) | Power icon | Session controls, user info |
| [Launcher](#) | Launcher button | App search with keyboard nav |
| [Audio Mixer](#) | Mixer tile in Quick Settings | Per-app volume, device selector |
| [Calendar/Weather](#) | Clock | Month calendar, weather, events |
| [Notifications](#) | Bell icon | Notification list, DND, clear all |
| [System Monitor](#) | CPU/mem stats | CPU, memory, disk, GPU, processes |
| [Media](#) | Now-playing ticker | Album art, transport controls |
| [Settings App](#) | Config tile in Quick Settings | 11-page settings with sidebar nav |

See [Panels](panels.md) for full documentation of each panel.

### Toast Notifications

Zenith includes a built-in notification daemon. Toast notifications slide in from the top-right with a CSS animation, show the app icon, summary, and body, then auto-dismiss after a timeout. All 12 themes are supported in the toast surface.

### Lock Screen

A full-screen lock surface activated by `loginctl lock-session` or the lock button in the Power panel. Displays a large centered clock, date, and a PAM-authenticated password input. Failed attempts trigger a shake animation and error message.

### Themes

12 Catppuccin-family themes, switchable at runtime via the Appearance page in Settings or via command:

| Theme | Family |
|-------|--------|
| `mocha` | Catppuccin (default) |
| `macchiato` | Catppuccin |
| `frappe` | Catppuccin |
| `latte` | Catppuccin (light) |
| `tokyonight` | Tokyo Night |
| `nord` | Nord |
| `gruvbox` | Gruvbox |
| `rosepine` | Rose Pine |
| `onedark` | One Dark |
| `dracula` | Dracula |
| `solarized` | Solarized Dark |
| `flexoki` | Flexoki |

Each theme defines 15 CSS variables (`--bg`, `--bg-surface`, `--bg-overlay`, `--fg`, `--fg-muted`, `--fg-dim`, `--accent`, `--blue`, `--green`, `--red`, `--yellow`, `--peach`, `--teal`, `--pink`, `--mauve`, `--text-on-color`) that cascade through every surface.

## Source Repository

Zenith lives at [github.com/JacobWLMS/zenith](https://github.com/JacobWLMS/zenith).

## Installation

Clone Zenith into the Pulpkit shells directory and run it:

```bash
# Clone into the shells directory
git clone https://github.com/JacobWLMS/zenith.git poc/shells/zenith

# Run Zenith
pulpkit-webshell-poc zenith
```

Or if running from a source build:

```bash
./target/release/pulpkit-webshell-poc zenith
```

You should see a frosted-glass bar appear at the top of your screen. Click the clock to open the calendar, or click any status icon to open its panel.

## File Structure

```
poc/shells/zenith/
  config.json            # Shell configuration (bar height, popup size, backdrop)
  bar.html               # Bar surface — workspaces, taskbar, clock, status
  popup.html             # Popup surface — all 10 panels
  toast.html             # Toast notification surface
  lock.html              # Lock screen surface
```

All rendering logic is self-contained in each HTML file — no external JS build step, no bundler, no node_modules.
