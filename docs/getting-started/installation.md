# Installation

## Dependencies

### Required

Pulpkit needs GTK4 with layer-shell support and WebKitGTK for rendering surfaces.

=== "Arch / CachyOS"

    ```bash
    sudo pacman -S gtk4 gtk4-layer-shell webkit2gtk-6.0
    ```

=== "Other distros"

    Install the equivalent packages for your distribution:

    - `gtk4` (>= 4.12)
    - `gtk4-layer-shell` (>= 1.0)
    - `webkit2gtk-6.0` (WebKitGTK for GTK4)

### Optional

These tools are used by specific watchers. Pulpkit runs without them, but the corresponding state fields will remain at their defaults.

| Package | Used for |
|---------|----------|
| `pipewire` + `wireplumber` | Audio volume, mute, sinks, sources, per-app streams |
| `brightnessctl` | Display brightness control |
| `networkmanager` | WiFi state, network scanning, VPN detection |
| `bluez` | Bluetooth power and device tracking |
| `playerctl` | MPRIS media metadata (title, artist, album art) |
| `upower` | Battery level, AC state, power draw |
| `inotifywait` (`inotify-tools`) | File-based watchers (trash, drives) |
| `udevadm` (`systemd`) | Removable drive detection |
| `notify-send` (`libnotify`) | Testing notification daemon |

Install them all on Arch/CachyOS:

```bash
sudo pacman -S pipewire wireplumber brightnessctl networkmanager \
  bluez playerctl upower inotify-tools libnotify
```

## Build from Source

Pulpkit is written in Rust. You need `cargo` (Rust 1.75+).

```bash
git clone https://github.com/JacobWLMS/pulpkit.git
cd pulpkit
cargo build --release -p pulpkit-webshell-poc
```

The binary lands at `./target/release/pulpkit-webshell-poc`.

!!! tip "Release builds matter"
    Always use `--release` for real usage. Debug builds have noticeably higher CPU usage due to unoptimized serialization of the 187-field state struct.

## Wayland Compositor Requirement

Pulpkit requires a running Wayland compositor that supports the `wlr-layer-shell` protocol. Compatible compositors include:

- **Niri**
- **Hyprland**
- **Sway**
- **river**
- **labwc**

!!! warning "X11 is not supported"
    Pulpkit uses GTK4 layer-shell for surface placement. It will not work on X11 or XWayland.

## Verify Installation

```bash
./target/release/pulpkit-webshell-poc zenith
```

You should see a bar appear at the top of your screen. If the bar shows workspace dots and a clock, everything is working.
