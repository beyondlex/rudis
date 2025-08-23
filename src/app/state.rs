use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::app::config::{AppConfig, ConnectionConfig};
use crate::error::AppResult;
use crate::redis::RedisConnection;
use crate::events::AppEvent;

/// Current view mode of the application
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    /// Connection list view
    ConnectionList,
    /// Database browser view
    DatabaseBrowser,
    /// Key viewer and editor
    KeyViewer,
    /// Command interface for Redis CLI
    CommandInterface,
    /// Application settings
    Settings,
    /// Help screen
    Help,
}

/// Application state container
#[derive(Debug)]
pub struct AppState {
    /// Is the application running?
    pub running: bool,
    
    /// Current view mode
    pub current_view: ViewMode,
    
    /// Active connection ID
    pub active_connection: Option<String>,
    
    /// All Redis connections
    pub connections: HashMap<String, RedisConnection>,
    
    /// Selected database number
    pub selected_database: Option<u8>,
    
    /// Currently selected key
    pub selected_key: Option<String>,
    
    /// Application configuration
    pub config: AppConfig,
    
    /// Event receiver for async operations
    pub event_rx: Option<mpsc::UnboundedReceiver<AppEvent>>,
    
    /// Event sender for async operations
    pub event_tx: mpsc::UnboundedSender<AppEvent>,
    
    /// Current status message
    pub status_message: Option<String>,
    
    /// UI state for different panels
    pub ui_state: UiState,
}

/// UI-specific state information
#[derive(Debug, Default)]
pub struct UiState {
    /// Currently focused panel
    pub focused_panel: FocusedPanel,
    
    /// Connection list state
    pub connection_list: ConnectionListState,
    
    /// Database browser state
    pub database_browser: DatabaseBrowserState,
    
    /// Key viewer state
    pub key_viewer: KeyViewerState,
    
    /// Command input state
    pub command_input: CommandInputState,
    
    /// Connection dialog state
    pub connection_dialog: ConnectionDialogState,
}

/// Which panel currently has focus
#[derive(Debug, Default, Clone, PartialEq)]
pub enum FocusedPanel {
    #[default]
    ConnectionList,
    DatabaseBrowser,
    KeyViewer,
    CommandInput,
}

/// State for connection list panel
#[derive(Debug, Default)]
pub struct ConnectionListState {
    /// Currently selected connection index
    pub selected_index: usize,
    /// Scroll offset for the list
    pub scroll_offset: usize,
}

/// State for database browser panel
#[derive(Debug, Default)]
pub struct DatabaseBrowserState {
    /// Currently selected key index
    pub selected_index: usize,
    /// Scroll offset for the key list
    pub scroll_offset: usize,
    /// Current search/filter pattern
    pub filter_pattern: String,
    /// Cached keys for current database
    pub keys: Vec<String>,
    /// Key scan cursor for pagination
    pub scan_cursor: u64,
    /// Whether we're currently loading keys
    pub loading: bool,
}

/// State for key viewer panel
#[derive(Debug, Default)]
pub struct KeyViewerState {
    /// Current key content
    pub content: Option<String>,
    /// Key metadata (type, ttl, size)
    pub metadata: Option<KeyMetadata>,
    /// Scroll position in content
    pub scroll_position: usize,
    /// Whether we're in edit mode
    pub edit_mode: bool,
}

/// Key metadata information
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub key_type: String,
    pub ttl: Option<i64>,
    pub size: usize,
    pub encoding: Option<String>,
}

/// State for command input panel
#[derive(Debug, Default)]
pub struct CommandInputState {
    /// Current command input
    pub input: String,
    /// Cursor position in input
    pub cursor_position: usize,
    /// Command history
    pub history: Vec<String>,
    /// Current history index
    pub history_index: usize,
    /// Command results
    pub results: Vec<CommandResult>,
}

/// State for connection creation dialog
#[derive(Debug, Default)]
pub struct ConnectionDialogState {
    /// Whether the dialog is open
    pub is_open: bool,
    /// Currently focused field
    pub focused_field: ConnectionDialogField,
    /// Connection form data
    pub form: ConnectionFormData,
}

/// Fields in the connection dialog
#[derive(Debug, Default, Clone, PartialEq)]
pub enum ConnectionDialogField {
    #[default]
    Name,
    Host,
    Port,
    Password,
    Database,
    Buttons, // Save/Cancel buttons
}

/// Form data for connection creation
#[derive(Debug, Default, Clone)]
pub struct ConnectionFormData {
    pub name: String,
    pub host: String,
    pub port: String,
    pub password: String,
    pub database: String,
    pub ssl: bool,
}

/// Result of a Redis command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub command: String,
    pub result: Result<String, String>,
    pub timestamp: std::time::SystemTime,
}

impl AppState {
    /// Create a new application state
    pub fn new(config: AppConfig) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Self {
            running: true,
            current_view: ViewMode::ConnectionList,
            active_connection: None,
            connections: HashMap::new(),
            selected_database: None,
            selected_key: None,
            config,
            event_rx: Some(event_rx),
            event_tx,
            status_message: None,
            ui_state: UiState::default(),
        }
    }

    /// Set the current view mode
    pub fn set_view(&mut self, view: ViewMode) {
        self.current_view = view;
    }

    /// Set status message
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Get the currently active connection
    pub fn get_active_connection(&self) -> Option<&RedisConnection> {
        self.active_connection.as_ref()
            .and_then(|id| self.connections.get(id))
    }

    /// Get mutable reference to active connection
    pub fn get_active_connection_mut(&mut self) -> Option<&mut RedisConnection> {
        self.active_connection.clone()
            .and_then(|id| self.connections.get_mut(&id))
    }

    /// Add a new Redis connection
    pub fn add_connection(&mut self, id: String, connection: RedisConnection) {
        self.connections.insert(id.clone(), connection);
        if self.active_connection.is_none() {
            self.active_connection = Some(id);
        }
    }

    /// Remove a Redis connection
    pub fn remove_connection(&mut self, id: &str) -> Option<RedisConnection> {
        let connection = self.connections.remove(id);
        if self.active_connection.as_ref() == Some(&id.to_string()) {
            self.active_connection = self.connections.keys().next().cloned();
        }
        connection
    }

    /// Set the active connection
    pub fn set_active_connection(&mut self, id: String) -> AppResult<()> {
        if self.connections.contains_key(&id) {
            self.active_connection = Some(id);
            Ok(())
        } else {
            Err(crate::error::AppError::Generic(format!("Connection {} not found", id)))
        }
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Move focus to next panel
    pub fn next_panel(&mut self) {
        self.ui_state.focused_panel = match self.ui_state.focused_panel {
            FocusedPanel::ConnectionList => FocusedPanel::DatabaseBrowser,
            FocusedPanel::DatabaseBrowser => FocusedPanel::KeyViewer,
            FocusedPanel::KeyViewer => FocusedPanel::CommandInput,
            FocusedPanel::CommandInput => FocusedPanel::ConnectionList,
        };
    }

    /// Move focus to previous panel
    pub fn previous_panel(&mut self) {
        self.ui_state.focused_panel = match self.ui_state.focused_panel {
            FocusedPanel::ConnectionList => FocusedPanel::CommandInput,
            FocusedPanel::DatabaseBrowser => FocusedPanel::ConnectionList,
            FocusedPanel::KeyViewer => FocusedPanel::DatabaseBrowser,
            FocusedPanel::CommandInput => FocusedPanel::KeyViewer,
        };
    }

    /// Open connection creation dialog
    pub fn open_connection_dialog(&mut self) {
        self.ui_state.connection_dialog.is_open = true;
        self.ui_state.connection_dialog.focused_field = ConnectionDialogField::Name;
        // Pre-fill with defaults
        self.ui_state.connection_dialog.form = ConnectionFormData {
            name: "localhost".to_string(),
            host: "127.0.0.1".to_string(),
            port: "6379".to_string(),
            password: String::new(),
            database: "0".to_string(),
            ssl: false,
        };
    }

    /// Close connection creation dialog
    pub fn close_connection_dialog(&mut self) {
        self.ui_state.connection_dialog.is_open = false;
    }

    /// Move to next field in connection dialog
    pub fn next_dialog_field(&mut self) {
        self.ui_state.connection_dialog.focused_field = match self.ui_state.connection_dialog.focused_field {
            ConnectionDialogField::Name => ConnectionDialogField::Host,
            ConnectionDialogField::Host => ConnectionDialogField::Port,
            ConnectionDialogField::Port => ConnectionDialogField::Password,
            ConnectionDialogField::Password => ConnectionDialogField::Database,
            ConnectionDialogField::Database => ConnectionDialogField::Buttons,
            ConnectionDialogField::Buttons => ConnectionDialogField::Name,
        };
    }

    /// Move to previous field in connection dialog
    pub fn previous_dialog_field(&mut self) {
        self.ui_state.connection_dialog.focused_field = match self.ui_state.connection_dialog.focused_field {
            ConnectionDialogField::Name => ConnectionDialogField::Buttons,
            ConnectionDialogField::Host => ConnectionDialogField::Name,
            ConnectionDialogField::Port => ConnectionDialogField::Host,
            ConnectionDialogField::Password => ConnectionDialogField::Port,
            ConnectionDialogField::Database => ConnectionDialogField::Password,
            ConnectionDialogField::Buttons => ConnectionDialogField::Database,
        };
    }

    /// Update current field value in connection dialog
    pub fn update_dialog_field(&mut self, ch: char) {
        let form = &mut self.ui_state.connection_dialog.form;
        match self.ui_state.connection_dialog.focused_field {
            ConnectionDialogField::Name => form.name.push(ch),
            ConnectionDialogField::Host => form.host.push(ch),
            ConnectionDialogField::Port => {
                if ch.is_ascii_digit() {
                    form.port.push(ch);
                }
            }
            ConnectionDialogField::Password => form.password.push(ch),
            ConnectionDialogField::Database => {
                if ch.is_ascii_digit() {
                    form.database.push(ch);
                }
            }
            ConnectionDialogField::Buttons => {} // No text input for buttons
        }
    }

    /// Backspace in current field
    pub fn backspace_dialog_field(&mut self) {
        let form = &mut self.ui_state.connection_dialog.form;
        match self.ui_state.connection_dialog.focused_field {
            ConnectionDialogField::Name => { form.name.pop(); }
            ConnectionDialogField::Host => { form.host.pop(); }
            ConnectionDialogField::Port => { form.port.pop(); }
            ConnectionDialogField::Password => { form.password.pop(); }
            ConnectionDialogField::Database => { form.database.pop(); }
            ConnectionDialogField::Buttons => {}
        }
    }

    /// Create connection from dialog form
    pub async fn create_connection_from_dialog(&mut self) -> AppResult<()> {
        let form = self.ui_state.connection_dialog.form.clone();
        
        // Validate form data
        if form.name.trim().is_empty() {
            return Err(crate::error::AppError::Config("Connection name cannot be empty".to_string()));
        }
        if form.host.trim().is_empty() {
            return Err(crate::error::AppError::Config("Host cannot be empty".to_string()));
        }
        
        let port: u16 = form.port.parse()
            .map_err(|_| crate::error::AppError::Config("Invalid port number".to_string()))?;
        
        let database: u8 = form.database.parse()
            .map_err(|_| crate::error::AppError::Config("Invalid database number".to_string()))?;
        
        // Create connection config
        let connection_config = ConnectionConfig {
            name: form.name.clone(),
            host: form.host.clone(),
            port,
            password: if form.password.is_empty() { None } else { Some(form.password.clone()) },
            username: None,
            database,
            ssl: form.ssl,
            timeout: 5,
        };
        
        // Create Redis connection
        let mut redis_connection = crate::redis::RedisConnection::new(connection_config.clone())?;
        
        // Try to connect
        redis_connection.connect().await?;
        
        // Generate unique ID for connection
        let connection_id = uuid::Uuid::new_v4().to_string();
        
        // Add to connections
        self.add_connection(connection_id.clone(), redis_connection);
        
        // Add to config
        self.config.add_connection(connection_id, connection_config);
        
        // Close dialog
        self.close_connection_dialog();
        
        // Set status message
        self.set_status(format!("Connected to {}", form.name));
        
        Ok(())
    }
}