# Pulpkit MCP Server — Design Spec

## Context

Build an MCP server (`pulpkit-mcp`) that gives AI agents active capabilities for shell development: hot-reload HTML, read live state, take screenshots, push mock state. This is the runtime foundation that the shell designer skill will build on.

Separate Rust binary in the same Cargo workspace. Communicates with the running Pulpkit shell via the existing IPC socket at `$XDG_RUNTIME_DIR/pulpkit.sock`.

## Architecture

```
Claude Code ←── stdio (JSON-RPC) ──→ pulpkit-mcp
                                          │
                                     Unix socket
                                          │
                                     pulpkit-webshell-poc (running shell)
                                          │
                                     WebKitGTK webviews
```

The MCP server is a bridge: it translates MCP tool calls from Claude into IPC commands for the shell, and returns results.

## IPC Protocol Extension

The shell's IPC socket currently accepts simple string commands (`vol-up`, `toggle-launcher`, etc.). We need to extend it to support structured request/response for the MCP tools.

**New protocol:** JSON lines over the Unix socket. Each message is a JSON object on one line.

```
→ {"method":"get_state"}
← {"ok":true,"data":{...full state JSON...}}

→ {"method":"reload_bar","html":"<!DOCTYPE html>..."}
← {"ok":true}

→ {"method":"reload_popup","html":"<!DOCTYPE html>..."}
← {"ok":true}

→ {"method":"set_mock_state","data":{...partial state overrides...}}
← {"ok":true}

→ {"method":"eval_js","target":"bar","script":"document.title"}
← {"ok":true,"data":"Pulpkit Bar"}
```

Old-style string commands continue to work (backwards compatible). The shell detects JSON by checking if the line starts with `{`.

## MCP Tools

### Shell Development

| Tool | Parameters | Description |
|---|---|---|
| `hot_reload_bar` | `html: string` | Replace bar HTML and reload webview |
| `hot_reload_popup` | `html: string` | Replace popup HTML and reload webview |
| `get_state` | none | Return current full state JSON |
| `set_mock_state` | `state: object` | Override state fields for testing |
| `clear_mock_state` | none | Return to real system state |
| `eval_js` | `target: "bar"\|"popup"`, `script: string` | Run JS in a webview, return result |
| `screenshot` | `target?: "bar"\|"popup"\|"full"` | Capture webview as PNG, return base64 |
| `get_console_logs` | `target: "bar"\|"popup"` | Return JS console output (errors, warnings, logs) |

### Shell Management

| Tool | Parameters | Description |
|---|---|---|
| `list_shells` | none | List available shell theme directories |
| `load_shell` | `name: string` | Switch to a different shell theme |
| `get_shell_files` | `name?: string` | Return bar.html + popup.html content for a shell |
| `save_shell` | `name: string`, `bar_html: string`, `popup_html: string`, `config?: object` | Save a new shell theme |

### API Reference

| Tool | Parameters | Description |
|---|---|---|
| `get_api_docs` | none | Return full API documentation (state shape, commands, themes, CSS vars) |
| `list_themes` | none | List available color themes with their CSS variable values |
| `list_icons` | `query?: string` | Search Nerd Font icon names |

## Implementation

### Crate: `pulpkit-mcp`

New crate at `~/pulpkit/mcp/` in the workspace.

**Dependencies:**
- `serde`, `serde_json` — JSON handling
- `tokio` — async runtime (for stdio + socket)
- Standard library — Unix socket client

**Structure:**
```
mcp/
  src/
    main.rs      — stdio loop, JSON-RPC dispatch
    ipc.rs       — Unix socket client to pulpkit shell
    tools.rs     — Tool implementations
    api_docs.rs  — Embedded API documentation
```

**MCP protocol:** JSON-RPC over stdio. Claude Code sends requests to stdin, MCP server responds on stdout. Standard `initialize`, `tools/list`, `tools/call` methods.

### Shell-side changes

The shell binary needs:
1. **Extended IPC handler** — detect JSON messages, dispatch to new handlers
2. **`reload_bar`** — call `bar_wv.load_html(new_html)`
3. **`reload_popup`** — call `popup_wv.load_html(new_html)`
4. **`get_state`** — serialize current FullState and return
5. **`set_mock_state`** — merge overrides into a mock state layer
6. **`eval_js`** — call `evaluate_javascript` on the target webview, return result
7. **Response channel** — IPC currently is fire-and-forget. Need request/response: each request gets an `id`, response includes the same `id`.

### Screenshot implementation

Wayland doesn't allow arbitrary screenshot of specific surfaces from another process. Options:
- **`grim` with geometry** — capture the bar's region. Need to know bar geometry.
- **WebKitGTK `get_snapshot`** — the webview has a snapshot API. The shell can call it and write to a temp file.
- **`eval_js` + canvas** — render the page to a canvas in JS, return as data URL.

Best option: WebKitGTK snapshot. The shell handles `screenshot` by calling `webview.get_snapshot()` and returning the PNG bytes.

## API Documentation (embedded in MCP)

The `get_api_docs` tool returns a structured document:

```json
{
  "state_fields": {
    "vol": {"type": "u32", "range": "0-100", "desc": "Audio volume percentage"},
    "muted": {"type": "bool", "desc": "Whether audio is muted"},
    ...
  },
  "commands": {
    "vol_set": {"data": "number 0-100", "desc": "Set audio volume"},
    "exec": {"data": "string", "desc": "Run arbitrary shell command"},
    ...
  },
  "themes": ["mocha", "macchiato", "frappe", "latte", "tokyonight", "nord", ...],
  "css_variables": {
    "--bg": "Base background color",
    "--fg": "Primary text color",
    "--accent": "Accent/highlight color",
    ...
  },
  "html_contract": {
    "bar": "Must define updateState(s) function. Receives full state on every update.",
    "popup": "Must define updateState(s). Panels use id='panel-{name}' with class 'active'.",
    "commands": "Send via window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify({cmd,data}))"
  }
}
```

## Claude Code Integration

Add to `~/.claude/settings.json`:
```json
{
  "mcpServers": {
    "pulpkit": {
      "command": "/home/jacob/pulpkit/target/release/pulpkit-mcp",
      "args": []
    }
  }
}
```

## Verification

1. Build: `cargo build -p pulpkit-mcp`
2. Start shell: `./target/debug/pulpkit-webshell-poc`
3. Test MCP manually: `echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | ./target/debug/pulpkit-mcp`
4. Test hot-reload: call `hot_reload_bar` with modified HTML, verify bar updates
5. Test state: call `get_state`, verify JSON contains all fields
6. Test screenshot: call `screenshot`, verify base64 PNG returned
7. Add to Claude Code settings, verify tools appear in conversation
