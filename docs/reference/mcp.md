# MCP Server Reference

Pulpkit includes an MCP (Model Context Protocol) server that enables AI-assisted
shell development. The MCP server communicates with the running shell via a Unix
socket IPC, allowing tools like Claude Code to inspect state, hot-reload HTML, evaluate
JavaScript, and iterate on shell designs in real time.

## Connecting the MCP Server

The MCP server binary is `pulpkit-mcp`. It communicates via stdin/stdout using the
MCP JSON-RPC protocol.

### Claude Code Configuration

Add to your Claude Code MCP settings:

```json
{
  "mcpServers": {
    "pulpkit": {
      "command": "/path/to/pulpkit-mcp"
    }
  }
}
```

### Prerequisites

The MCP server connects to the running Pulpkit shell via a Unix socket at:

```
$XDG_RUNTIME_DIR/pulpkit.sock
```

Override with the `PULPKIT_SOCK` environment variable if needed.

!!! warning "Shell must be running"
    Most MCP tools require the Pulpkit shell to be running. Tools that interact
    with the live shell (get_state, hot_reload, eval_js, set_mock_state) will
    return an error if the socket is not available.

## Available Tools

### `get_state`

Get the current system state from the running shell.

| | |
|---|---|
| **Parameters** | None |
| **Returns** | Full state JSON object |

Returns the same state object pushed to `updateState(s)`. Useful for inspecting
current values while developing.

---

### `hot_reload_bar`

Reload the bar webview with new HTML.

| Parameter | Type | Description |
|---|---|---|
| `path` | `string` (optional) | Absolute path to an HTML file to load (preferred) |
| `html` | `string` (optional) | Raw HTML string (alternative) |

!!! tip "Prefer path over html"
    Writing HTML to a file first and passing the path is more reliable than sending
    large HTML strings directly, which may stall the IPC.

```
hot_reload_bar(path: "/home/user/pulpkit/poc/shells/my-shell/bar.html")
```

---

### `hot_reload_popup`

Reload the popup webview with new HTML.

| Parameter | Type | Description |
|---|---|---|
| `path` | `string` (optional) | Absolute path to an HTML file to load (preferred) |
| `html` | `string` (optional) | Raw HTML string (alternative) |

---

### `eval_js`

Evaluate JavaScript in a shell webview.

| Parameter | Type | Description |
|---|---|---|
| `target` | `string` | `"bar"` or `"popup"` |
| `script` | `string` | JavaScript code to evaluate |

```
eval_js(target: "bar", script: "document.getElementById('clock').textContent")
```

---

### `set_mock_state`

Push mock state to both webviews for testing. Overrides the real system state
temporarily.

| Parameter | Type | Description |
|---|---|---|
| `state` | `object` | Partial or full state object |

```
set_mock_state(state: {"vol": 50, "muted": true, "bat": 15, "bat_status": "Discharging"})
```

---

### `get_console_logs`

Get JavaScript console output from a webview.

| Parameter | Type | Description |
|---|---|---|
| `target` | `string` | `"bar"` or `"popup"` |

Useful for debugging JavaScript errors in shell code.

---

### `screenshot`

Capture the full screen as a PNG image (via grim).

| | |
|---|---|
| **Parameters** | None |
| **Returns** | Base64-encoded PNG image |

---

### `list_shells`

List available shell theme directories.

| | |
|---|---|
| **Parameters** | None |
| **Returns** | Comma-separated list of shell names |

---

### `get_shell_files`

Read bar.html and popup.html for a shell.

| Parameter | Type | Description |
|---|---|---|
| `name` | `string` (optional) | Shell name. Omit for the built-in default. |

---

### `save_shell`

Save a new shell theme to disk.

| Parameter | Type | Description |
|---|---|---|
| `name` | `string` | Shell directory name |
| `bar_html` | `string` | bar.html content |
| `popup_html` | `string` | popup.html content |
| `config` | `object` (optional) | config.json content |

---

### `scaffold_shell`

Create a new shell project directory with the component structure, including
bar.html, popup.html, theme.css, config.json, and stub component/panel JS files.

| Parameter | Type | Description |
|---|---|---|
| `name` | `string` | Shell directory name |

Returns the file list and development workflow instructions.

---

### `preview_shell`

Generate a browser preview of a shell and open it. Shows bar and popup side-by-side
with mock system state. No running shell required.

| Parameter | Type | Description |
|---|---|---|
| `name` | `string` | Shell theme name |
| `popup` | `string` (optional) | Which popup panel to show. Default: `"settings"` |

Opens at `http://localhost:9847/preview.html`.

---

### `validate_shell`

Validate a shell for common mistakes: missing render functions, mismatched IDs,
syntax errors, encoding issues.

| Parameter | Type | Description |
|---|---|---|
| `name` | `string` | Shell theme name |

Checks for:

- Missing required files (bar.html, popup.html, theme.css)
- Render functions called in HTML but not defined in JS
- Missing panel DOM elements
- Unbalanced braces in JS files
- Missing `<meta charset="utf-8">`
- Literal multi-byte characters (should use `ICONS.*` constants)
- Stub/TODO files not yet implemented

---

### `get_api_docs`

Get the complete Pulpkit API documentation (state fields, commands, themes, CSS
variables, HTML contract).

| | |
|---|---|
| **Parameters** | None |
| **Returns** | Full API documentation as markdown |

---

### `list_themes`

List available color themes and their CSS variable names.

| | |
|---|---|
| **Parameters** | None |
| **Returns** | Theme list and variable names |

## AI-Assisted Shell Development Workflow

The MCP server enables a powerful development loop with AI assistance:

### 1. Scaffold

Ask the AI to create a new shell:

> "Create a new shell called 'aurora' with a minimal gaming-focused bar"

The AI calls `scaffold_shell(name: "aurora")` to create the directory structure.

### 2. Implement Components

The AI writes component files one at a time, validating after each:

```
scaffold_shell("aurora")
  → writes components/clock.js
  → validate_shell("aurora")
  → hot_reload_bar(path: ".../aurora/bar.html")
  → screenshot()  // verify visually
  → writes components/status.js
  → hot_reload_bar(path: ".../aurora/bar.html")
  → ...
```

### 3. Preview Without Running Shell

Use `preview_shell` to see the design in a browser with mock data:

```
preview_shell(name: "aurora", popup: "settings")
```

### 4. Test with Live Data

Once the shell looks good in preview, hot-reload it into the running shell:

```
hot_reload_bar(path: ".../aurora/bar.html")
hot_reload_popup(path: ".../aurora/popup.html")
```

### 5. Debug

If something looks wrong:

```
get_console_logs(target: "bar")       // check for JS errors
eval_js(target: "bar", script: "...")  // inspect DOM state
get_state()                            // verify state data
set_mock_state(state: {...})           // test edge cases
```

### 6. Save

When satisfied, the shell is already saved to disk. Switch to it permanently:

```bash
pulpkit aurora
```
