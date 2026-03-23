# Customization

Zenith is designed to be modified. Every visual element is controlled by CSS variables and HTML structure that you can edit directly. There is no build step — save a file and hot-reload.

## Changing Themes

### At runtime

Send a `set_theme` command from any surface:

```javascript
send({ cmd: 'set_theme', data: 'nord' });
```

Or use the **Appearance** page in the Settings App panel (Config tile in Quick Settings).

### Available themes

| Theme | Accent color |
|-------|-------------|
| `mocha` | `#cba6f7` (mauve) |
| `macchiato` | `#c6a0f6` (mauve) |
| `frappe` | `#ca9ee6` (mauve) |
| `latte` | `#8839ef` (mauve) |
| `tokyonight` | `#7aa2f7` (blue) |
| `nord` | `#88c0d0` (frost) |
| `gruvbox` | `#d79921` (yellow) |
| `rosepine` | `#c4a7e7` (iris) |
| `onedark` | `#c678dd` (purple) |
| `dracula` | `#bd93f9` (purple) |
| `solarized` | `#268bd2` (blue) |
| `flexoki` | `#da702c` (orange) |

### Adding a custom theme

Add a new `[data-theme="yourtheme"]` block to the `<style>` section in `bar.html`, `popup.html`, `toast.html`, and `lock.html`:

```css
[data-theme="yourtheme"] {
  --bg: #1a1a2e;
  --bg-surface: #16213e;
  --bg-overlay: #0f3460;
  --fg: #e0e0e0;
  --fg-muted: #a0a0a0;
  --fg-dim: #606060;
  --accent: #e94560;
  --blue: #5089c6;
  --green: #50c878;
  --red: #e94560;
  --yellow: #f0c040;
  --peach: #e07040;
  --teal: #40c0a0;
  --pink: #e06090;
  --mauve: #9060c0;
  --text-on-color: #1a1a2e;
}
```

Then register it in the `THEME_ACCENTS` object in `popup.html` so it appears in the Appearance grid:

```javascript
var THEME_ACCENTS = {
  // ... existing themes ...
  yourtheme: '#e94560'
};
```

---

## Modifying the Bar Layout

The bar is a three-section flex layout in `bar.html`:

```
[.left]                    [.center]                   [.right]
launcher | ws | taskbar    clock                       stats | indicators | tray
```

### Reordering elements

Move elements between `.left`, `.center`, and `.right` divs. Each element is a self-contained `<span>` or `<div>`. For example, to move the clock to the left:

```html
<div class="left">
  <span id="clock" onclick="send({cmd:'popup',data:'calendar'})"></span>
  <div class="divider"></div>
  <div id="workspaces"></div>
  <!-- ... -->
</div>
<div class="center">
  <!-- empty, or put something else here -->
</div>
```

### Removing elements

Delete the HTML element and its corresponding render function call in `updateState()`. For example, to remove the taskbar, delete the `<div id="taskbar"></div>` element and the `renderTaskbar(s)` call.

### Bar height and position

Edit `config.json`:

```json
{
  "bar_height": 36,
  "bar_position": "top"
}
```

`bar_position` accepts `"top"` or `"bottom"`.

---

## Adding and Removing Indicators

Contextual indicators live in the `#indicators` div in the bar. They are rendered conditionally in the `updateState()` function based on state fields.

### Existing indicators

| Indicator | Shown when | State field |
|-----------|-----------|-------------|
| VPN | VPN is connected | `s.vpn_name` |
| DND | Do Not Disturb is on | `s.dnd` |
| Gaming + GPU temp | GameMode is active | `s.gamemode_active`, `s.gpu_temp` |
| Caffeine | Idle inhibit is active | `s.caffeine_active` |
| Mic | Microphone is unmuted/active | `s.mic_mute` (shown when not muted) |

### Adding a custom indicator

In `bar.html`, find the indicator rendering section in `updateState()` and add your logic:

```javascript
// Inside the indicator building block:
if (s.your_state_field) {
  html += '<span class="ind accent" title="Your Label">YOUR_ICON</span>';
}
```

Use Nerd Font glyphs for icons. Find glyphs at [nerdfonts.com/cheat-sheet](https://www.nerdfonts.com/cheat-sheet).

### Removing an indicator

Delete or comment out the corresponding `if` block in the indicator rendering section.

---

## Adding Custom Panels

### Step 1: Add the panel div

In `popup.html`, add a new panel container alongside the existing ones:

```html
<div id="panel-mypanel" class="panel"></div>
```

### Step 2: Write the render function

```javascript
function renderMyPanel(s) {
  if (s.popup !== 'mypanel') return;
  var el = document.getElementById('panel-mypanel');
  if (!el) return;

  if (!el.dataset.built) {
    el.dataset.built = '1';
    el.innerHTML =
      '<div class="panel-title">' +
        '<span class="panel-title-icon">ICON</span> My Panel' +
      '</div>' +
      '<div id="mypanel-content"></div>';
  }

  // Update dynamic content
  setText('mypanel-content', s.some_field || 'No data');
}
```

### Step 3: Call it from updateState

In the `updateState()` function in `popup.html`, add:

```javascript
renderMyPanel(s);
```

### Step 4: Add a trigger

In `bar.html`, add a clickable element that opens the panel:

```javascript
send({ cmd: 'popup', data: 'mypanel' });
```

---

## Changing the Toast Style

Toast notifications are rendered in `toast.html`. The toast layout, animation, and timing are all in that file.

### Animation

The default slide-in animation is defined in the CSS `@keyframes` block. Modify it to change entry/exit behavior:

```css
@keyframes toastIn {
  from { opacity: 0; transform: translateX(100%); }
  to { opacity: 1; transform: translateX(0); }
}
```

### Positioning

Toasts appear in the top-right by default. Change the container's CSS `justify-content` and `align-items` to reposition.

### Auto-dismiss timing

The toast timeout is controlled in the JavaScript. Look for the `setTimeout` call that removes toast elements and adjust the delay (in milliseconds).

---

## config.json Options

Zenith reads its configuration from `poc/shells/zenith/config.json`:

```json
{
  "bar_height": 36,
  "bar_position": "top",
  "popup_width": 560,
  "popup_height": 560,
  "popup_backdrop": "dim",
  "popup_dim_opacity": 0.25
}
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bar_height` | integer | `36` | Bar height in pixels |
| `bar_position` | string | `"top"` | Bar position: `"top"` or `"bottom"` |
| `popup_width` | integer | `560` | Popup panel width in pixels |
| `popup_height` | integer | `560` | Popup panel height in pixels |
| `popup_backdrop` | string | `"dim"` | Backdrop mode: `"none"`, `"dim"`, or `"opaque"` |
| `popup_dim_opacity` | float | `0.25` | Backdrop opacity when `popup_backdrop` is `"dim"` (0.0 - 1.0) |

### Backdrop modes

- **`"dim"`** — Semi-transparent black backdrop. Desktop is visible but darkened. Clicking the backdrop dismisses the popup.
- **`"opaque"`** — Solid black backdrop. Desktop is fully hidden behind the popup.
- **`"none"`** — No backdrop surface. Popup floats over the desktop without any overlay. Dismiss via Escape key only.

---

## CSS Variable Overrides

Every color in Zenith is a CSS variable. You can override any of them in the theme block or add a `:root` override that applies regardless of theme.

### Available variables

| Variable | Purpose | Example (Mocha) |
|----------|---------|-----------------|
| `--bg` | Window/panel background | `#181825` |
| `--bg-surface` | Elevated surface (tiles, inputs) | `#1e1e2e` |
| `--bg-overlay` | Borders, dividers, hover backgrounds | `#313244` |
| `--fg` | Primary text | `#cdd6f4` |
| `--fg-muted` | Secondary text, labels | `#a6adc8` |
| `--fg-dim` | Tertiary text, placeholders | `#585b70` |
| `--accent` | Accent color (active states, highlights) | `#cba6f7` |
| `--blue` | Info, links | `#89b4fa` |
| `--green` | Success, connected, charging | `#a6e3a1` |
| `--red` | Error, danger, critical | `#f38ba8` |
| `--yellow` | Warning states | `#f9e2af` |
| `--peach` | Secondary accent | `#fab387` |
| `--teal` | Tertiary accent | `#94e2d5` |
| `--pink` | Decorative | `#f2cdcd` |
| `--mauve` | Decorative (often same as accent) | `#cba6f7` |
| `--text-on-color` | Text on accent-colored backgrounds | `#181825` |

### Example: custom bar opacity

Override the bar background to be more or less transparent:

```css
.bar {
  background: color-mix(in srgb, var(--bg) 70%, transparent); /* more transparent */
}
```

### Example: disable backdrop blur

```css
.bar {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  background: var(--bg); /* solid fallback */
}
```

### Font stack

Zenith uses `Inter` for UI text and `JetBrainsMono Nerd Font` for icons. To change the UI font, override the `font-family` on `body` in each HTML file. The Nerd Font is required for all icons — do not remove it from `.nf`, `.tile-icon`, and other icon-specific selectors.
