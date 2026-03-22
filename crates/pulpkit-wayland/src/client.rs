//! Wayland client connection and event loop setup.
//!
//! Wraps smithay-client-toolkit to connect to the Wayland display,
//! bind required globals, and create a calloop event loop.

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    reexports::calloop_wayland_source::WaylandSource,
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::wlr_layer::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    backend::ObjectId,
    globals::registry_queue_init,
    protocol::{wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, Proxy, QueueHandle,
};

use crate::input::InputEvent;
use crate::output::OutputInfo;

/// Application state holding all sctk sub-states and surface tracking.
///
/// This is the central state struct passed to calloop and wayland dispatch.
/// Other crates interact with Wayland through references to this struct.
pub struct AppState {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell: LayerShell,
    pub shm: Shm,
    pub qh: QueueHandle<Self>,

    /// Tracked outputs — updated via OutputHandler callbacks.
    pub outputs: Vec<OutputInfo>,

    /// Pending layer surface configures that need to be handled.
    pub pending_configures: Vec<LayerSurfaceConfigureEvent>,

    /// Queued input events, drained each frame by the runtime.
    pub input_events: Vec<InputEvent>,

    /// Current pointer position (surface-local coordinates), if known.
    pub pointer_position: Option<(f64, f64)>,

    /// The surface the pointer is currently over, identified by ObjectId.
    pub pointer_surface: Option<ObjectId>,

    /// Active WlPointer objects, so we can release them on capability removal.
    pointers: Vec<wl_pointer::WlPointer>,

    /// Set to true when the compositor requests closing a layer surface.
    pub exit_requested: bool,
}

/// A configure event received for a layer surface.
#[derive(Debug, Clone)]
pub struct LayerSurfaceConfigureEvent {
    pub width: u32,
    pub height: u32,
}

/// Wraps the Wayland connection and calloop event loop.
pub struct WaylandClient {
    pub event_loop: calloop::EventLoop<'static, AppState>,
    pub state: AppState,
}

impl WaylandClient {
    /// Connect to the Wayland display and set up all required globals.
    ///
    /// This binds the compositor, layer-shell, shm, output, and seat globals,
    /// and inserts the Wayland event source into a calloop event loop.
    pub fn connect() -> anyhow::Result<Self> {
        let conn = Connection::connect_to_env()?;
        let (globals, event_queue) = registry_queue_init(&conn)?;
        let qh = event_queue.handle();

        let compositor_state =
            CompositorState::bind(&globals, &qh).map_err(|e| anyhow::anyhow!("{e}"))?;
        let layer_shell =
            LayerShell::bind(&globals, &qh).map_err(|e| anyhow::anyhow!("{e}"))?;
        let shm = Shm::bind(&globals, &qh).map_err(|e| anyhow::anyhow!("{e}"))?;

        let state = AppState {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &qh),
            output_state: OutputState::new(&globals, &qh),
            compositor_state,
            layer_shell,
            shm,
            qh: qh.clone(),
            outputs: Vec::new(),
            pending_configures: Vec::new(),
            input_events: Vec::new(),
            pointer_position: None,
            pointer_surface: None,
            pointers: Vec::new(),
            exit_requested: false,
        };

        let event_loop: calloop::EventLoop<'static, AppState> =
            calloop::EventLoop::try_new()?;

        // Insert the Wayland event source into calloop so events are dispatched.
        WaylandSource::new(conn, event_queue)
            .insert(event_loop.handle())
            .map_err(|e| anyhow::anyhow!("Failed to insert wayland source: {e}"))?;

        Ok(WaylandClient { event_loop, state })
    }

    /// Return a mutable reference to the calloop event loop for adding sources.
    pub fn event_loop(&mut self) -> &mut calloop::EventLoop<'static, AppState> {
        &mut self.event_loop
    }

}

// ---------------------------------------------------------------------------
// sctk handler trait implementations
// ---------------------------------------------------------------------------

impl CompositorHandler for AppState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Will be used for HiDPI support in later tasks.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // Frame callbacks will be handled by the render loop in later tasks.
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for AppState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.output_state.info(&output) {
            log::info!("New output: {:?}", info.name);
            self.outputs.push(OutputInfo::from_sctk(info, output));
        }
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.output_state.info(&output) {
            // Update existing entry or insert new one.
            if let Some(existing) = self
                .outputs
                .iter_mut()
                .find(|o| o.wl_output == output)
            {
                *existing = OutputInfo::from_sctk(info, output);
            }
        }
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.outputs.retain(|o| o.wl_output != output);
    }
}

impl LayerShellHandler for AppState {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
    ) {
        self.exit_requested = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let width = if configure.new_size.0 > 0 {
            configure.new_size.0
        } else {
            256
        };
        let height = if configure.new_size.1 > 0 {
            configure.new_size.1
        } else {
            256
        };
        self.pending_configures.push(LayerSurfaceConfigureEvent {
            width,
            height,
        });
    }
}

impl SeatHandler for AppState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
    ) {
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            match self.seat_state.get_pointer(qh, &seat) {
                Ok(pointer) => {
                    log::debug!("Pointer capability acquired");
                    self.pointers.push(pointer);
                }
                Err(e) => {
                    log::warn!("Failed to get pointer: {e}");
                }
            }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            // Release all pointers — they are no longer valid.
            self.pointers.clear();
            self.pointer_surface = None;
            self.pointer_position = None;
        }
    }

    fn remove_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
    ) {
    }
}

impl ShmHandler for AppState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl PointerHandler for AppState {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let surface_id = event.surface.id();
            let (x, y) = event.position;

            match &event.kind {
                PointerEventKind::Enter { .. } => {
                    self.pointer_surface = Some(surface_id.clone());
                    self.pointer_position = Some((x, y));
                    self.input_events.push(InputEvent::PointerEnter {
                        surface_id,
                        x,
                        y,
                    });
                }
                PointerEventKind::Leave { .. } => {
                    self.pointer_surface = None;
                    self.pointer_position = None;
                    self.input_events.push(InputEvent::PointerLeave { surface_id });
                }
                PointerEventKind::Motion { .. } => {
                    self.pointer_position = Some((x, y));
                    self.input_events.push(InputEvent::PointerMotion {
                        surface_id,
                        x,
                        y,
                    });
                }
                PointerEventKind::Press { button, .. } => {
                    self.input_events.push(InputEvent::PointerButton {
                        surface_id,
                        x,
                        y,
                        button: *button,
                        pressed: true,
                    });
                }
                PointerEventKind::Release { button, .. } => {
                    self.input_events.push(InputEvent::PointerButton {
                        surface_id,
                        x,
                        y,
                        button: *button,
                        pressed: false,
                    });
                }
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    if vertical.absolute != 0.0 {
                        self.input_events.push(InputEvent::PointerAxis {
                            surface_id: surface_id.clone(),
                            x,
                            y,
                            delta: vertical.absolute,
                            horizontal: false,
                        });
                    }
                    if horizontal.absolute != 0.0 {
                        self.input_events.push(InputEvent::PointerAxis {
                            surface_id,
                            x,
                            y,
                            delta: horizontal.absolute,
                            horizontal: true,
                        });
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// sctk delegate macros
// ---------------------------------------------------------------------------

delegate_compositor!(AppState);
delegate_output!(AppState);
delegate_shm!(AppState);
delegate_seat!(AppState);
delegate_pointer!(AppState);
delegate_layer!(AppState);
delegate_registry!(AppState);

impl ProvidesRegistryState for AppState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
