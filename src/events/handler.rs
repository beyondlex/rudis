use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::app::{AppState, ConnectionDialogField, FocusedPanel};
use crate::error::AppResult;
use crate::events::AppEvent;

/// Event handler for the application
pub struct EventHandler;

impl EventHandler {
    /// Handles crossterm events (user input)
    pub async fn handle_crossterm_events(app_state: &mut AppState) -> AppResult<()> {
        match crossterm::event::read().map_err(|e| crate::error::AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                Self::handle_key_event(app_state, key).await?
            }
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles application events from async operations
    pub async fn handle_app_event(app_state: &mut AppState, event: AppEvent) -> AppResult<()> {
        match event {
            AppEvent::KeyPressed(key) => Self::handle_key_event(app_state, key).await?,
            AppEvent::ConnectionStatusChanged { connection_id, status } => {
                if let Some(connection) = app_state.connections.get_mut(&connection_id) {
                    connection.status = status.clone();
                    // If connection is now established, initialize database browser
                    if matches!(status, crate::redis::ConnectionStatus::Connected) {
                        Self::initialize_database_browser(app_state).await?;
                    }
                }
            }
            AppEvent::RefreshData => {
                // Handle background key loading for responsive navigation
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    let _ = app_state.load_more_keys().await;
                }
            }
            AppEvent::StatusMessage(msg) => {
                app_state.set_status(msg);
            }
            AppEvent::Error(err) => {
                app_state.set_status(format!("Error: {}", err));
            }
            AppEvent::Quit => {
                app_state.quit();
            }
            _ => {} // Handle other events in future phases
        }
        Ok(())
    }

    /// Handles key events and updates the application state
    async fn handle_key_event(app_state: &mut AppState, key: KeyEvent) -> AppResult<()> {
        // Handle connection dialog events first
        if app_state.ui_state.connection_dialog.is_open {
            return Self::handle_dialog_key_event(app_state, key).await;
        }
        
        // Handle progress bar dismissal
        if app_state.has_active_progress() && matches!(key.code, KeyCode::Esc) {
            app_state.dismiss_progress_bars();
            return Ok(());
        }
        
        match (key.modifiers, key.code) {
            // Quit application or exit search mode
            (_, KeyCode::Esc | KeyCode::Char('q'))  => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser)
                    && app_state.ui_state.database_browser.search_mode {
                    // Exit search mode
                    app_state.exit_search_mode();
                } else {
                    // Quit application
                    app_state.quit();
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                app_state.quit();
            }
            
            // Panel navigation
            (_, KeyCode::Tab) => {
                app_state.next_panel();
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                app_state.previous_panel();
            }
            
            // Command execution and search application
            (_, KeyCode::Enter) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::CommandInput) {
                    Self::execute_command(app_state).await?;
                } else if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser)
                    && app_state.ui_state.database_browser.search_mode {
                    // Apply search filter
                    app_state.apply_search_filter().await?;
                }
            }
            
            // Character input for command panel and special keys
            (_, KeyCode::Char(ch)) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::CommandInput) {
                    app_state.ui_state.command_input.input.push(ch);
                } else if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    if app_state.ui_state.database_browser.search_mode {
                        // In search mode, add character to search pattern
                        app_state.add_search_char(ch);
                    } else {
                        // Handle special keys for database browser
                        match ch {
                            'c' => {
                                // Handle 'c' for connection dialog if not in command input mode
                                app_state.open_connection_dialog();
                            }
                            'r' => {
                                // Refresh keys in database browser
                                Self::refresh_keys(app_state).await?;
                            }
                            '/' => {
                                // Enter search mode
                                app_state.enter_search_mode();
                            }
                            '1' => {
                                app_state.set_view(crate::app::ViewMode::ConnectionList);
                            }
                            '2' => {
                                app_state.set_view(crate::app::ViewMode::DatabaseBrowser);
                            }
                            '3' => {
                                app_state.set_view(crate::app::ViewMode::KeyViewer);
                            }
                            '4' => {
                                app_state.set_view(crate::app::ViewMode::CommandInterface);
                            }
                            '?' => {
                                app_state.set_view(crate::app::ViewMode::Help);
                            }
                            _ => {}
                        }
                    }
                } else if ch == 'c' {
                    // Handle 'c' for connection dialog in other panels
                    app_state.open_connection_dialog();
                } else {
                    // Handle view switching in other panels
                    match ch {
                        '1' => {
                            app_state.set_view(crate::app::ViewMode::ConnectionList);
                        }
                        '2' => {
                            app_state.set_view(crate::app::ViewMode::DatabaseBrowser);
                        }
                        '3' => {
                            app_state.set_view(crate::app::ViewMode::KeyViewer);
                        }
                        '4' => {
                            app_state.set_view(crate::app::ViewMode::CommandInterface);
                        }
                        '?' => {
                            app_state.set_view(crate::app::ViewMode::Help);
                        }
                        _ => {}
                    }
                }
            }
            
            // Arrow key navigation - optimized for responsiveness
            (_, KeyCode::Down) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    app_state.select_next_key();
                    // Only load more keys if we're very close to the end to avoid blocking
                    let browser = &app_state.ui_state.database_browser;
                    if !browser.scan_complete && 
                       browser.selected_key_index >= browser.keys.len().saturating_sub(1) &&
                       !browser.loading {
                        // Schedule async loading without blocking current navigation
                        let _ = app_state.schedule_key_loading();
                    }
                }
            }
            (_, KeyCode::Up) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    app_state.select_previous_key();
                }
            }
            (_, KeyCode::Right) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    // Load selected key value and switch focus to Key Viewer
                    if let Some(key_info) = app_state.get_selected_key_info() {
                        let key_name = key_info.name.clone();
                        match Self::load_key_value(app_state, &key_name).await {
                            Ok(()) => {
                                // Switch focus to Key Viewer panel
                                app_state.ui_state.focused_panel = FocusedPanel::KeyViewer;
                                app_state.set_status(format!("Loaded key: {}", key_name));
                            }
                            Err(err) => {
                                app_state.set_status(format!("Failed to load key {}: {}", key_name, err));
                            }
                        }
                    } else {
                        app_state.set_status("No key selected".to_string());
                    }
                }
            }
            (_, KeyCode::Left) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::KeyViewer) {
                    // Switch focus back to Database Browser
                    app_state.ui_state.focused_panel = FocusedPanel::DatabaseBrowser;
                }
            }
            
            // Page navigation in database browser - optimized
            (_, KeyCode::PageUp) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    // Move up by 5 keys in one operation for better performance
                    app_state.select_key_by_offset(-5);
                }
            }
            (_, KeyCode::PageDown) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    // Move down by 5 keys in one operation for better performance
                    app_state.select_key_by_offset(5);
                }
            }
            
            // Home and End navigation
            (_, KeyCode::Home) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    app_state.ui_state.database_browser.selected_key_index = 0;
                    app_state.ui_state.database_browser.scroll_offset = 0;
                    // Update selected key
                    if let Some(key_info) = app_state.ui_state.database_browser.keys.first() {
                        app_state.selected_key = Some(key_info.name.clone());
                    }
                }
            }
            (_, KeyCode::End) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    let keys_len = app_state.ui_state.database_browser.keys.len();
                    if keys_len > 0 {
                        app_state.ui_state.database_browser.selected_key_index = keys_len - 1;
                        // Adjust scroll offset to show the last key
                        let visible_count = 10;
                        if keys_len > visible_count {
                            app_state.ui_state.database_browser.scroll_offset = keys_len - visible_count;
                        }
                        // Update selected key
                        if let Some(key_info) = app_state.ui_state.database_browser.keys.last() {
                            app_state.selected_key = Some(key_info.name.clone());
                        }
                    }
                }
            }
            
            // Delete key functionality
            (_, KeyCode::Delete) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    Self::delete_selected_key(app_state).await?;
                }
            }
            
            // Backspace for command panel and search mode
            (_, KeyCode::Backspace) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::CommandInput) {
                    app_state.ui_state.command_input.input.pop();
                } else if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser)
                    && app_state.ui_state.database_browser.search_mode {
                    app_state.backspace_search();
                }
            }
            
            // Clear status message on any other key
            _ => {
                app_state.clear_status();
            }
        }
        Ok(())
    }

    /// Handle key events when connection dialog is open
    async fn handle_dialog_key_event(app_state: &mut AppState, key: KeyEvent) -> AppResult<()> {
        match (key.modifiers, key.code) {
            // Close dialog
            (_, KeyCode::Esc) => {
                app_state.close_connection_dialog();
            }
            
            // Navigate fields
            (_, KeyCode::Tab) => {
                app_state.next_dialog_field();
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                app_state.previous_dialog_field();
            }
            
            // Save connection
            (_, KeyCode::Enter) => {
                if matches!(app_state.ui_state.connection_dialog.focused_field, ConnectionDialogField::Buttons) {
                    // On buttons, Enter means Save
                    match app_state.create_connection_from_dialog().await {
                        Ok(()) => {
                            // Connection created successfully
                        }
                        Err(err) => {
                            app_state.set_status(format!("Connection failed: {}", err));
                        }
                    }
                } else {
                    // On form fields, Enter means save
                    match app_state.create_connection_from_dialog().await {
                        Ok(()) => {
                            // Connection created successfully
                        }
                        Err(err) => {
                            app_state.set_status(format!("Connection failed: {}", err));
                        }
                    }
                }
            }
            
            // Backspace
            (_, KeyCode::Backspace) => {
                app_state.backspace_dialog_field();
            }
            
            // Character input
            (_, KeyCode::Char(ch)) => {
                app_state.update_dialog_field(ch);
            }
            
            _ => {}
        }
        Ok(())
    }
    
    /// Execute Redis command from command input
    async fn execute_command(app_state: &mut AppState) -> AppResult<()> {
        let command_text = app_state.ui_state.command_input.input.trim().to_string();
        
        if command_text.is_empty() {
            return Ok(());
        }
        
        // Check if we have an active connection
        let has_connection = app_state.active_connection.is_some();
        
        if !has_connection {
            app_state.set_status("No active connection. Please connect to Redis first.".to_string());
            return Ok(());
        }
        
        // Parse command (simple split by whitespace for now)
        let parts: Vec<&str> = command_text.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }
        
        let cmd = parts[0].to_uppercase();
        let args: Vec<&str> = parts[1..].to_vec();
        
        // Execute the command
        let result = {
            if let Some(connection) = app_state.get_active_connection_mut() {
                connection.execute_command(&cmd, &args).await
            } else {
                return Ok(());
            }
        };
        
        // Handle the result
        match result {
            Ok(output) => {
                // Add to command history
                let command_result = crate::app::CommandResult {
                    command: command_text.clone(),
                    result: Ok(output.clone()),
                    timestamp: std::time::SystemTime::now(),
                };
                app_state.ui_state.command_input.results.push(command_result);
                
                // Show result in status
                let preview = if output.len() > 50 {
                    format!("{}...", &output[..50])
                } else {
                    output
                };
                app_state.set_status(format!("Result: {}", preview));
            }
            Err(err) => {
                // Add error to command history
                let command_result = crate::app::CommandResult {
                    command: command_text.clone(),
                    result: Err(err.to_string()),
                    timestamp: std::time::SystemTime::now(),
                };
                app_state.ui_state.command_input.results.push(command_result);
                
                app_state.set_status(format!("Error: {}", err));
            }
        }
        
        // Add to history and clear input
        app_state.ui_state.command_input.history.push(command_text);
        app_state.ui_state.command_input.input.clear();
        
        Ok(())
    }
    
    /// Refresh keys in the current database
    async fn refresh_keys(app_state: &mut AppState) -> AppResult<()> {
        // Reset scanning state
        app_state.ui_state.database_browser.keys.clear();
        app_state.ui_state.database_browser.scan_cursor = 0;
        app_state.ui_state.database_browser.scan_complete = false;
        app_state.ui_state.database_browser.selected_key_index = 0;
        
        // Load keys
        app_state.load_keys().await?;
        Ok(())
    }
    
    /// Initialize database browser for active connection
    async fn initialize_database_browser(app_state: &mut AppState) -> AppResult<()> {
        if app_state.active_connection.is_some() {
            // Load available databases
            app_state.load_databases().await?;
            
            // Select database 0 by default and load keys silently
            if let Some(connection) = app_state.get_active_connection_mut() {
                match connection.select_database(0).await {
                    Ok(()) => {
                        app_state.ui_state.database_browser.selected_database = 0;
                        app_state.selected_database = Some(0);
                        
                        // Load keys silently without progress dialog
                        app_state.load_keys_silent().await?;
                        
                        app_state.set_status("Connected to database 0".to_string());
                    }
                    Err(err) => {
                        app_state.set_status(format!("Failed to select database 0: {}", err));
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Delete the currently selected key
    async fn delete_selected_key(app_state: &mut AppState) -> AppResult<()> {
        if let Some(key_info) = app_state.get_selected_key_info() {
            let key_name = key_info.name.clone();
            
            if let Some(connection) = app_state.get_active_connection_mut() {
                match connection.delete_key(&key_name).await {
                    Ok(_) => {
                        // Remove key from local list
                        let browser = &mut app_state.ui_state.database_browser;
                        browser.keys.remove(browser.selected_key_index);
                        
                        // Adjust selection index if needed
                        if browser.selected_key_index >= browser.keys.len() && browser.selected_key_index > 0 {
                            browser.selected_key_index -= 1;
                        }
                        
                        // Update selected key
                        if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                            app_state.selected_key = Some(key_info.name.clone());
                        } else {
                            app_state.selected_key = None;
                        }
                        
                        app_state.set_status(format!("Deleted key: {}", key_name));
                    }
                    Err(err) => {
                        app_state.set_status(format!("Failed to delete key {}: {}", key_name, err));
                    }
                }
            }
        } else {
            app_state.set_status("No key selected".to_string());
        }
        Ok(())
    }
    
    /// Load key value and metadata into Key Viewer panel
    async fn load_key_value(app_state: &mut AppState, key_name: &str) -> AppResult<()> {
        if let Some(connection) = app_state.get_active_connection_mut() {
            // Get key type first
            let key_type = connection.get_key_type(key_name).await?
                .trim().to_string();
            
            if key_type == "none" {
                return Err(crate::error::AppError::Generic("Key does not exist".to_string()));
            }
            
            // Get key TTL
            let ttl = match connection.get_key_ttl(key_name).await {
                Ok(ttl_val) => {
                    match ttl_val {
                        -2 => None, // Key doesn't exist
                        -1 => Some(-1), // No expiry
                        ttl if ttl > 0 => Some(ttl), // Has expiry
                        _ => None,
                    }
                }
                Err(_) => None,
            };
            
            // Load value based on key type
            let redis_value = match key_type.as_str() {
                "string" => {
                    let value = connection.get_string(key_name).await?;
                    crate::redis::value_types::RedisValue::String(value)
                }
                "hash" => {
                    let fields = connection.get_hash_all(key_name).await?;
                    crate::redis::value_types::RedisValue::Hash(fields)
                }
                "list" => {
                    let elements = connection.get_list_range(key_name, 0, -1).await?;
                    crate::redis::value_types::RedisValue::List(elements)
                }
                "set" => {
                    let (_, members) = connection.get_set_members(key_name, 0, 1000).await?;
                    crate::redis::value_types::RedisValue::Set(members)
                }
                "zset" => {
                    let members_with_scores = connection.get_zset_range_with_scores(key_name, 0, -1).await?;
                    crate::redis::value_types::RedisValue::ZSet(members_with_scores)
                }
                "stream" => {
                    let entries = connection.get_stream_range(key_name, "-", "+", Some(100)).await?;
                    // Convert connection::StreamEntry to value_types::StreamEntry
                    let converted_entries: Vec<crate::redis::value_types::StreamEntry> = entries
                        .into_iter()
                        .map(|e| crate::redis::value_types::StreamEntry {
                            id: e.id,
                            fields: e.fields,
                        })
                        .collect();
                    crate::redis::value_types::RedisValue::Stream(converted_entries)
                }
                _ => {
                    return Err(crate::error::AppError::Generic(format!("Unsupported key type: {}", key_type)));
                }
            };
            
            // Update Key Viewer state
            let key_viewer = &mut app_state.ui_state.key_viewer;
            key_viewer.current_key = Some(key_name.to_string());
            key_viewer.value = Some(redis_value);
            key_viewer.metadata = Some(crate::app::KeyMetadata {
                key_type: key_type.clone(),
                ttl,
                size: 0, // TODO: Calculate size based on value
                encoding: None,
            });
            
            // Reset viewer state
            key_viewer.current_page = 0;
            key_viewer.scroll_position = 0;
            key_viewer.edit_mode = false;
            key_viewer.edit_buffer.clear();
            key_viewer.has_unsaved_changes = false;
            
            Ok(())
        } else {
            Err(crate::error::AppError::Generic("No active connection".to_string()))
        }
    }
}