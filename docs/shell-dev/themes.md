# Theming

Pulpkit ships with 12 color themes implemented as CSS custom properties. Themes are
applied via the `data-theme` attribute on the `<html>` element. Shells can switch
themes at runtime with a single command.

## CSS Variables

All themes define the same set of CSS custom properties:

| Variable | Purpose |
|---|---|
| `--bg` | Deepest background (bar, body) |
| `--bg-surface` | Elevated surface (popup panels, cards) |
| `--bg-overlay` | Overlay/hover backgrounds |
| `--fg` | Primary text |
| `--fg-muted` | Secondary/muted text |
| `--fg-dim` | Tertiary/disabled text |
| `--accent` | Primary accent color |
| `--blue` | Blue accent |
| `--green` | Green accent (success, online) |
| `--red` | Red accent (error, critical) |
| `--yellow` | Yellow accent (warning) |
| `--peach` | Peach/orange accent |
| `--teal` | Teal accent |
| `--pink` | Pink accent |
| `--mauve` | Mauve/purple accent |
| `--text-on-color` | Text color for use on accent backgrounds |

### Usage in CSS

```css
body {
  background: var(--bg);
  color: var(--fg);
}

.card {
  background: var(--bg-surface);
  border: 1px solid var(--bg-overlay);
}

.label-muted {
  color: var(--fg-muted);
}

.badge-accent {
  background: var(--accent);
  color: var(--text-on-color);
}

.battery-low {
  color: var(--red);
}

.battery-charging {
  color: var(--green);
}
```

## Applying Themes in JavaScript

In your `updateState()` function, apply the theme from state:

```js
function updateState(s) {
  if (s.theme) {
    document.documentElement.setAttribute('data-theme', s.theme);
  }
  // ... rest of rendering
}
```

This sets `<html data-theme="gruvbox">` (or whichever theme is active), which
activates the matching CSS selector `[data-theme="gruvbox"]`.

## Switching Themes

Send the `set_theme` command from JavaScript:

```js
send({cmd: 'set_theme', data: 'tokyonight'});
```

On the next state push, `s.theme` will be `"tokyonight"` and `updateState()` will
apply it.

## Available Themes

### Catppuccin Family

=== "Mocha"
    ```css
    [data-theme="mocha"] {
      --bg: #181825; --bg-surface: #1e1e2e; --bg-overlay: #313244;
      --fg: #cdd6f4; --fg-muted: #a6adc8; --fg-dim: #585b70;
      --accent: #cba6f7; --blue: #89b4fa; --green: #a6e3a1;
      --red: #f38ba8; --yellow: #f9e2af; --peach: #fab387;
      --teal: #94e2d5; --pink: #f2cdcd; --mauve: #cba6f7;
      --text-on-color: #181825;
    }
    ```

=== "Macchiato"
    ```css
    [data-theme="macchiato"] {
      --bg: #1e2030; --bg-surface: #24273a; --bg-overlay: #363a4f;
      --fg: #cad3f5; --fg-muted: #a5adcb; --fg-dim: #5b6078;
      --accent: #c6a0f6; --blue: #8aadf4; --green: #a6da95;
      --red: #ed8796; --yellow: #eed49f; --peach: #f5a97f;
      --teal: #8bd5ca; --pink: #f0c6c6; --mauve: #c6a0f6;
      --text-on-color: #1e2030;
    }
    ```

=== "Frappe"
    ```css
    [data-theme="frappe"] {
      --bg: #292c3c; --bg-surface: #303446; --bg-overlay: #414559;
      --fg: #c6d0f5; --fg-muted: #a5adce; --fg-dim: #626880;
      --accent: #ca9ee6; --blue: #8caaee; --green: #a6d189;
      --red: #e78284; --yellow: #e5c890; --peach: #ef9f76;
      --teal: #81c8be; --pink: #eebebe; --mauve: #ca9ee6;
      --text-on-color: #292c3c;
    }
    ```

=== "Latte"
    ```css
    [data-theme="latte"] {
      --bg: #e6e9ef; --bg-surface: #eff1f5; --bg-overlay: #ccd0da;
      --fg: #4c4f69; --fg-muted: #6c6f85; --fg-dim: #9ca0b0;
      --accent: #8839ef; --blue: #1e66f5; --green: #40a02b;
      --red: #d20f39; --yellow: #df8e1d; --peach: #fe640b;
      --teal: #179299; --pink: #ea76cb; --mauve: #8839ef;
      --text-on-color: #eff1f5;
    }
    ```

### Community Themes

=== "Tokyo Night"
    ```css
    [data-theme="tokyonight"] {
      --bg: #1a1b26; --bg-surface: #16161e; --bg-overlay: #24283b;
      --fg: #c0caf5; --fg-muted: #565f89; --fg-dim: #414868;
      --accent: #7aa2f7; --blue: #7aa2f7; --green: #9ece6a;
      --red: #f7768e; --yellow: #e0af68; --peach: #ff9e64;
      --teal: #73daca; --pink: #bb9af7; --mauve: #bb9af7;
      --text-on-color: #1a1b26;
    }
    ```

=== "Nord"
    ```css
    [data-theme="nord"] {
      --bg: #2e3440; --bg-surface: #3b4252; --bg-overlay: #434c5e;
      --fg: #eceff4; --fg-muted: #d8dee9; --fg-dim: #4c566a;
      --accent: #88c0d0; --blue: #81a1c1; --green: #a3be8c;
      --red: #bf616a; --yellow: #ebcb8b; --peach: #d08770;
      --teal: #8fbcbb; --pink: #b48ead; --mauve: #b48ead;
      --text-on-color: #2e3440;
    }
    ```

=== "Gruvbox"
    ```css
    [data-theme="gruvbox"] {
      --bg: #1d2021; --bg-surface: #282828; --bg-overlay: #3c3836;
      --fg: #ebdbb2; --fg-muted: #a89984; --fg-dim: #665c54;
      --accent: #d79921; --blue: #458588; --green: #98971a;
      --red: #cc241d; --yellow: #d79921; --peach: #d65d0e;
      --teal: #689d6a; --pink: #b16286; --mauve: #b16286;
      --text-on-color: #1d2021;
    }
    ```

=== "Rose Pine"
    ```css
    [data-theme="rosepine"] {
      --bg: #191724; --bg-surface: #1f1d2e; --bg-overlay: #26233a;
      --fg: #e0def4; --fg-muted: #908caa; --fg-dim: #6e6a86;
      --accent: #c4a7e7; --blue: #9ccfd8; --green: #31748f;
      --red: #eb6f92; --yellow: #f6c177; --peach: #f6c177;
      --teal: #9ccfd8; --pink: #ebbcba; --mauve: #c4a7e7;
      --text-on-color: #191724;
    }
    ```

=== "One Dark"
    ```css
    [data-theme="onedark"] {
      --bg: #1e2127; --bg-surface: #282c34; --bg-overlay: #353b45;
      --fg: #abb2bf; --fg-muted: #7f848e; --fg-dim: #545862;
      --accent: #c678dd; --blue: #61afef; --green: #98c379;
      --red: #e06c75; --yellow: #e5c07b; --peach: #d19a66;
      --teal: #56b6c2; --pink: #c678dd; --mauve: #c678dd;
      --text-on-color: #1e2127;
    }
    ```

=== "Dracula"
    ```css
    [data-theme="dracula"] {
      --bg: #21222c; --bg-surface: #282a36; --bg-overlay: #44475a;
      --fg: #f8f8f2; --fg-muted: #bfbfbf; --fg-dim: #6272a4;
      --accent: #bd93f9; --blue: #8be9fd; --green: #50fa7b;
      --red: #ff5555; --yellow: #f1fa8c; --peach: #ffb86c;
      --teal: #8be9fd; --pink: #ff79c6; --mauve: #bd93f9;
      --text-on-color: #282a36;
    }
    ```

=== "Solarized"
    ```css
    [data-theme="solarized"] {
      --bg: #002b36; --bg-surface: #073642; --bg-overlay: #0a4050;
      --fg: #839496; --fg-muted: #657b83; --fg-dim: #586e75;
      --accent: #268bd2; --blue: #268bd2; --green: #859900;
      --red: #dc322f; --yellow: #b58900; --peach: #cb4b16;
      --teal: #2aa198; --pink: #d33682; --mauve: #6c71c4;
      --text-on-color: #002b36;
    }
    ```

=== "Flexoki"
    ```css
    [data-theme="flexoki"] {
      --bg: #100f0f; --bg-surface: #1c1b1a; --bg-overlay: #282726;
      --fg: #cecdc3; --fg-muted: #878580; --fg-dim: #575653;
      --accent: #da702c; --blue: #4385be; --green: #879a39;
      --red: #d14d41; --yellow: #d0a215; --peach: #da702c;
      --teal: #3aa99f; --pink: #ce5d97; --mauve: #8b7ec8;
      --text-on-color: #100f0f;
    }
    ```

## Creating Custom Themes

Add a new `[data-theme="your-theme"]` selector to your `theme.css` defining all
16 variables:

```css
[data-theme="cyberpunk"] {
  --bg: #0a0a0f;
  --bg-surface: #12121a;
  --bg-overlay: #1a1a2e;
  --fg: #e0e0ff;
  --fg-muted: #8888aa;
  --fg-dim: #444466;
  --accent: #ff00ff;
  --blue: #00ccff;
  --green: #00ff88;
  --red: #ff0044;
  --yellow: #ffcc00;
  --peach: #ff8800;
  --teal: #00ffcc;
  --pink: #ff44aa;
  --mauve: #aa44ff;
  --text-on-color: #0a0a0f;
}
```

Then switch to it:

```js
send({cmd: 'set_theme', data: 'cyberpunk'});
```

!!! tip "Light themes"
    For light themes, swap the relationship: `--bg` should be light, `--fg` should
    be dark, and `--text-on-color` should be the light background color. See the
    `latte` theme for reference.

## Theme File Location

Theme CSS can live in two places:

1. **Shell-level** `theme.css` — in the shell directory, linked via `<link rel="stylesheet" href="theme.css">`
2. **Built-in** — the runtime injects the default theme template if no `theme.css` is provided

The default `:root` selector sets `mocha` as the fallback theme when no `data-theme`
attribute is set.
