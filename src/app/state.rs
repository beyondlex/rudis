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
    /// Available databases
    pub databases: Vec<u8>,
    /// Currently selected database
    pub selected_database: u8,
    /// Currently selected key index
    pub selected_key_index: usize,
    /// Scroll offset for the key list
    pub scroll_offset: usize,
    /// Current search/filter pattern
    pub filter_pattern: String,
    /// Whether we're in search mode
    pub search_mode: bool,
    /// Cached keys for current database
    pub keys: Vec<KeyInfo>,
    /// Key scan cursor for pagination
    pub scan_cursor: u64,
    /// Whether we're currently loading keys
    pub loading: bool,
    /// Whether we've loaded all keys (scan cursor = 0)
    pub scan_complete: bool,
    /// Total key count for current database
    pub total_keys: Option<usize>,
}

/// Information about a Redis key
#[derive(Debug, Clone)]
pub struct KeyInfo {
    /// Key name
    pub name: String,
    /// Key type (string, hash, list, set, zset, stream)
    pub key_type: Option<String>,
    /// TTL in seconds (-1 for no expiry, -2 for key doesn't exist)
    pub ttl: Option<i64>,
    /// Key size/length
    pub size: Option<usize>,
    /// Whether this key matches current filter
    pub matches_filter: bool,
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
        let connection_id_for_config = connection_id.clone();
        let connection_id_for_event = connection_id.clone();
        
        // Add to connections
        self.add_connection(connection_id, redis_connection);
        
        // Add to config
        self.config.add_connection(connection_id_for_config, connection_config);
        
        // Close dialog
        self.close_connection_dialog();
        
        // Set status message
        self.set_status(format!("Connected to {}", form.name));
        
        // Trigger database browser initialization
        let _ = self.event_tx.send(crate::events::AppEvent::ConnectionStatusChanged {
            connection_id: connection_id_for_event,
            status: crate::redis::ConnectionStatus::Connected,
        });
        
        Ok(())
    }
    
    /// Load available databases for active connection
    pub async fn load_databases(&mut self) -> AppResult<()> {
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.get_databases().await {
                Ok(databases) => {
                    self.ui_state.database_browser.databases = databases;
                    self.set_status(format!("Found {} databases", self.ui_state.database_browser.databases.len()));
                }
                Err(err) => {
                    self.set_status(format!("Failed to load databases: {}", err));
                }
            }
        }
        Ok(())
    }
    
    /// Select a database
    pub async fn select_database(&mut self, db_num: u8) -> AppResult<()> {
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.select_database(db_num).await {
                Ok(()) => {
                    self.ui_state.database_browser.selected_database = db_num;
                    self.selected_database = Some(db_num);
                    // Clear current keys and reset scanning
                    self.ui_state.database_browser.keys.clear();
                    self.ui_state.database_browser.scan_cursor = 0;
                    self.ui_state.database_browser.scan_complete = false;
                    self.ui_state.database_browser.selected_key_index = 0;
                    // Load keys for the new database
                    self.load_keys().await?;
                    self.set_status(format!("Selected database {}", db_num));
                }
                Err(err) => {
                    self.set_status(format!("Failed to select database {}: {}", db_num, err));
                }
            }
        }
        Ok(())
    }
    
    /// Load keys from current database
    pub async fn load_keys(&mut self) -> AppResult<()> {
        if self.ui_state.database_browser.loading {
            return Ok(()); // Already loading
        }
        
        self.ui_state.database_browser.loading = true;
        
        // Extract values to avoid borrowing conflicts
        let pattern = if self.ui_state.database_browser.filter_pattern.is_empty() {
            "*".to_string()
        } else {
            format!("*{}*", self.ui_state.database_browser.filter_pattern)
        };
        
        let scan_cursor = self.ui_state.database_browser.scan_cursor;
        let keys_per_page = self.config.preferences.keys_per_page;
        
        // Get connection ID for later reference
        let connection_id = self.active_connection.clone();
        
        // Perform scan operation
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.scan_keys(scan_cursor, &pattern, keys_per_page).await {
                Ok((new_cursor, key_names)) => {
                    // Update scan state
                    self.ui_state.database_browser.scan_cursor = new_cursor;
                    if new_cursor == 0 {
                        self.ui_state.database_browser.scan_complete = true;
                    }
                    
                    if !key_names.is_empty() {
                        // For now, create KeyInfo without type information
                        // We'll add type detection as a separate operation
                        let mut key_infos = Vec::new();
                        for key_name in key_names {
                            let key_info = KeyInfo {
                                name: key_name,
                                key_type: None, // Will be loaded separately
                                ttl: None,      // Will be loaded separately
                                size: None,
                                matches_filter: true,
                            };
                            key_infos.push(key_info);
                        }
                        
                        // Append new keys to existing ones
                        self.ui_state.database_browser.keys.extend(key_infos);
                        
                        self.set_status(format!(
                            "Loaded {} keys", 
                            self.ui_state.database_browser.keys.len()
                        ));
                        
                        // Load types and TTLs for the first few keys asynchronously
                        self.load_key_details().await?;
                    } else {
                        self.set_status("No keys found".to_string());
                    }
                }
                Err(err) => {
                    self.set_status(format!("Failed to scan keys: {}", err));
                }
            }
        }
        
        self.ui_state.database_browser.loading = false;
        Ok(())
    }
    
    /// Load type and TTL information for keys that don't have it yet
    pub async fn load_key_details(&mut self) -> AppResult<()> {
        // Load details for up to 10 keys at a time to avoid blocking UI
        let mut keys_to_process = Vec::new();
        let mut indices_to_update = Vec::new();
        
        for (idx, key_info) in self.ui_state.database_browser.keys.iter().enumerate() {
            if key_info.key_type.is_none() && keys_to_process.len() < 10 {
                keys_to_process.push(key_info.name.clone());
                indices_to_update.push(idx);
            }
        }
        
        if keys_to_process.is_empty() {
            return Ok(());
        }
        
        // Load key information
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.get_keys_info(&keys_to_process).await {
                Ok(key_infos_data) => {
                    let mut types_loaded = 0;
                    let mut ttls_loaded = 0;
                    
                    // Update the key information
                    for ((_, key_type, ttl), &idx) in key_infos_data.iter().zip(indices_to_update.iter()) {
                        if let Some(key_info) = self.ui_state.database_browser.keys.get_mut(idx) {
                            key_info.key_type = key_type.clone();
                            key_info.ttl = *ttl;
                            
                            if key_type.is_some() {
                                types_loaded += 1;
                            }
                            if ttl.is_some() {
                                ttls_loaded += 1;
                            }
                        }
                    }
                    
                    if types_loaded > 0 || ttls_loaded > 0 {
                        self.set_status(format!(
                            "Loaded details: {} types, {} TTLs", 
                            types_loaded, ttls_loaded
                        ));
                    }
                }
                Err(err) => {
                    self.set_status(format!("Failed to load key details: {}", err));
                }
            }
        }
        
        Ok(())
    }
    
    /// Load more keys (pagination)
    pub async fn load_more_keys(&mut self) -> AppResult<()> {
        if !self.ui_state.database_browser.scan_complete {
            self.load_keys().await?
        }
        Ok(())
    }
    
    /// Schedule key loading without blocking UI - for responsive navigation
    pub fn schedule_key_loading(&mut self) -> AppResult<()> {
        if !self.ui_state.database_browser.loading && !self.ui_state.database_browser.scan_complete {
            // Send an async event to load more keys in the background
            let _ = self.event_tx.send(crate::events::AppEvent::RefreshData);
        }
        Ok(())
    }
    
    /// Select next key in the browser - optimized for performance
    pub fn select_next_key(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        if !browser.keys.is_empty() {
            let old_index = browser.selected_key_index;
            browser.selected_key_index = (browser.selected_key_index + 1).min(browser.keys.len() - 1);
            
            // Only update if index actually changed
            if old_index != browser.selected_key_index {
                // Adjust scroll offset if needed
                let visible_count = 10; // Number of keys visible at once
                if browser.selected_key_index >= browser.scroll_offset + visible_count {
                    browser.scroll_offset = browser.selected_key_index - visible_count + 1;
                }
                
                // Update selected key - use reference to avoid cloning when possible
                if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                    self.selected_key = Some(key_info.name.clone());
                }
            }
        }
    }
    
    /// Select previous key in the browser - optimized for performance
    pub fn select_previous_key(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        if browser.selected_key_index > 0 {
            let old_index = browser.selected_key_index;
            browser.selected_key_index -= 1;
            
            // Only update if index actually changed
            if old_index != browser.selected_key_index {
                // Adjust scroll offset if needed
                if browser.selected_key_index < browser.scroll_offset {
                    browser.scroll_offset = browser.selected_key_index;
                }
                
                // Update selected key - use reference to avoid cloning when possible
                if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                    self.selected_key = Some(key_info.name.clone());
                }
            }
        }
    }
    
    /// Select key by offset for efficient page navigation
    pub fn select_key_by_offset(&mut self, offset: i32) {
        let browser = &mut self.ui_state.database_browser;
        if browser.keys.is_empty() {
            return;
        }
        
        let old_index = browser.selected_key_index;
        let new_index = if offset < 0 {
            browser.selected_key_index.saturating_sub((-offset) as usize)
        } else {
            (browser.selected_key_index + offset as usize).min(browser.keys.len() - 1)
        };
        
        if old_index != new_index {
            browser.selected_key_index = new_index;
            
            // Adjust scroll offset for the new position
            let visible_count = 10;
            if browser.selected_key_index >= browser.scroll_offset + visible_count {
                browser.scroll_offset = browser.selected_key_index - visible_count + 1;
            } else if browser.selected_key_index < browser.scroll_offset {
                browser.scroll_offset = browser.selected_key_index;
            }
            
            // Update selected key
            if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                self.selected_key = Some(key_info.name.clone());
            }
        }
    }
    
    /// Set filter pattern for key search
    pub async fn set_key_filter(&mut self, pattern: String) -> AppResult<()> {
        self.ui_state.database_browser.filter_pattern = pattern;
        // Reset scanning and reload keys with new filter
        self.ui_state.database_browser.keys.clear();
        self.ui_state.database_browser.scan_cursor = 0;
        self.ui_state.database_browser.scan_complete = false;
        self.ui_state.database_browser.selected_key_index = 0;
        self.load_keys().await
    }
    
    /// Get currently selected key info
    pub fn get_selected_key_info(&self) -> Option<&KeyInfo> {
        let browser = &self.ui_state.database_browser;
        browser.keys.get(browser.selected_key_index)
    }
    
    /// Enter search mode for key filtering
    pub fn enter_search_mode(&mut self) {
        self.ui_state.database_browser.search_mode = true;
        self.ui_state.database_browser.filter_pattern.clear();
    }
    
    /// Exit search mode
    pub fn exit_search_mode(&mut self) {
        self.ui_state.database_browser.search_mode = false;
        if !self.ui_state.database_browser.filter_pattern.is_empty() {
            // Clear filter and reload all keys
            self.ui_state.database_browser.filter_pattern.clear();
            // Reset scanning state
            self.ui_state.database_browser.keys.clear();
            self.ui_state.database_browser.scan_cursor = 0;
            self.ui_state.database_browser.scan_complete = false;
            self.ui_state.database_browser.selected_key_index = 0;
        }
    }
    
    /// Add character to search pattern
    pub fn add_search_char(&mut self, ch: char) {
        if self.ui_state.database_browser.search_mode {
            self.ui_state.database_browser.filter_pattern.push(ch);
        }
    }
    
    /// Remove last character from search pattern
    pub fn backspace_search(&mut self) {
        if self.ui_state.database_browser.search_mode {
            self.ui_state.database_browser.filter_pattern.pop();
        }
    }
    
    /// Apply current search filter
    pub async fn apply_search_filter(&mut self) -> AppResult<()> {
        if self.ui_state.database_browser.search_mode {
            // Reset scanning state and search with new pattern
            self.ui_state.database_browser.keys.clear();
            self.ui_state.database_browser.scan_cursor = 0;
            self.ui_state.database_browser.scan_complete = false;
            self.ui_state.database_browser.selected_key_index = 0;
            // Load keys with filter
            self.load_keys().await?;
            // Exit search mode after applying
            self.ui_state.database_browser.search_mode = false;
        }
        Ok(())
    }
}