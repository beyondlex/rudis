use crate::error::AppResult;
use crate::app::config::ConnectionConfig;
use crate::app::states::{ConnectionDialogField, ConnectionFormData};
use super::state_core::AppState;

impl AppState {
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
} 