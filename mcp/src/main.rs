use std::io::{BufRead, Write};
use std::os::unix::net::UnixStream;
use serde_json::{json, Value};

fn sock_path() -> String {
    std::env::var("PULPKIT_SOCK").unwrap_or_else(|_| {
        format!("{}/pulpkit.sock",
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into()))
    })
}

fn ipc_call(request: &Value) -> Result<Value, String> {
    let path = sock_path();
    let mut stream = UnixStream::connect(&path)
        .map_err(|e| format!("Cannot connect to pulpkit at {path}: {e}. Is the shell running?"))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).ok();
    let msg = serde_json::to_string(request).map_err(|e| e.to_string())?;
    writeln!(stream, "{msg}").map_err(|e| format!("Write: {e}"))?;
    stream.flush().map_err(|e| format!("Flush: {e}"))?;
    let mut reader = std::io::BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| format!("Read: {e}"))?;
    serde_json::from_str(line.trim()).map_err(|e| format!("Parse: {e}"))
}

fn tool_definitions() -> Value {
    json!([
        {"name":"get_state","description":"Get current system state from running Pulpkit shell (volume, battery, wifi, workspaces, windows, tray, CPU, memory, disk, etc).","inputSchema":{"type":"object","properties":{}}},
        {"name":"hot_reload_bar","description":"Reload the bar webview. PREFERRED: write HTML to a file first, then pass the path. Alternative: pass html directly (but large HTML may stall).","inputSchema":{"type":"object","properties":{"path":{"type":"string","description":"Absolute path to an HTML file to load (preferred — write file first, then pass path)"},"html":{"type":"string","description":"Raw HTML string (alternative — may stall on large content)"}}}},
        {"name":"hot_reload_popup","description":"Reload the popup webview. PREFERRED: write HTML to a file first, then pass the path.","inputSchema":{"type":"object","properties":{"path":{"type":"string","description":"Absolute path to an HTML file to load (preferred)"},"html":{"type":"string","description":"Raw HTML string (alternative)"}}}},
        {"name":"eval_js","description":"Evaluate JavaScript in a shell webview.","inputSchema":{"type":"object","properties":{"target":{"type":"string","enum":["bar","popup"]},"script":{"type":"string"}},"required":["target","script"]}},
        {"name":"set_mock_state","description":"Push mock state to both webviews for testing.","inputSchema":{"type":"object","properties":{"state":{"type":"object"}},"required":["state"]}},
        {"name":"get_console_logs","description":"Get JS console output from a webview.","inputSchema":{"type":"object","properties":{"target":{"type":"string","enum":["bar","popup"]}},"required":["target"]}},
        {"name":"screenshot","description":"Capture full screen as PNG (via grim).","inputSchema":{"type":"object","properties":{}}},
        {"name":"list_shells","description":"List available shell theme directories.","inputSchema":{"type":"object","properties":{}}},
        {"name":"get_shell_files","description":"Read bar.html and popup.html for a shell.","inputSchema":{"type":"object","properties":{"name":{"type":"string","description":"Theme name. Omit for default."}}}},
        {"name":"save_shell","description":"Save a new shell theme.","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"bar_html":{"type":"string"},"popup_html":{"type":"string"},"config":{"type":"object"}},"required":["name","bar_html","popup_html"]}},
        {"name":"scaffold_shell","description":"Create a new shell project directory with component structure. Returns the paths to edit. The bar.html/popup.html are thin skeletons that load components via <script> tags. Write each component as a separate .js file, then hot_reload_bar with the bar.html path.","inputSchema":{"type":"object","properties":{"name":{"type":"string","description":"Shell name (directory name)"}},"required":["name"]}},
        {"name":"preview_shell","description":"Generate a browser preview of a shell and open it. Shows bar + popup side-by-side with mock system state, no running shell needed. Use this for design iteration before hot-reloading to the live shell. After editing components, call this again to refresh.","inputSchema":{"type":"object","properties":{"name":{"type":"string","description":"Shell theme name"},"popup":{"type":"string","description":"Which popup panel to show (settings/wifi/power/launcher/config). Default: settings"}},"required":["name"]}},
        {"name":"validate_shell","description":"Validate a shell for common mistakes: missing render functions, mismatched IDs, syntax errors, encoding issues. Run after writing components, before hot-reloading.","inputSchema":{"type":"object","properties":{"name":{"type":"string","description":"Shell theme name"}},"required":["name"]}},
        {"name":"get_api_docs","description":"Get complete Pulpkit API docs: state fields, commands, themes, CSS vars, HTML contract.","inputSchema":{"type":"object","properties":{}}},
        {"name":"list_themes","description":"List color themes and their CSS variable names.","inputSchema":{"type":"object","properties":{}}}
    ])
}

fn shells_dir() -> String {
    let exe = std::env::current_exe().unwrap_or_default();
    let ws = exe.ancestors().nth(3).unwrap_or(std::path::Path::new("."));
    ws.join("poc/shells").to_string_lossy().to_string()
}

fn base64_encode(data: &[u8]) -> String {
    const C: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut r = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let (b0, b1, b2) = (chunk[0] as u32, chunk.get(1).copied().unwrap_or(0) as u32, chunk.get(2).copied().unwrap_or(0) as u32);
        let n = (b0 << 16) | (b1 << 8) | b2;
        r.push(C[(n >> 18 & 63) as usize] as char);
        r.push(C[(n >> 12 & 63) as usize] as char);
        r.push(if chunk.len() > 1 { C[(n >> 6 & 63) as usize] as char } else { '=' });
        r.push(if chunk.len() > 2 { C[(n & 63) as usize] as char } else { '=' });
    }
    r
}

fn content_is_todo(s: &str) -> bool {
    s.lines().count() < 5 && s.contains("TODO")
}

fn walkdir(dir: &str) -> Vec<String> {
    let mut files = Vec::new();
    fn walk(dir: &str, files: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() { walk(&p.to_string_lossy(), files); }
                else { files.push(p.to_string_lossy().to_string()); }
            }
        }
    }
    walk(dir, &mut files);
    files
}

fn mock_state_json() -> String {
    serde_json::to_string(&json!({
        "vol": 72, "muted": false, "audio_device": "HD Audio Speaker",
        "bright": 65,
        "bat": 78, "bat_status": "Discharging", "has_bat": true,
        "cpu": 23, "mem": 45,
        "disk_used": "171G", "disk_total": "248G", "disk_pct": 69,
        "power_profile": "balanced",
        "wifi": "HomeNetwork", "net_signal": 82, "net_ip": "192.168.1.42",
        "notif_count": 2, "dnd": false,
        "ws": [
            {"idx": 1, "active": true},
            {"idx": 2, "active": false},
            {"idx": 3, "active": false}
        ],
        "windows": [
            {"id": 1, "title": "Firefox", "app_id": "firefox", "focused": true, "icon": ""},
            {"id": 2, "title": "Terminal", "app_id": "kitty", "focused": false, "icon": ""},
            {"id": 3, "title": "Code", "app_id": "code", "focused": false, "icon": ""}
        ],
        "active_title": "Firefox", "active_app_id": "firefox",
        "wifi_nets": [
            {"ssid": "HomeNetwork", "signal": 82, "secure": true, "active": true},
            {"ssid": "Neighbor_5G", "signal": 54, "secure": true, "active": false},
            {"ssid": "CoffeeShop", "signal": 31, "secure": false, "active": false}
        ],
        "apps": [
            {"name": "Firefox", "exec": "firefox", "icon": ""},
            {"name": "Kitty", "exec": "kitty", "icon": ""},
            {"name": "Files", "exec": "nautilus", "icon": ""},
            {"name": "Steam", "exec": "steam", "icon": ""},
            {"name": "Spotify", "exec": "spotify", "icon": ""},
            {"name": "Discord", "exec": "discord", "icon": ""}
        ],
        "tray_items": [
            {"id": "steam", "address": ":1.100", "title": "Steam", "icon": ""},
            {"id": "discord", "address": ":1.101", "title": "Discord", "icon": ""}
        ],
        "popup": "settings",
        "theme": "gruvbox",
        "custom": {},
        "user": "user", "host": "archlinux", "kernel": "6.19.7-1-cachyos", "uptime": "3 hours, 12 min"
    })).unwrap_or_default()
}

fn execute_tool(name: &str, args: &Value) -> Value {
    match name {
        "get_state" => match ipc_call(&json!({"method":"get_state"})) {
            Ok(r) => json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&r["data"]).unwrap_or_default()}]}),
            Err(e) => json!({"content":[{"type":"text","text":e}],"isError":true}),
        },
        "hot_reload_bar" => {
            let req = if let Some(path) = args["path"].as_str() {
                json!({"method":"reload_bar","path":path})
            } else {
                json!({"method":"reload_bar","html":args["html"]})
            };
            match ipc_call(&req) {
                Ok(_) => json!({"content":[{"type":"text","text":"Bar reloaded."}]}),
                Err(e) => json!({"content":[{"type":"text","text":e}],"isError":true}),
            }
        }
        "hot_reload_popup" => {
            let req = if let Some(path) = args["path"].as_str() {
                json!({"method":"reload_popup","path":path})
            } else {
                json!({"method":"reload_popup","html":args["html"]})
            };
            match ipc_call(&req) {
                Ok(_) => json!({"content":[{"type":"text","text":"Popup reloaded."}]}),
                Err(e) => json!({"content":[{"type":"text","text":e}],"isError":true}),
            }
        }
        "eval_js" => match ipc_call(&json!({"method":"eval_js","target":args["target"],"script":args["script"]})) {
            Ok(r) => json!({"content":[{"type":"text","text":format!("OK: {}",r.get("data").unwrap_or(&json!(null)))}]}),
            Err(e) => json!({"content":[{"type":"text","text":e}],"isError":true}),
        },
        "set_mock_state" => match ipc_call(&json!({"method":"set_mock_state","data":args["state"]})) {
            Ok(_) => json!({"content":[{"type":"text","text":"Mock state pushed."}]}),
            Err(e) => json!({"content":[{"type":"text","text":e}],"isError":true}),
        },
        "get_console_logs" => match ipc_call(&json!({"method":"get_console_logs","target":args["target"]})) {
            Ok(r) => json!({"content":[{"type":"text","text":r["data"].as_str().unwrap_or("[]")}]}),
            Err(e) => json!({"content":[{"type":"text","text":e}],"isError":true}),
        },
        "screenshot" => match std::process::Command::new("grim").args(["-"]).output() {
            Ok(out) if out.status.success() => json!({"content":[{"type":"image","data":base64_encode(&out.stdout),"mimeType":"image/png"}]}),
            Ok(out) => json!({"content":[{"type":"text","text":format!("grim failed: {}",String::from_utf8_lossy(&out.stderr))}],"isError":true}),
            Err(e) => json!({"content":[{"type":"text","text":format!("grim error: {e}")}],"isError":true}),
        },
        "list_shells" => {
            let mut shells = Vec::new();
            if let Ok(entries) = std::fs::read_dir(shells_dir()) {
                for e in entries.flatten() {
                    if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        shells.push(e.file_name().to_string_lossy().to_string());
                    }
                }
            }
            shells.sort();
            json!({"content":[{"type":"text","text":format!("Shells: {}",shells.join(", "))}]})
        }
        "get_shell_files" => {
            let name = args["name"].as_str().unwrap_or("");
            let (bp, pp) = if name.is_empty() {
                let exe = std::env::current_exe().unwrap_or_default();
                let ws = exe.ancestors().nth(3).unwrap_or(std::path::Path::new("."));
                (ws.join("poc/src/bar.html").to_string_lossy().to_string(), ws.join("poc/src/popup.html").to_string_lossy().to_string())
            } else {
                (format!("{}/{name}/bar.html", shells_dir()), format!("{}/{name}/popup.html", shells_dir()))
            };
            let bar = std::fs::read_to_string(&bp).unwrap_or_else(|_| format!("Not found: {bp}"));
            let popup = std::fs::read_to_string(&pp).unwrap_or_else(|_| format!("Not found: {pp}"));
            json!({"content":[{"type":"text","text":format!("=== bar.html ===\n{bar}\n\n=== popup.html ===\n{popup}")}]})
        }
        "save_shell" => {
            let name = args["name"].as_str().unwrap_or("untitled");
            let dir = format!("{}/{name}", shells_dir());
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(format!("{dir}/bar.html"), args["bar_html"].as_str().unwrap_or(""));
            let _ = std::fs::write(format!("{dir}/popup.html"), args["popup_html"].as_str().unwrap_or(""));
            if let Some(cfg) = args.get("config") {
                let _ = std::fs::write(format!("{dir}/config.json"), serde_json::to_string_pretty(cfg).unwrap_or_default());
            }
            json!({"content":[{"type":"text","text":format!("Shell '{name}' saved to {dir}/")}]})
        }
        "preview_shell" => {
            let name = args["name"].as_str().unwrap_or("default");
            let popup = args["popup"].as_str().unwrap_or("settings");
            let dir = format!("{}/{name}", shells_dir());

            if !std::path::Path::new(&format!("{dir}/bar.html")).exists() {
                return json!({"content":[{"type":"text","text":format!("Shell '{name}' not found at {dir}/")}],"isError":true});
            }

            // Generate preview HTML that wraps bar + popup with mock state
            let preview = format!(r##"<!DOCTYPE html>
<html data-theme="gruvbox">
<head>
<meta charset="utf-8">
<title>Pulpkit Preview: {name}</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ background: #1a1a2e; font-family: system-ui; color: #ccc; }}
  .desktop {{
    width: 100vw; height: 100vh;
    display: flex; flex-direction: column;
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
  }}
  .bar-frame {{
    width: 100%; height: 40px;
    border-bottom: 1px solid rgba(255,255,255,0.05);
  }}
  .bar-frame iframe {{ width: 100%; height: 100%; border: none; }}
  .content-area {{
    flex: 1; display: flex; align-items: center; justify-content: center;
    gap: 24px; padding: 24px;
  }}
  .popup-frame {{
    width: 380px; height: 500px;
    border: 1px solid rgba(255,255,255,0.1);
    box-shadow: 0 8px 32px rgba(0,0,0,0.5);
  }}
  .popup-frame iframe {{ width: 100%; height: 100%; border: none; }}
  .label {{
    font-size: 11px; color: rgba(255,255,255,0.3);
    text-align: center; padding: 8px;
    text-transform: uppercase; letter-spacing: 2px;
  }}
  .controls {{
    position: fixed; bottom: 16px; right: 16px;
    display: flex; gap: 8px;
  }}
  .controls button {{
    padding: 6px 14px; background: rgba(255,255,255,0.1);
    border: 1px solid rgba(255,255,255,0.15); color: #ccc;
    font-size: 12px; cursor: pointer;
  }}
  .controls button:hover {{ background: rgba(255,255,255,0.2); }}
  .popup-selector {{
    position: fixed; bottom: 16px; left: 16px;
    display: flex; gap: 4px;
  }}
  .popup-selector button {{
    padding: 4px 10px; background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.1); color: #888;
    font-size: 11px; cursor: pointer;
  }}
  .popup-selector button.active {{ background: rgba(255,255,255,0.15); color: #fff; }}
</style>
</head>
<body>
<div class="desktop">
  <div class="bar-frame">
    <iframe id="bar-iframe" src="bar-preview.html"></iframe>
  </div>
  <div class="content-area">
    <div>
      <div class="label">Popup Preview</div>
      <div class="popup-frame">
        <iframe id="popup-iframe" src="popup-preview.html"></iframe>
      </div>
    </div>
  </div>
</div>
<div class="popup-selector">
  <button onclick="switchPopup('settings')" class="active">Settings</button>
  <button onclick="switchPopup('wifi')">WiFi</button>
  <button onclick="switchPopup('power')">Power</button>
  <button onclick="switchPopup('launcher')">Launcher</button>
  <button onclick="switchPopup('config')">Config</button>
</div>
<div class="controls">
  <button onclick="refresh()">Refresh</button>
</div>
<script>
let currentPopup = '{popup}';
function switchPopup(name) {{
  currentPopup = name;
  document.querySelectorAll('.popup-selector button').forEach(b => b.classList.toggle('active', b.textContent.toLowerCase() === name));
  document.getElementById('popup-iframe').contentWindow.postMessage({{type:'setPopup',popup:name}}, '*');
}}
function refresh() {{
  document.getElementById('bar-iframe').src = 'bar-preview.html?' + Date.now();
  document.getElementById('popup-iframe').src = 'popup-preview.html?' + Date.now();
}}
</script>
</body>
</html>"##, name=name, popup=popup);

            // Bar preview wrapper — loads real bar.html content but mocks webkit
            let bar_preview = format!(r##"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="margin:0;padding:0;overflow:hidden;">
<script>
// Mock webkit message handler
window.webkit = {{ messageHandlers: {{ pulpkit: {{ postMessage: function(msg) {{
  console.log('[pulpkit cmd]', msg);
}} }} }} }};
</script>
<script>
// Load the real bar HTML via fetch, inject into this document
fetch('bar.html').then(r => r.text()).then(html => {{
  // Extract and execute
  document.open();
  document.write(html);
  document.close();

  // Inject mock state after a moment
  setTimeout(() => {{
    if (typeof updateState === 'function') {{
      updateState({mock_state});
    }}
  }}, 200);
  setInterval(() => {{
    if (typeof updateState === 'function') {{
      updateState({mock_state});
    }}
  }}, 2000);
}});
</script>
</body>
</html>"##, mock_state = mock_state_json());

            // Popup preview wrapper
            let popup_preview = format!(r##"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="margin:0;padding:0;overflow:hidden;">
<script>
window.webkit = {{ messageHandlers: {{ pulpkit: {{ postMessage: function(msg) {{
  console.log('[pulpkit cmd]', msg);
}} }} }} }};
</script>
<script>
let currentPopup = '{popup}';
window.addEventListener('message', e => {{
  if (e.data && e.data.type === 'setPopup') {{
    currentPopup = e.data.popup;
    if (typeof updateState === 'function') {{
      let s = JSON.parse('{mock_state_escaped}');
      s.popup = currentPopup;
      updateState(s);
    }}
  }}
}});
fetch('popup.html').then(r => r.text()).then(html => {{
  document.open();
  document.write(html);
  document.close();
  setTimeout(() => {{
    if (typeof updateState === 'function') {{
      let s = JSON.parse('{mock_state_escaped}');
      s.popup = currentPopup;
      updateState(s);
    }}
  }}, 200);
}});
</script>
</body>
</html>"##,
                popup = popup,
                mock_state_escaped = mock_state_json().replace('\'', "\\'").replace('\n', "")
            );

            // Write preview files to the shell directory
            let _ = std::fs::write(format!("{dir}/preview.html"), &preview);
            let _ = std::fs::write(format!("{dir}/bar-preview.html"), &bar_preview);
            let _ = std::fs::write(format!("{dir}/popup-preview.html"), &popup_preview);

            // Start a simple HTTP server if not already running
            let port = 9847u16;
            let _ = std::process::Command::new("sh")
                .args(["-c", &format!(
                    "fuser {port}/tcp 2>/dev/null || (cd '{dir}' && python3 -m http.server {port} &>/dev/null &)"
                )])
                .output();

            // Give server a moment to start
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Open in browser
            let url = format!("http://localhost:{port}/preview.html");
            let _ = std::process::Command::new("xdg-open").arg(&url).output();

            json!({"content":[{"type":"text","text":format!("Preview opened at {url}\n\nShowing bar at top, {popup} panel below.\nUse buttons at bottom-left to switch popup panels.\nClick 'Refresh' after editing components.\n\nEdit components, then call preview_shell again to refresh.")}]})
        }
        "scaffold_shell" => {
            let name = args["name"].as_str().unwrap_or("new-shell");
            let dir = format!("{}/{name}", shells_dir());
            let _ = std::fs::create_dir_all(format!("{dir}/components"));
            let _ = std::fs::create_dir_all(format!("{dir}/panels"));

            // bar.html skeleton
            std::fs::write(format!("{dir}/bar.html"), format!(r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<link rel="stylesheet" href="theme.css">
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    font-family: 'JetBrainsMono Nerd Font', monospace;
    font-size: 13px;
    background: var(--bg);
    color: var(--fg);
    height: 100vh;
    display: flex;
    align-items: center;
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
  }}
</style>
</head>
<body>
<div id="bar"></div>
<script>
// ── Framework helpers (available to all components) ──
function send(o) {{ window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o)); }}
let _st = {{}};

// Throttle rapid commands (sliders). Max 1 per 80ms per key.
function sendThrottled(key, o) {{ if(window['_t_'+key]) return; send(o); window['_t_'+key] = setTimeout(() => {{ delete window['_t_'+key]; }}, 80); }}

// Smart list update: only rebuild if data changed. Preserves scroll position.
function updateList(containerId, items, keyFn, renderFn) {{
  const el = document.getElementById(containerId);
  if (!el) return;
  const newKey = items.map(keyFn).join(',');
  if (el.dataset.key === newKey) return; // no change
  el.dataset.key = newKey;
  const scroll = el.scrollTop;
  el.innerHTML = '';
  items.forEach(item => el.appendChild(renderFn(item)));
  el.scrollTop = scroll;
}}

// Set text only if changed (avoids flicker)
function setText(id, text) {{
  const el = document.getElementById(id);
  if (el && el.textContent !== text) el.textContent = text;
}}

// Set HTML only if changed
function setHtml(id, html) {{
  const el = document.getElementById(id);
  if (el && el.innerHTML !== html) el.innerHTML = html;
}}

// Nerd Font icon map (use ICONS.vol_hi instead of literal chars in component JS)
const ICONS = {{
  vol_hi:'󰕾', vol_mid:'󰖀', vol_lo:'󰕿', vol_mute:'󰝟',
  bat_full:'󰁹', bat_good:'󰂀', bat_half:'󰁾', bat_low:'󰁻',
  bat_chrg:'󰂄', bat_empty:'󰂎',
  wifi_4:'󰤨', wifi_3:'󰤥', wifi_2:'󰤢', wifi_1:'󰤟', wifi_off:'󰤭',
  bright:'󰃟', power:'󰐥', lock:'󰌾', suspend:'󰤄',
  logout:'󰗼', reboot:'󰜉', shutdown:'󰐦',
  search:'󰍉', settings:'󰒓', check:'󰄬',
  night_on:'󰌵', night_off:'󰌶',
  dnd_on:'󰍶', dnd_off:'󰍷',
  bt_on:'󰂯', bt_off:'󰂲',
  cpu:'󰍛', mem:'󰍛',
  dot_filled:'󱓻', dot_empty:'',
}};

// Volume icon helper
function volIcon(v, m) {{ return m ? ICONS.vol_mute : v > 50 ? ICONS.vol_hi : v > 0 ? ICONS.vol_mid : ICONS.vol_lo; }}
// WiFi signal icon helper
function wifiIcon(sig) {{ return sig > 75 ? ICONS.wifi_4 : sig > 50 ? ICONS.wifi_3 : sig > 25 ? ICONS.wifi_2 : sig > 0 ? ICONS.wifi_1 : ICONS.wifi_off; }}
// Battery icon helper
function batIcon(pct, st) {{
  if (st === 'Charging') return ICONS.bat_chrg;
  return pct > 80 ? ICONS.bat_full : pct > 60 ? ICONS.bat_good : pct > 30 ? ICONS.bat_half : pct > 10 ? ICONS.bat_low : ICONS.bat_empty;
}}
</script>
<script src="components/workspaces.js" charset="utf-8"></script>
<script src="components/taskbar.js" charset="utf-8"></script>
<script src="components/clock.js" charset="utf-8"></script>
<script src="components/status.js" charset="utf-8"></script>
<script src="components/tray.js" charset="utf-8"></script>
<script>
function updateState(s) {{
  _st = s;
  if (s.theme) document.documentElement.setAttribute('data-theme', s.theme);
  renderWorkspaces(s);
  renderTaskbar(s);
  renderClock(s);
  renderStatus(s);
  renderTray(s);
}}
</script>
</body>
</html>"#)).ok();

            // popup.html skeleton
            std::fs::write(format!("{dir}/popup.html"), format!(r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<link rel="stylesheet" href="theme.css">
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    font-family: 'JetBrainsMono Nerd Font', monospace;
    font-size: 13px;
    background: transparent;
    color: var(--fg);
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
  }}
  .panel {{
    background: var(--bg-surface);
    border: 1px solid var(--bg-overlay);
    padding: 20px;
    display: none;
    flex-direction: column;
    gap: 14px;
    width: 100%;
    height: 100vh;
    overflow-y: auto;
  }}
  .panel.active {{ display: flex; }}
  .panel::-webkit-scrollbar {{ width: 4px; }}
  .panel::-webkit-scrollbar-thumb {{ background: var(--fg-dim); }}
</style>
</head>
<body>
<div id="panel-settings" class="panel"></div>
<div id="panel-wifi" class="panel"></div>
<div id="panel-power" class="panel"></div>
<div id="panel-launcher" class="panel"></div>
<div id="panel-config" class="panel"></div>
<script>
// ── Framework helpers ──
function send(o) {{ window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o)); }}
function sendThrottled(key, o) {{ if(window['_t_'+key]) return; send(o); window['_t_'+key] = setTimeout(() => {{ delete window['_t_'+key]; }}, 80); }}
function updateList(id, items, keyFn, renderFn) {{
  const el = document.getElementById(id); if (!el) return;
  const k = items.map(keyFn).join(','); if (el.dataset.key === k) return;
  el.dataset.key = k; const sc = el.scrollTop; el.innerHTML = '';
  items.forEach(i => el.appendChild(renderFn(i))); el.scrollTop = sc;
}}
function setText(id, t) {{ const e = document.getElementById(id); if (e && e.textContent !== t) e.textContent = t; }}
function setHtml(id, h) {{ const e = document.getElementById(id); if (e && e.innerHTML !== h) e.innerHTML = h; }}
const ICONS = {{
  vol_hi:'󰕾',vol_mid:'󰖀',vol_lo:'󰕿',vol_mute:'󰝟',
  wifi_4:'󰤨',wifi_3:'󰤥',wifi_2:'󰤢',wifi_1:'󰤟',wifi_off:'󰤭',
  bright:'󰃟',power:'󰐥',lock:'󰌾',suspend:'󰤄',
  logout:'󰗼',reboot:'󰜉',shutdown:'󰐦',
  search:'󰍉',settings:'󰒓',check:'󰄬',
  night_on:'󰌵',night_off:'󰌶',dnd_on:'󰍶',dnd_off:'󰍷',
  bt_on:'󰂯',bt_off:'󰂲',
}};
function volIcon(v,m) {{ return m?ICONS.vol_mute:v>50?ICONS.vol_hi:v>0?ICONS.vol_mid:ICONS.vol_lo; }}
function wifiIcon(s) {{ return s>75?ICONS.wifi_4:s>50?ICONS.wifi_3:s>25?ICONS.wifi_2:s>0?ICONS.wifi_1:ICONS.wifi_off; }}
</script>
<script src="panels/settings.js" charset="utf-8"></script>
<script src="panels/wifi.js" charset="utf-8"></script>
<script src="panels/power.js" charset="utf-8"></script>
<script src="panels/launcher.js" charset="utf-8"></script>
<script src="panels/config.js" charset="utf-8"></script>
<script>
function updateState(s) {{
  if (s.theme) document.documentElement.setAttribute('data-theme', s.theme);
  document.querySelectorAll('.panel').forEach(p => p.classList.remove('active'));
  const panel = document.getElementById('panel-' + s.popup);
  if (panel) panel.classList.add('active');
  if (typeof renderSettings === 'function') renderSettings(s);
  if (typeof renderWifi === 'function') renderWifi(s);
  if (typeof renderPower === 'function') renderPower(s);
  if (typeof renderLauncher === 'function') renderLauncher(s);
  if (typeof renderConfig === 'function') renderConfig(s);
}}
document.addEventListener('keydown', e => {{ if (e.key === 'Escape') send({{cmd:'dismiss'}}); }});
</script>
</body>
</html>"#)).ok();

            // Empty component stubs
            for comp in &["workspaces", "taskbar", "clock", "status", "tray"] {
                std::fs::write(format!("{dir}/components/{comp}.js"),
                    format!("// {comp} component\nfunction render{}(s) {{\n  // TODO: implement\n}}\n",
                        comp.chars().next().unwrap().to_uppercase().to_string() + &comp[1..])).ok();
            }
            for panel in &["settings", "wifi", "power", "launcher", "config"] {
                std::fs::write(format!("{dir}/panels/{panel}.js"),
                    format!("// {panel} panel\nfunction render{}(s) {{\n  // TODO: implement\n}}\n",
                        panel.chars().next().unwrap().to_uppercase().to_string() + &panel[1..])).ok();
            }

            // Theme CSS with variables
            std::fs::write(format!("{dir}/theme.css"), include_str!("theme_template.css")).ok();

            // Config
            std::fs::write(format!("{dir}/config.json"), r#"{"bar_height": 40, "bar_position": "top", "popup_width": 380, "popup_height": 500}"#).ok();

            let file_list = format!(
                "Shell '{name}' scaffolded at {dir}/\n\n\
                Files to edit:\n\
                  {dir}/bar.html              — bar skeleton (edit layout structure)\n\
                  {dir}/popup.html            — popup skeleton (edit panel structure)\n\
                  {dir}/theme.css             — colors and base styles\n\
                  {dir}/components/workspaces.js  — renderWorkspaces(s)\n\
                  {dir}/components/taskbar.js     — renderTaskbar(s)\n\
                  {dir}/components/clock.js       — renderClock(s)\n\
                  {dir}/components/status.js      — renderStatus(s)\n\
                  {dir}/components/tray.js        — renderTray(s)\n\
                  {dir}/panels/settings.js        — renderSettings(s)\n\
                  {dir}/panels/wifi.js            — renderWifi(s)\n\
                  {dir}/panels/power.js           — renderPower(s)\n\
                  {dir}/panels/launcher.js        — renderLauncher(s)\n\
                  {dir}/panels/config.js          — renderConfig(s)\n\n\
                Workflow:\n\
                  1. Write a component file (e.g. components/clock.js)\n\
                  2. Call hot_reload_bar(path: \"{dir}/bar.html\")\n\
                  3. See it live, iterate\n\
                  4. Move to next component\n\
                  5. Same for panels → hot_reload_popup(path: \"{dir}/popup.html\")"
            );
            json!({"content":[{"type":"text","text": file_list}]})
        }
        "validate_shell" => {
            let name = args["name"].as_str().unwrap_or("");
            let dir = format!("{}/{name}", shells_dir());
            let mut issues: Vec<String> = Vec::new();
            let mut warnings: Vec<String> = Vec::new();

            // Check required files exist
            for f in &["bar.html", "popup.html", "theme.css"] {
                if !std::path::Path::new(&format!("{dir}/{f}")).exists() {
                    issues.push(format!("MISSING: {f}"));
                }
            }

            // Read bar.html and check what render functions it calls
            let bar_html = std::fs::read_to_string(format!("{dir}/bar.html")).unwrap_or_default();
            let expected_bar_fns: Vec<&str> = vec!["renderWorkspaces", "renderTaskbar", "renderClock", "renderStatus", "renderTray"];
            for func in &expected_bar_fns {
                if bar_html.contains(func) {
                    // Check the component file exists and defines this function
                    let comp_name = func.strip_prefix("render").unwrap().to_lowercase();
                    let comp_path = format!("{dir}/components/{comp_name}.js");
                    if let Ok(content) = std::fs::read_to_string(&comp_path) {
                        if !content.contains(&format!("function {func}")) {
                            issues.push(format!("ERROR: {comp_path} does not define function {func}()"));
                        }
                        // Check for common issues
                        if content.contains("innerHTML") && !content.contains("updateList") && !content.contains("dataset.built") {
                            warnings.push(format!("WARN: {comp_name}.js uses innerHTML — may cause flicker. Consider setText()/updateList()"));
                        }
                        if content.contains("setInterval") && !content.contains("renderClock") {
                            warnings.push(format!("WARN: {comp_name}.js uses setInterval — make sure it doesn't conflict with updateState calls"));
                        }
                    } else {
                        issues.push(format!("MISSING: components/{comp_name}.js (bar.html calls {func})"));
                    }
                }
            }

            // Read popup.html and check panels
            let popup_html = std::fs::read_to_string(format!("{dir}/popup.html")).unwrap_or_default();
            let expected_panel_fns: Vec<&str> = vec!["renderSettings", "renderWifi", "renderPower", "renderLauncher", "renderConfig"];
            for func in &expected_panel_fns {
                if popup_html.contains(func) {
                    let panel_name = func.strip_prefix("render").unwrap().to_lowercase();
                    let panel_path = format!("{dir}/panels/{panel_name}.js");
                    if let Ok(content) = std::fs::read_to_string(&panel_path) {
                        if !content.contains(&format!("function {func}")) {
                            issues.push(format!("ERROR: {panel_path} does not define function {func}()"));
                        }
                        // Check panel has corresponding DOM element
                        let panel_id = format!("panel-{panel_name}");
                        if !popup_html.contains(&panel_id) {
                            issues.push(format!("ERROR: popup.html missing element with id=\"{panel_id}\" for {func}"));
                        }
                    } else if content_is_todo(&std::fs::read_to_string(&panel_path).unwrap_or_default()) {
                        warnings.push(format!("TODO: panels/{panel_name}.js is still a stub"));
                    } else {
                        issues.push(format!("MISSING: panels/{panel_name}.js (popup.html calls {func})"));
                    }
                }
            }

            // Check all JS files for syntax issues (basic checks)
            for entry in walkdir(&dir) {
                if entry.ends_with(".js") {
                    let content = std::fs::read_to_string(&entry).unwrap_or_default();
                    let basename = entry.rsplit('/').next().unwrap_or(&entry);

                    // Check for literal Nerd Font chars (should use ICONS.*)
                    let has_nerd = content.bytes().any(|b| b > 0xEF);
                    if has_nerd {
                        warnings.push(format!("WARN: {basename} contains literal multi-byte chars — may not render. Use ICONS.* constants instead"));
                    }

                    // Check balanced braces
                    let opens = content.matches('{').count();
                    let closes = content.matches('}').count();
                    if opens != closes {
                        issues.push(format!("ERROR: {basename} has unbalanced braces ({{:{opens} }}:{closes})"));
                    }

                    // Check for stub/TODO
                    if content.contains("// TODO") && content.lines().count() < 5 {
                        warnings.push(format!("STUB: {basename} is not implemented yet"));
                    }
                }
            }

            // Check charset
            if !bar_html.contains("charset") {
                issues.push("ERROR: bar.html missing <meta charset=\"utf-8\">".into());
            }
            if !popup_html.contains("charset") {
                issues.push("ERROR: popup.html missing <meta charset=\"utf-8\">".into());
            }

            let result = if issues.is_empty() && warnings.is_empty() {
                "All checks passed. Shell is ready to hot-reload.".to_string()
            } else {
                let mut out = String::new();
                if !issues.is_empty() {
                    out.push_str(&format!("Issues ({}):\n", issues.len()));
                    for i in &issues { out.push_str(&format!("  {i}\n")); }
                }
                if !warnings.is_empty() {
                    out.push_str(&format!("\nWarnings ({}):\n", warnings.len()));
                    for w in &warnings { out.push_str(&format!("  {w}\n")); }
                }
                out
            };

            json!({"content":[{"type":"text","text": result}]})
        }
        "get_api_docs" => json!({"content":[{"type":"text","text": include_str!("api_docs.md")}]}),
        "list_themes" => json!({"content":[{"type":"text","text":"Themes: mocha, macchiato, frappe, latte, tokyonight, nord, gruvbox, rosepine, onedark, dracula, solarized, flexoki\n\nSet: send({cmd:'set_theme', data:'<name>'})\nCSS vars: --bg, --bg-surface, --bg-overlay, --fg, --fg-muted, --fg-dim, --accent, --blue, --green, --red, --yellow, --peach, --teal, --pink, --mauve, --text-on-color"}]}),
        _ => json!({"content":[{"type":"text","text":format!("Unknown tool: {name}")}],"isError":true}),
    }
}

fn main() {
    eprintln!("[pulpkit-mcp] starting");
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { eprintln!("[pulpkit-mcp] stdin closed"); break };
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        eprintln!("[pulpkit-mcp] recv: {}", &trimmed[..trimmed.len().min(120)]);
        let Ok(req) = serde_json::from_str::<Value>(trimmed) else {
            eprintln!("[pulpkit-mcp] failed to parse JSON");
            continue;
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req["method"].as_str().unwrap_or("");
        let resp = match method {
            "initialize" => json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"pulpkit","version":"0.1.0"}}}),
            "notifications/initialized" => { eprintln!("[pulpkit-mcp] initialized"); continue; }
            "tools/list" => json!({"jsonrpc":"2.0","id":id,"result":{"tools":tool_definitions()}}),
            "tools/call" => {
                let name = req["params"]["name"].as_str().unwrap_or("");
                let empty = json!({});
                let args = req["params"].get("arguments").unwrap_or(&empty);
                json!({"jsonrpc":"2.0","id":id,"result":execute_tool(name, args)})
            }
            _ => {
                eprintln!("[pulpkit-mcp] unknown method: {method}");
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":format!("Unknown: {method}")}})
            }
        };
        eprintln!("[pulpkit-mcp] send: {}", method);
        let mut out = stdout.lock();
        let _ = serde_json::to_writer(&mut out, &resp);
        let _ = writeln!(out);
        let _ = out.flush();
    }
}
