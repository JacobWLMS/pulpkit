# config.json Reference

The `config.json` file in a shell directory controls the dimensions and behavior of
the bar and popup surfaces. All fields are optional and have sensible defaults.

## Location

```
poc/shells/{shell-name}/config.json
```

## Full Example

```json
{
  "bar_height": 40,
  "bar_position": "top",
  "popup_width": 380,
  "popup_height": 500,
  "popup_backdrop": "dim",
  "popup_dim_opacity": 0.25
}
```

## Fields

### `bar_height`

| | |
|---|---|
| **Type** | `integer` |
| **Default** | `40` |
| **Unit** | Pixels |

Height of the bar layer-shell surface.

```json
{"bar_height": 36}
```

---

### `bar_position`

| | |
|---|---|
| **Type** | `string` |
| **Default** | `"top"` |
| **Valid values** | `"top"`, `"bottom"` |

Which screen edge the bar is anchored to.

```json
{"bar_position": "bottom"}
```

!!! note
    When set to `"bottom"`, the bar is anchored to the bottom edge and the exclusive
    zone reserves space at the bottom of the screen.

---

### `popup_width`

| | |
|---|---|
| **Type** | `integer` |
| **Default** | `380` |
| **Unit** | Pixels |

Width of the popup layer-shell surface.

```json
{"popup_width": 560}
```

---

### `popup_height`

| | |
|---|---|
| **Type** | `integer` |
| **Default** | `500` |
| **Unit** | Pixels |

Height of the popup layer-shell surface.

```json
{"popup_height": 600}
```

---

### `popup_backdrop`

| | |
|---|---|
| **Type** | `string` |
| **Default** | `"dim"` |
| **Valid values** | `"dim"`, `"blur"`, `"none"` |

Backdrop style when a popup is open. Controls whether a semi-transparent overlay
appears behind the popup.

- **`"dim"`** — Dark semi-transparent overlay behind the popup. Clicking the backdrop
  dismisses the popup.
- **`"blur"`** — Same as dim (blur is compositor-dependent).
- **`"none"`** — No backdrop. Popup floats without an overlay.

```json
{"popup_backdrop": "none"}
```

---

### `popup_dim_opacity`

| | |
|---|---|
| **Type** | `float` |
| **Default** | `0.25` |
| **Range** | `0.0` to `1.0` |

Opacity of the dim backdrop. Only effective when `popup_backdrop` is `"dim"`.

- `0.0` — fully transparent
- `1.0` — fully opaque black

```json
{"popup_dim_opacity": 0.4}
```

## Minimal Config

If you are happy with all defaults, you can use an empty object:

```json
{}
```

Or omit `config.json` entirely -- the runtime uses defaults for all fields.

## Real-World Examples

### Compact bar with large popup

```json
{
  "bar_height": 32,
  "popup_width": 560,
  "popup_height": 560
}
```

### Bottom bar without backdrop

```json
{
  "bar_height": 44,
  "bar_position": "bottom",
  "popup_backdrop": "none"
}
```

### Heavy dim effect

```json
{
  "popup_backdrop": "dim",
  "popup_dim_opacity": 0.6
}
```
