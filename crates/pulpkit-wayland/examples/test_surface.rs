//! Test example: opens a layer-shell bar at the top of the screen,
//! fills the buffer with a solid teal color, runs for 2 seconds, then exits.
//!
//! Run with: cargo run -p pulpkit-wayland --example test_surface

use std::time::{Duration, Instant};

use pulpkit_wayland::{Anchor, Layer, LayerSurface, SurfaceConfig, SurfaceMargins, WaylandClient};

fn main() -> anyhow::Result<()> {

    let mut client = WaylandClient::connect()?;

    // Dispatch once to process initial globals and output events.
    client
        .event_loop
        .dispatch(Duration::from_millis(100), &mut client.state)?;

    let config = SurfaceConfig {
        width: 1920,
        height: 32,
        anchor: Anchor::Top,
        layer: Layer::Top,
        exclusive_zone: 32,
        namespace: "pulpkit-test".to_string(),
        output: None,
        margins: SurfaceMargins::default(),
    };

    let mut surface = LayerSurface::new(&mut client.state, config)?;

    // Wait for the compositor to send a configure event.
    client
        .event_loop
        .dispatch(Duration::from_millis(100), &mut client.state)?;

    // Apply any pending configure.
    if let Some(cfg) = client.state.pending_configures.pop() {
        surface.resize(cfg.width, cfg.height);
    }

    // Fill buffer with solid teal (ARGB: FF008080).
    let buf = surface.get_buffer();
    for pixel in buf.chunks_exact_mut(4) {
        // ARGB8888 in little-endian memory: [B, G, R, A]
        pixel[0] = 0x80; // B
        pixel[1] = 0x80; // G
        pixel[2] = 0x00; // R
        pixel[3] = 0xFF; // A
    }
    surface.commit();

    // Run for 2 seconds.
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        client
            .event_loop
            .dispatch(Duration::from_millis(50), &mut client.state)?;

        if client.state.exit_requested {
            break;
        }
    }

    println!("Test surface example finished.");
    Ok(())
}
