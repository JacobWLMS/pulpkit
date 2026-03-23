//! Pulpkit core — Elm runtime, event loop, surface management.

pub mod event_loop;
pub mod hover;
pub mod runtime;
pub mod surfaces;

/// Run the shell from the given directory.
pub fn run(shell_dir: std::path::PathBuf) -> anyhow::Result<()> {
    runtime::run(shell_dir)
}
