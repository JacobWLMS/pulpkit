//! Pulpkit plugin system

pub mod loader;

/// ABI version — checked at load time
pub const ABI_VERSION: u32 = 1;

/// Trait every plugin implements
pub trait PulpkitPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn abi_version(&self) -> u32 {
        ABI_VERSION
    }
    fn init(&self, ctx: &mut PluginContext) -> anyhow::Result<()>;
    fn shutdown(&self) -> anyhow::Result<()>;
}

/// Values that can be stored in signals exposed to Lua
#[derive(Debug, Clone)]
pub enum SignalValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    String(String),
    List(Vec<SignalValue>),
    Nil,
}

/// Handle to a registered signal — just an ID, safe to send across threads
#[derive(Debug, Clone)]
pub struct SignalHandle {
    pub namespace: String,
    pub name: String,
}

/// Signal update message sent from plugin async tasks to main thread
pub struct SignalUpdate {
    pub handle: SignalHandle,
    pub value: SignalValue,
}

/// Provided to plugins during init for registering signals.
/// Lives on the main thread only.
pub struct PluginContext {
    namespace: String,
    runtime_handle: tokio::runtime::Handle,
    update_sender: tokio::sync::mpsc::UnboundedSender<SignalUpdate>,
}

impl PluginContext {
    pub fn new(
        namespace: String,
        runtime_handle: tokio::runtime::Handle,
        update_sender: tokio::sync::mpsc::UnboundedSender<SignalUpdate>,
    ) -> Self {
        Self {
            namespace,
            runtime_handle,
            update_sender,
        }
    }

    /// Register a signal. Returns a handle (ID) that can be sent to async tasks.
    pub fn register_signal(&mut self, name: &str, _initial: SignalValue) -> SignalHandle {
        // Note: actual Signal<SignalValue> creation on the main thread happens
        // in the core runtime, not here. This just returns the handle.
        SignalHandle {
            namespace: self.namespace.clone(),
            name: name.to_string(),
        }
    }

    pub fn runtime(&self) -> &tokio::runtime::Handle {
        &self.runtime_handle
    }

    pub fn signal_sender(&self) -> tokio::sync::mpsc::UnboundedSender<SignalUpdate> {
        self.update_sender.clone()
    }
}

pub use loader::PluginLoader;
