//! File watcher for Lua hot-reload.
//!
//! Monitors a shell directory for `.lua` file changes using debounced
//! filesystem notifications, so the runtime can reload scripts on the fly.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};

/// An event emitted when a watched file changes.
pub enum WatchEvent {
    /// A `.lua` file was created, modified, or removed.
    FileChanged(PathBuf),
}

/// Watches a directory tree for `.lua` file changes.
///
/// Uses `notify-debouncer-mini` under the hood so rapid successive writes
/// are collapsed into a single event (200 ms debounce window).
pub struct FileWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    pub receiver: mpsc::Receiver<WatchEvent>,
}

impl FileWatcher {
    /// Start watching `dir` recursively for `.lua` file changes.
    pub fn new(dir: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();

        let sender = tx.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(200),
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for event in events {
                        if event.path.extension().map(|e| e == "lua").unwrap_or(false) {
                            let _ = sender.send(WatchEvent::FileChanged(event.path.clone()));
                        }
                    }
                }
            },
        )?;

        debouncer.watcher().watch(dir, RecursiveMode::Recursive)?;

        Ok(Self {
            _debouncer: debouncer,
            receiver: rx,
        })
    }

    /// Check for pending file change events (non-blocking).
    pub fn poll(&self) -> Option<WatchEvent> {
        self.receiver.try_recv().ok()
    }
}
