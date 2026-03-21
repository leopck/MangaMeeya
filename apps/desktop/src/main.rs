use anyhow::Result;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("MangaMeeya starting...");

    // Load config
    let config_path = dirs_config_path();
    let config = mm_config::Config::load(&config_path).unwrap_or_default();
    tracing::info!("Config loaded: {}x{}", config.window.width, config.window.height);

    // TODO: Phase 2 - Initialize eframe + egui window
    // TODO: Phase 2 - Create wgpu renderer
    // TODO: Phase 2 - Enter event loop

    tracing::info!("MangaMeeya ready. (UI not yet implemented - Phase 2)");
    Ok(())
}

fn dirs_config_path() -> std::path::PathBuf {
    if let Some(config_dir) = dirs_next() {
        config_dir.join("mangameeya").join("config.toml")
    } else {
        std::path::PathBuf::from("config.toml")
    }
}

fn dirs_next() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(std::path::PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| std::path::PathBuf::from(h).join(".config"))
            })
    }
}
