use std::fmt;

/// Application-specific error types
#[derive(Debug)]
pub enum AppError {
    /// Redis connection errors
    Redis(redis::RedisError),
    /// IO errors (file operations, network)
    Io(std::io::Error),
    /// Configuration errors
    Config(String),
    /// Serialization errors
    Serialization(String),
    /// UI rendering errors
    Ui(String),
    /// Generic application errors
    Generic(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Redis(err) => write!(f, "Redis error: {}", err),
            AppError::Io(err) => write!(f, "IO error: {}", err),
            AppError::Config(msg) => write!(f, "Configuration error: {}", msg),
            AppError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            AppError::Ui(msg) => write!(f, "UI error: {}", msg),
            AppError::Generic(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::Redis(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(err: toml::ser::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(err: serde_yaml::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

impl From<csv::Error> for AppError {
    fn from(err: csv::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

/// Application result type
pub type AppResult<T> = Result<T, AppError>;