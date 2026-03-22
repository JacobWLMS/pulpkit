//! Pulpkit — a Rust desktop shell framework.
//!
//! This is the main entry point for the `pulpkit` binary. It takes a shell
//! directory as its argument and runs the shell defined there.

use std::path::PathBuf;

mod ipc;
mod runtime;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    let shell_dir = match args.get(1) {
        Some(dir) => PathBuf::from(dir),
        None => {
            eprintln!("Usage: pulpkit <shell-directory>");
            std::process::exit(1);
        }
    };

    runtime::run(shell_dir)
}
