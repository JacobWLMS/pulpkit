# Pulpkit

**A Wayland desktop shell framework where your UI is just HTML/CSS/JS.**

Pulpkit is a Rust-powered shell framework for Wayland compositors. It runs your bar, popups, notifications, and lock screen as WebKitGTK webviews — write them in HTML/CSS/JS and get instant access to 187 reactive system state fields.

## Why Pulpkit?

- **Web technologies for desktop UI** — Use the tools you already know
- **53 reactive watchers** — Audio, network, bluetooth, media, GPU, gaming, containers, and more
- **Sub-100ms latency** — Event-driven, not polling
- **Built-in notification daemon** — No mako or dunst needed
- **Fully themable** — 12 color themes, CSS variables everywhere
- **MCP server** — AI-assisted shell development with Claude Code

## Quick Example

Your shell is just HTML that receives state:

```javascript
function updateState(s) {
  document.getElementById('volume').textContent = s.vol + '%';
  document.getElementById('wifi').textContent = s.wifi || 'Disconnected';
  document.getElementById('cpu').textContent = s.cpu + '%';
}
```

[Get Started](getting-started/installation.md){ .md-button .md-button--primary }
[View on GitHub](https://github.com/JacobWLMS/pulpkit){ .md-button }
