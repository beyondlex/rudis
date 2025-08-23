use crossterm::event::KeyEvent;
use crate::redis::ConnectionStatus;

/// Application events for async communication
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// User input events
    KeyPressed(KeyEvent),
    
    /// Redis connection events
    ConnectionStatusChanged {
        connection_id: String,
        status: ConnectionStatus,
    },
    
    /// Database selection changed
    DatabaseSelected {
        connection_id: String,
        database: u8,
    },
    
    /// Key selection changed
    KeySelected {
        key: String,
    },
    
    /// Keys loaded from Redis
    KeysLoaded {
        keys: Vec<String>,
        cursor: u64,
        total_count: Option<usize>,
    },
    
    /// Key content loaded
    KeyContentLoaded {
        key: String,
        content: String,
        key_type: String,
        ttl: Option<i64>,
    },
    
    /// Command execution result
    CommandExecuted {
        command: String,
        result: Result<String, String>,
    },
    
    /// Refresh data request
    RefreshData,
    
    /// Status message update
    StatusMessage(String),
    
    /// Error occurred
    Error(String),
    
    /// Application quit request
    Quit,
}

/// UI input events
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Character input
    Char(char),
    
    /// Special key input
    Key(KeyEvent),
    
    /// Mouse event
    Mouse,
    
    /// Window resize
    Resize(u16, u16),
}