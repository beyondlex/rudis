use color_eyre::Result;
use std::path::PathBuf;

// Application modules
mod app;
mod error;
mod redis;
mod events;
mod ui;
mod utils;

use app::{AppConfig, AppState, AppController};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Initialize error handling and logging
    color_eyre::install()?;
    env_logger::init();
    
    // Load configuration
    let config_path = AppConfig::default_config_path()
        .unwrap_or_else(|_| PathBuf::from("config.toml"));
    let config = AppConfig::load_from_file(&config_path)
        .unwrap_or_else(|_| {
            log::warn!("Could not load config, using defaults");
            AppConfig::default()
        });
    
    // Initialize terminal
    let terminal = ratatui::init();
    
    // Create application state and controller
    let state = AppState::new(config);
    let app = AppController::new(state);
    
    // Run application
    let result = app.run(terminal).await;
    
    // Restore terminal
    ratatui::restore();
    
    // Convert AppResult to color_eyre::Result
    match result {
        Ok(()) => Ok(()),
        Err(app_err) => Err(color_eyre::eyre::eyre!(app_err.to_string())),
    }
}