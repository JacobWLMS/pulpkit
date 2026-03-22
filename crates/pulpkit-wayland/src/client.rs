//! Wayland client connection and event loop setup.
//!
//! Wraps smithay-client-toolkit to connect to the Wayland display,
//! bind required globals, and create a calloop event loop.

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    reexports::calloop_wayland_source::WaylandSource,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        WaylandSurface,
    },
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    backend::ObjectId,
    globals::registry_queue_init,
    protocol::{
        wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface,
    },
    Connection, Proxy, QueueHandle,
};
use wayland_cursor::CursorTheme;

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
    /// Active WlKeyboard objects.
    keyboards: Vec<wl_keyboard::WlKeyboard>,
    /// Surface that currently has keyboard focus.
    pub keyboard_surface: Option<ObjectId>,

    /// Cursor theme for setting the pointer cursor.
    cursor_theme: Option<CursorTheme>,
    /// Cursor surface for rendering the cursor.
    cursor_surface: Option<wl_surface::WlSurface>,
    /// Last pointer enter serial (needed for set_cursor).
    last_pointer_serial: u32,
    /// Current cursor name (avoid redundant set_cursor calls).
    current_cursor: String,

    /// Set to true when the compositor requests closing a layer surface.
    pub exit_requested: bool,
}

impl AppState {
    /// Set the pointer cursor shape by name (e.g., "default", "pointer", "col-resize").
    /// No-op if the cursor is already set to the requested shape.
    pub fn set_cursor(&mut self, name: &str) {
        if self.current_cursor == name {
            return;
        }
        self.current_cursor = name.to_string();

        let Some(theme) = &mut self.cursor_theme else {
            return;
        };
        let Some(cursor_surface) = &self.cursor_surface else {
            return;
        };
        let Some(cursor) = theme.get_cursor(name) else {
            return;
        };

        let image = &cursor[0];
        let (hx, hy) = image.hotspot();
        let (w, h) = image.dimensions();
        let wl_buffer: &wayland_client::protocol::wl_buffer::WlBuffer = image;
        cursor_surface.attach(Some(wl_buffer), 0, 0);
        cursor_surface.damage_buffer(0, 0, w as i32, h as i32);
        cursor_surface.commit();

        // Set on all active pointers.
        for pointer in &self.pointers {
            pointer.set_cursor(self.last_pointer_serial, Some(cursor_surface), hx as i32, hy as i32);
        }
    }
}

/// A configure event received for a layer surface.
#[derive(Debug, Clone)]
pub struct LayerSurfaceConfigureEvent {
    pub surface_id: ObjectId,
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

        // Load cursor theme for setting pointer cursor.
        let cursor_theme = CursorTheme::load(&conn, shm.wl_shm().clone(), 24).ok();
        let cursor_surface = compositor_state.create_surface(&qh);

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
            keyboards: Vec::new(),
            keyboard_surface: None,
            cursor_theme,
            cursor_surface: Some(cursor_surface),
            last_pointer_serial: 0,
            current_cursor: "default".into(),
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
        layer: &LayerSurface,
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
            surface_id: layer.wl_surface().id(),
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
        // Keyboard support disabled temporarily — investigating broken pipe on NIRI
        // if capability == Capability::Keyboard {
        //     match self.seat_state.get_keyboard(qh, &seat, None) {
        //         Ok(keyboard) => {
        //             log::debug!("Keyboard capability acquired");
        //             self.keyboards.push(keyboard);
        //         }
        //         Err(e) => {
        //             log::warn!("Failed to get keyboard: {e}");
        //         }
        //     }
        // }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            self.pointers.clear();
            self.pointer_surface = None;
            self.pointer_position = None;
        }
        if capability == Capability::Keyboard {
            self.keyboards.clear();
            self.keyboard_surface = None;
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
                PointerEventKind::Enter { serial, .. } => {
                    self.pointer_surface = Some(surface_id.clone());
                    self.pointer_position = Some((x, y));
                    self.last_pointer_serial = *serial;
                    self.current_cursor = String::new(); // force re-set

                    // Set the cursor to the default arrow.
                    if let (Some(theme), Some(cursor_surface)) =
                        (&mut self.cursor_theme, &self.cursor_surface)
                    {
                        if let Some(cursor) = theme.get_cursor("default") {
                            let image = &cursor[0];
                            let (hx, hy) = image.hotspot();
                            let (w, h) = image.dimensions();
                            // CursorImageBuffer derefs to WlBuffer
                            let wl_buffer: &wayland_client::protocol::wl_buffer::WlBuffer = image;
                            cursor_surface.attach(Some(wl_buffer), 0, 0);
                            cursor_surface.damage_buffer(0, 0, w as i32, h as i32);
                            cursor_surface.commit();
                            _pointer.set_cursor(*serial, Some(cursor_surface), hx as i32, hy as i32);
                        }
                    }

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
// Keyboard handler
// ---------------------------------------------------------------------------

impl KeyboardHandler for AppState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        self.keyboard_surface = Some(surface.id());
        log::debug!("Keyboard entered surface {:?}", surface.id());
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        self.keyboard_surface = None;
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if let Some(ref surface_id) = self.keyboard_surface {
            self.input_events.push(InputEvent::KeyPress {
                surface_id: surface_id.clone(),
                raw_code: event.raw_code,
                keysym: event.keysym.raw(),
                utf8: event.utf8,
            });
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if let Some(ref surface_id) = self.keyboard_surface {
            self.input_events.push(InputEvent::KeyRelease {
                surface_id: surface_id.clone(),
                raw_code: event.raw_code,
                keysym: event.keysym.raw(),
            });
        }
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        // Treat repeats as presses.
        if let Some(ref surface_id) = self.keyboard_surface {
            self.input_events.push(InputEvent::KeyPress {
                surface_id: surface_id.clone(),
                raw_code: event.raw_code,
                keysym: event.keysym.raw(),
                utf8: event.utf8,
            });
        }
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
        _raw_modifiers: smithay_client_toolkit::seat::keyboard::RawModifiers,
        _layout: u32,
    ) {
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
delegate_keyboard!(AppState);
delegate_layer!(AppState);
delegate_registry!(AppState);

impl ProvidesRegistryState for AppState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
