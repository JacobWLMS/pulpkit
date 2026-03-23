//! Subscription manager — diffs subscription lists, starts/stops sources.

use std::time::Duration;

use calloop::channel::Sender;
use calloop::timer::{TimeoutAction, Timer};
use calloop::LoopHandle;

/// A message produced by a subscription.
#[derive(Debug, Clone)]
pub struct SubMessage {
    pub msg_type: String,
    pub data: Option<String>,
}

/// Opaque handle to an active subscription for cleanup.
pub enum SubHandle {
    Timer(calloop::RegistrationToken),
    Channel(calloop::RegistrationToken),
    Process {
        token: calloop::RegistrationToken,
        child_pid: Option<u32>,
    },
}

/// An active subscription being managed.
struct ActiveSub {
    kind: String,      // variant name
    msg_name: String,  // message name (the matching key)
    handle: SubHandle,
}

/// Manages the lifecycle of subscriptions: start, stop, reconcile.
pub struct SubscriptionManager {
    active: Vec<ActiveSub>,
    sender: Sender<SubMessage>,
}

impl SubscriptionManager {
    pub fn new(sender: Sender<SubMessage>) -> Self {
        Self {
            active: Vec::new(),
            sender,
        }
    }

    /// Get the sender for creating new subscription sources.
    pub fn sender(&self) -> &Sender<SubMessage> {
        &self.sender
    }

    /// Number of active subscriptions.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Start an interval timer subscription.
    pub fn start_interval<D: 'static>(
        &mut self,
        ms: u64,
        msg_name: String,
        handle: &LoopHandle<'static, D>,
    ) {
        let sender = self.sender.clone();
        let name = msg_name.clone();
        let token = handle.insert_source(
            Timer::from_duration(Duration::from_millis(ms)),
            move |_deadline, _metadata, _data| {
                let _ = sender.send(SubMessage {
                    msg_type: name.clone(),
                    data: None,
                });
                TimeoutAction::ToDuration(Duration::from_millis(ms))
            },
        ).expect("failed to insert interval timer");

        self.active.push(ActiveSub {
            kind: "interval".into(),
            msg_name,
            handle: SubHandle::Timer(token),
        });
    }

    /// Start a one-shot timeout subscription.
    pub fn start_timeout<D: 'static>(
        &mut self,
        ms: u64,
        msg_name: String,
        handle: &LoopHandle<'static, D>,
    ) {
        let sender = self.sender.clone();
        let name = msg_name.clone();
        let token = handle.insert_source(
            Timer::from_duration(Duration::from_millis(ms)),
            move |_deadline, _metadata, _data| {
                let _ = sender.send(SubMessage {
                    msg_type: name.clone(),
                    data: None,
                });
                TimeoutAction::Drop
            },
        ).expect("failed to insert timeout timer");

        self.active.push(ActiveSub {
            kind: "timeout".into(),
            msg_name,
            handle: SubHandle::Timer(token),
        });
    }

    /// Start a stream subscription (subprocess stdout).
    pub fn start_stream(
        &mut self,
        cmd: String,
        msg_name: String,
        channel_token: calloop::RegistrationToken,
        child_pid: Option<u32>,
    ) {
        self.active.push(ActiveSub {
            kind: "stream".into(),
            msg_name,
            handle: SubHandle::Process { token: channel_token, child_pid },
        });
    }

    /// Start an exec (one-shot command) subscription.
    pub fn start_exec(
        &mut self,
        msg_name: String,
        channel_token: calloop::RegistrationToken,
    ) {
        self.active.push(ActiveSub {
            kind: "exec".into(),
            msg_name,
            handle: SubHandle::Channel(channel_token),
        });
    }

    /// Add a generic channel-based subscription (IPC, config_watch).
    pub fn add_channel_sub(
        &mut self,
        kind: &str,
        msg_name: String,
        token: calloop::RegistrationToken,
    ) {
        self.active.push(ActiveSub {
            kind: kind.into(),
            msg_name,
            handle: SubHandle::Channel(token),
        });
    }

    /// Stop a subscription by index, removing it from the event loop.
    pub fn stop<D: 'static>(&mut self, index: usize, loop_handle: &LoopHandle<'static, D>) {
        if index >= self.active.len() {
            return;
        }
        let sub = self.active.remove(index);
        match sub.handle {
            SubHandle::Timer(token) | SubHandle::Channel(token) => {
                loop_handle.remove(token);
            }
            SubHandle::Process { token, child_pid } => {
                loop_handle.remove(token);
                if let Some(pid) = child_pid {
                    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                }
            }
        }
    }

    /// Stop all subscriptions.
    pub fn stop_all<D: 'static>(&mut self, loop_handle: &LoopHandle<'static, D>) {
        while !self.active.is_empty() {
            self.stop(0, loop_handle);
        }
    }

    /// Find an active subscription by (kind, msg_name).
    pub fn find(&self, kind: &str, msg_name: &str) -> Option<usize> {
        self.active.iter().position(|s| s.kind == kind && s.msg_name == msg_name)
    }

    /// Check if a subscription with the given key exists.
    pub fn has(&self, kind: &str, msg_name: &str) -> bool {
        self.find(kind, msg_name).is_some()
    }
}
