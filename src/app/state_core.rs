use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::app::config::{AppConfig, ConnectionConfig};
use crate::error::AppResult;
use crate::redis::RedisConnection;
use crate::events::AppEvent;
use crate::app::states::{
    ViewMode, UiState, KeyInfo, FocusedPanel
};

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
    
    /// Flag to indicate if a full terminal redraw is needed
    pub needs_full_redraw: bool,
}

/// Key metadata information
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub key_type: String,
    pub ttl: Option<i64>,
    pub size: usize,
    pub encoding: Option<String>,
}

/// Hash field editing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum HashEditMode {
    #[default]
    None,
    Field,    // Editing field name
    Value,    // Editing field value
    NewField, // Adding new field
}

/// List element editing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ListEditMode {
    #[default]
    None,
    Element,  // Editing existing element
    Insert,   // Inserting new element
    Append,   // Appending new element
}

/// Set member editing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SetEditMode {
    #[default]
    None,
    Add,      // Adding new member
    Remove,   // Removing member (confirmation)
}

/// Sorted set editing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ZSetEditMode {
    #[default]
    None,
    Add,          // Adding new member with score
    Remove,       // Removing member (confirmation)
    UpdateScore,  // Updating score of existing member
}

/// Stream viewing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum StreamViewMode {
    #[default]
    List,    // List view showing entry IDs and summary
    Detail,  // Detail view showing selected entry fields
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
            needs_full_redraw: false,
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
    
    /// Request a full terminal redraw (needed after external editor)
    pub fn request_full_redraw(&mut self) {
        self.needs_full_redraw = true;
    }
    
    /// Check if full redraw is needed and reset the flag
    pub fn take_full_redraw_flag(&mut self) -> bool {
        let needs_redraw = self.needs_full_redraw;
        self.needs_full_redraw = false;
        needs_redraw
    }


} 