# Pulpkit MCP Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an MCP server that gives AI agents hot-reload, state inspection, and screenshot capabilities for Pulpkit shell development.

**Architecture:** Separate Rust binary (`pulpkit-mcp`) communicates with the running shell via a Unix socket (`$XDG_RUNTIME_DIR/pulpkit.sock`). Shell gets a new IPC listener thread. MCP server bridges stdio JSON-RPC from Claude Code to the shell's socket protocol.

**Tech Stack:** Rust, tokio (MCP server), glib (shell IPC thread), serde_json, Unix domain sockets.

**Spec:** `docs/superpowers/specs/2026-03-23-pulpkit-mcp-design.md`

---

### Task 1: Add IPC socket listener to the shell

The shell needs a Unix socket at `$XDG_RUNTIME_DIR/pulpkit.sock` that accepts JSON request/response messages.

**Files:**
- Modify: `poc/src/main.rs` — add socket listener thread, JSON dispatch

- [ ] **Step 1: Add the socket listener function**

Add after the `start_niri_stream` function. The listener runs in a background thread, accepts connections, reads JSON lines, dispatches to handlers, writes responses.

```rust
use std::os::unix::net::UnixListener;
use std::io::{BufRead, Write};

fn start_ipc_server(
    app_state: Rc<RefCell<AppState>>,
    // We need a way to send commands to the main thread for webview operations.
    // Use a glib channel since webview ops must happen on the GTK main thread.
    ipc_tx: glib::Sender<(String, std::sync::mpsc::Sender<String>)>,
) {
    let sock_path = format!("{}/pulpkit.sock", std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into()));
    // Remove stale socket
    let _ = std::fs::remove_file(&sock_path);

    std::thread::spawn(move || {
        let listener = UnixListener::bind(&sock_path).expect("failed to bind IPC socket");
        eprintln!("[pulpkit] IPC socket: {sock_path}");
        for stream in listener.incoming().flatten() {
            let reader = std::io::BufReader::new(stream.try_clone().unwrap());
            let mut writer = stream;
            for line in reader.lines().flatten() {
                let trimmed = line.trim();
                if trimmed.starts_with('{') {
                    // JSON request — send to main thread, wait for response
                    let (resp_tx, resp_rx) = std::sync::mpsc::channel();
                    let _ = ipc_tx.send((trimmed.to_string(), resp_tx));
                    let response = resp_rx.recv_timeout(std::time::Duration::from_secs(5))
                        .unwrap_or_else(|_| r#"{"ok":false,"error":"timeout"}"#.to_string());
                    let _ = writeln!(writer, "{response}");
                } else {
                    // Legacy string command — fire and forget
                    // (handled by sending as a command through app_state)
                }
            }
        }
    });
}
```

Note: `Rc<RefCell<AppState>>` is not Send, so we can't use it directly in the thread. The socket thread only sends JSON to the main thread via `glib::Sender`. All state access and webview operations happen on the main thread.

- [ ] **Step 2: Add the main-thread IPC handler**

In the `connect_activate` callback, create the glib channel and handle incoming IPC requests. This runs on the GTK main thread so it can access webviews.

```rust
let (ipc_tx, ipc_rx) = glib::MainContext::channel::<(String, std::sync::mpsc::Sender<String>)>(glib::Priority::DEFAULT);

// Clone refs for the handler
let bar_wv_ipc = bar_wv.clone();
let popup_wv_ipc = popup_wv.clone();
let app_state_ipc = app_state.clone();
let polled_ipc = polled_state.clone();

ipc_rx.attach(None, move |msg, resp_tx| {
    let (request_json, resp_tx) = (msg, resp_tx);
    let parsed: serde_json::Value = serde_json::from_str(&request_json).unwrap_or_default();
    let method = parsed["method"].as_str().unwrap_or("");

    let response = match method {
        "get_state" => {
            let as_ = app_state_ipc.borrow();
            let mut state = polled_ipc.lock().map(|p| p.clone()).unwrap_or_default();
            // merge app state fields...
            state.popup = as_.popup.clone();
            state.theme = as_.theme.clone();
            // etc.
            serde_json::to_string(&serde_json::json!({"ok": true, "data": state}))
                .unwrap_or_else(|_| r#"{"ok":false}"#.into())
        }
        "reload_bar" => {
            if let Some(html) = parsed["html"].as_str() {
                bar_wv_ipc.load_html(html, Some("file:///"));
                r#"{"ok":true}"#.to_string()
            } else { r#"{"ok":false,"error":"missing html"}"#.to_string() }
        }
        "reload_popup" => {
            if let Some(html) = parsed["html"].as_str() {
                popup_wv_ipc.load_html(html, Some("file:///"));
                r#"{"ok":true}"#.to_string()
            } else { r#"{"ok":false,"error":"missing html"}"#.to_string() }
        }
        "eval_js" => {
            // Async — need to use a channel to get the result back
            // For now, fire and forget
            let target = parsed["target"].as_str().unwrap_or("bar");
            let script = parsed["script"].as_str().unwrap_or("");
            let wv = if target == "popup" { &popup_wv_ipc } else { &bar_wv_ipc };
            wv.evaluate_javascript(script, None, None, None::<&gtk4::gio::Cancellable>, |_| {});
            r#"{"ok":true}"#.to_string()
        }
        _ => format!(r#"{{"ok":false,"error":"unknown method: {method}"}}"#),
    };
    let _ = resp_tx.send(response);
    glib::ControlFlow::Continue
});
```

- [ ] **Step 3: Start the IPC server in activate**

```rust
start_ipc_server(ipc_tx);
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p pulpkit-webshell-poc`

Test with:
```bash
./target/debug/pulpkit-webshell-poc &
echo '{"method":"get_state"}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/pulpkit.sock
```

Expected: JSON response with full state.

- [ ] **Step 5: Commit**

```bash
git add poc/src/main.rs
git commit -m "feat: add IPC socket with JSON request/response to shell"
```

---

### Task 2: Create the MCP server crate

**Files:**
- Create: `mcp/Cargo.toml`
- Create: `mcp/src/main.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Initialize the crate**

```bash
cd ~/pulpkit && cargo init --name pulpkit-mcp mcp
```

- [ ] **Step 2: Set up Cargo.toml**

```toml
[package]
name = "pulpkit-mcp"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt", "io-util", "io-std", "net", "macros"] }
```

- [ ] **Step 3: Write the MCP server main.rs**

Implements stdio JSON-RPC: reads requests from stdin, dispatches to tool handlers, writes responses to stdout. Connects to the shell's Unix socket to forward requests.

The MCP protocol requires:
- `initialize` → return capabilities
- `tools/list` → return tool definitions
- `tools/call` → execute a tool, return result

Each tool call translates to a JSON message sent to the shell's socket.

```rust
// Pseudocode structure:
#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Read JSON-RPC lines from stdin
    // For each request:
    //   - If initialize: return server info
    //   - If tools/list: return tool definitions
    //   - If tools/call: connect to socket, send method, return result
    // Write JSON-RPC response to stdout
}
```

Full implementation in the step.

- [ ] **Step 4: Build and test**

```bash
cargo build -p pulpkit-mcp
echo '{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"capabilities":{}}}' | ./target/debug/pulpkit-mcp
```

Expected: JSON-RPC response with server capabilities.

- [ ] **Step 5: Commit**

```bash
git add mcp/ Cargo.toml
git commit -m "feat: add pulpkit-mcp server crate with stdio JSON-RPC"
```

---

### Task 3: Implement core MCP tools

**Files:**
- Modify: `mcp/src/main.rs` — add tool implementations

- [ ] **Step 1: Implement `get_state` tool**

Connects to shell socket, sends `{"method":"get_state"}`, returns the state JSON.

- [ ] **Step 2: Implement `hot_reload_bar` and `hot_reload_popup` tools**

Sends `{"method":"reload_bar","html":"..."}` to shell socket.

- [ ] **Step 3: Implement `eval_js` tool**

Sends `{"method":"eval_js","target":"bar","script":"..."}` to shell socket.

- [ ] **Step 4: Implement `get_api_docs` tool**

Returns embedded API documentation (state fields, commands, themes, CSS vars, HTML contract). This is a static response, no socket needed.

- [ ] **Step 5: Implement `screenshot` tool**

Sends `{"method":"screenshot","target":"bar"}` to shell socket. Shell calls WebKitGTK snapshot API, returns base64 PNG.

- [ ] **Step 6: Implement shell management tools**

`list_shells`, `save_shell`, `get_shell_files`, `load_shell` — these read/write the `shells/` directory and optionally reload the running shell.

- [ ] **Step 7: Implement `get_console_logs` tool**

Shell-side: inject a console.log interceptor into the webview that buffers messages. `get_console_logs` returns and clears the buffer.

- [ ] **Step 8: Build and test all tools**

```bash
cargo build -p pulpkit-mcp
# Start shell
./target/debug/pulpkit-webshell-poc &
# Test get_state
echo '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"get_state","arguments":{}}}' | ./target/debug/pulpkit-mcp
# Test hot_reload_bar
echo '{"jsonrpc":"2.0","method":"tools/call","id":2,"params":{"name":"hot_reload_bar","arguments":{"html":"<h1>Test</h1>"}}}' | ./target/debug/pulpkit-mcp
```

- [ ] **Step 9: Commit**

```bash
git add mcp/src/ poc/src/
git commit -m "feat: implement all MCP tools (state, reload, eval, screenshot, management)"
```

---

### Task 4: Shell-side screenshot and console log support

**Files:**
- Modify: `poc/src/main.rs` — add screenshot handler, console log buffer

- [ ] **Step 1: Add screenshot IPC handler**

In the main-thread IPC handler, add a `"screenshot"` method that:
1. Calls `webview.get_snapshot()` (WebKitGTK API)
2. Encodes the result as base64 PNG
3. Returns it in the response

- [ ] **Step 2: Add console log capture**

Inject a JS snippet on webview load that intercepts `console.log/warn/error`:
```javascript
window.__pulpkit_logs = [];
const _orig = {log: console.log, warn: console.warn, error: console.error};
['log','warn','error'].forEach(level => {
    console[level] = (...args) => {
        window.__pulpkit_logs.push({level, msg: args.map(String).join(' '), ts: Date.now()});
        _orig[level](...args);
    };
});
```

The `get_console_logs` IPC method evals `JSON.stringify(window.__pulpkit_logs)` and returns the result.

- [ ] **Step 3: Build and test**

- [ ] **Step 4: Commit**

---

### Task 5: Claude Code integration

**Files:**
- Modify: `~/.claude/settings.json` or `~/.claude/settings.local.json`

- [ ] **Step 1: Build release binary**

```bash
cargo build --release -p pulpkit-mcp
```

- [ ] **Step 2: Add MCP server to Claude Code settings**

```json
{
  "mcpServers": {
    "pulpkit": {
      "command": "/home/jacob/pulpkit/target/release/pulpkit-mcp"
    }
  }
}
```

- [ ] **Step 3: Verify tools appear in Claude Code**

Start a new Claude Code conversation. The MCP tools should be listed as available tools.

- [ ] **Step 4: End-to-end test**

With the shell running, ask Claude to:
1. Call `get_state` — verify state returned
2. Call `hot_reload_bar` with modified HTML — verify bar updates
3. Call `screenshot` — verify image returned
4. Call `get_api_docs` — verify docs returned

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: pulpkit MCP server complete — hot-reload, state, screenshot, API docs"
```
