use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::error::{AppError, AppResult};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Redis connections configuration
    pub connections: HashMap<String, ConnectionConfig>,
    /// UI configuration
    pub ui: UiConfig,
    /// Application preferences
    pub preferences: Preferences,
}

/// Redis connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection name/identifier
    pub name: String,
    /// Redis host
    pub host: String,
    /// Redis port
    pub port: u16,
    /// Authentication password
    pub password: Option<String>,
    /// Username for Redis ACL
    pub username: Option<String>,
    /// Database number
    pub database: u8,
    /// Use SSL/TLS
    pub ssl: bool,
    /// Connection timeout in seconds
    pub timeout: u64,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Color theme
    pub theme: String,
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Auto-refresh interval in seconds
    pub auto_refresh: Option<u64>,
}

/// Application preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    /// Maximum number of keys to display per page
    pub keys_per_page: usize,
    /// Default key scan pattern
    pub default_scan_pattern: String,
    /// Remember last selected database
    pub remember_database: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            connections: HashMap::new(),
            ui: UiConfig {
                theme: "default".to_string(),
                show_line_numbers: true,
                auto_refresh: Some(30),
            },
            preferences: Preferences {
                keys_per_page: 100,
                default_scan_pattern: "*".to_string(),
                remember_database: true,
            },
        }
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            name: "localhost".to_string(),
            host: "127.0.0.1".to_string(),
            port: 6379,
            password: None,
            username: None,
            database: 0,
            ssl: false,
            timeout: 5,
        }
    }
}

impl AppConfig {
    /// Load configuration from file
    pub fn load_from_file(path: &PathBuf) -> AppResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)
            .map_err(AppError::Io)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &PathBuf) -> AppResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(AppError::Io)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)
            .map_err(AppError::Io)?;
        Ok(())
    }

    /// Get default configuration file path
    pub fn default_config_path() -> AppResult<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| AppError::Config("Could not find config directory".to_string()))?;
        Ok(config_dir.join("rudis").join("config.toml"))
    }

    /// Add a new connection configuration
    pub fn add_connection(&mut self, id: String, config: ConnectionConfig) {
        self.connections.insert(id, config);
    }

    /// Remove a connection configuration
    pub fn remove_connection(&mut self, id: &str) -> Option<ConnectionConfig> {
        self.connections.remove(id)
    }

    /// Get connection configuration by ID
    pub fn get_connection(&self, id: &str) -> Option<&ConnectionConfig> {
        self.connections.get(id)
    }
}