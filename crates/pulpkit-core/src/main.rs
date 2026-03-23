fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    let shell_dir = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            // Default to examples/minimal
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_default();
            exe_dir.join("examples/minimal")
        });

    if let Err(err) = pulpkit_core::run(shell_dir) {
        log::error!("Fatal error: {err}");
        std::process::exit(1);
    }
}
