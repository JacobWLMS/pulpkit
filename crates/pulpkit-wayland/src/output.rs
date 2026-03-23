//! Output (monitor) tracking.
//!
//! Uses sctk's `OutputState` to track connected monitors and exposes
//! a simplified `OutputInfo` struct for the rest of the shell.

use smithay_client_toolkit::output::OutputInfo as SctkOutputInfo;
use wayland_client::protocol::wl_output;

/// Information about a connected monitor.
#[derive(Debug, Clone)]
pub struct OutputInfo {
    /// Human-readable name (e.g. "HDMI-A-1"). May be empty if the compositor
    /// does not report output names.
    pub name: String,

    /// Width of the current mode in pixels.
    pub width: u32,

    /// Height of the current mode in pixels.
    pub height: u32,

    /// Integer scale factor for this output.
    pub scale: i32,

    /// The underlying `wl_output` object, needed when targeting a specific
    /// output for layer surface creation.
    pub wl_output: wl_output::WlOutput,
}

impl OutputInfo {
    /// Construct from sctk's OutputInfo plus the wl_output handle.
    pub(crate) fn from_sctk(info: SctkOutputInfo, wl_output: wl_output::WlOutput) -> Self {
        // Find the current mode to get dimensions.
        let (width, height) = info
            .modes
            .iter()
            .find(|m| m.current)
            .map(|m| (m.dimensions.0 as u32, m.dimensions.1 as u32))
            .unwrap_or((0, 0));

        OutputInfo {
            name: info.name.unwrap_or_default(),
            width,
            height,
            scale: info.scale_factor,
            wl_output,
        }
    }

    /// Logical width (physical pixels / scale factor).
    /// This is what the compositor uses for surface positioning.
    pub fn logical_width(&self) -> u32 {
        if self.scale > 0 { self.width / self.scale as u32 } else { self.width }
    }

    /// Logical height (physical pixels / scale factor).
    pub fn logical_height(&self) -> u32 {
        if self.scale > 0 { self.height / self.scale as u32 } else { self.height }
    }
}
