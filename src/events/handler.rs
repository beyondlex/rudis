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
                } else if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    if app_state.ui_state.database_browser.search_mode {
                        // Apply search filter
                        app_state.apply_search_filter().await?;
                    } else if app_state.ui_state.database_browser.use_tree_view {
                        // In tree view, Enter expands/collapses nodes
                        app_state.toggle_tree_node();
                    }
                }
            }
            
            // Character input for command panel and special keys
            (_, KeyCode::Char(ch)) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::CommandInput) {
                    app_state.ui_state.command_input.input.push(ch);
                } else if matches!(app_state.ui_state.focused_panel, FocusedPanel::KeyViewer) {
                    // Handle Key Viewer panel specific keys
                    match ch {
                        'e' => {
                            // Open external editor for string values
                            if let Some(crate::redis::value_types::RedisValue::String(_)) = &app_state.ui_state.key_viewer.value {
                                Self::open_external_editor(app_state).await?;
                            }
                        }
                        'y' => {
                            // Handle vim-like yy command for copying to clipboard
                            if app_state.ui_state.key_viewer.handle_vim_sequence('y') {
                                // yy command executed - copied to clipboard
                                if let Some(key_name) = &app_state.ui_state.key_viewer.current_key {
                                    app_state.set_status(format!("Copied value of '{}' to clipboard", key_name));
                                } else {
                                    app_state.set_status("Copied value to clipboard".to_string());
                                }
                            }
                            // If not yy, the sequence tracking is still active for potential second 'y'
                        }
                        'c' => {
                            // Reset vim sequence when other keys are pressed
                            app_state.ui_state.key_viewer.reset_vim_sequence();
                            // Handle 'c' for connection dialog
                            app_state.open_connection_dialog();
                        }
                        '1' => {
                            app_state.ui_state.key_viewer.reset_vim_sequence();
                            app_state.set_view(crate::app::ViewMode::ConnectionList);
                        }
                        '2' => {
                            app_state.ui_state.key_viewer.reset_vim_sequence();
                            app_state.set_view(crate::app::ViewMode::DatabaseBrowser);
                        }
                        '3' => {
                            app_state.ui_state.key_viewer.reset_vim_sequence();
                            app_state.set_view(crate::app::ViewMode::KeyViewer);
                        }
                        '4' => {
                            app_state.ui_state.key_viewer.reset_vim_sequence();
                            app_state.set_view(crate::app::ViewMode::CommandInterface);
                        }
                        '?' => {
                            app_state.ui_state.key_viewer.reset_vim_sequence();
                            app_state.set_view(crate::app::ViewMode::Help);
                        }
                        _ => {
                            // Reset vim sequence for any other character
                            app_state.ui_state.key_viewer.reset_vim_sequence();
                        }
                    }
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
                            't' => {
                                // Toggle tree view mode
                                app_state.toggle_tree_view();
                                app_state.set_status(format!("Tree view: {}", 
                                    if app_state.ui_state.database_browser.use_tree_view { "ON" } else { "OFF" }));
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
                    // Automatically load the selected key value
                    if let Some(key_info) = app_state.get_selected_key_info() {
                        let key_name = key_info.name.clone();
                        let _ = Self::load_key_value(app_state, &key_name).await;
                    }
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
                    // Automatically load the selected key value
                    if let Some(key_info) = app_state.get_selected_key_info() {
                        let key_name = key_info.name.clone();
                        let _ = Self::load_key_value(app_state, &key_name).await;
                    }
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
                                // app_state.ui_state.focused_panel = FocusedPanel::KeyViewer;
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
                    // Automatically load the selected key value
                    if let Some(key_info) = app_state.get_selected_key_info() {
                        let key_name = key_info.name.clone();
                        let _ = Self::load_key_value(app_state, &key_name).await;
                    }
                }
            }
            (_, KeyCode::PageDown) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    // Move down by 5 keys in one operation for better performance
                    app_state.select_key_by_offset(5);
                    // Automatically load the selected key value
                    if let Some(key_info) = app_state.get_selected_key_info() {
                        let key_name = key_info.name.clone();
                        let _ = Self::load_key_value(app_state, &key_name).await;
                    }
                }
            }
            
            // Home and End navigation
            (_, KeyCode::Home) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    app_state.ui_state.database_browser.selected_key_index = 0;
                    app_state.ui_state.database_browser.scroll_offset = 0;
                    // Update selected key and get name for loading
                    let key_name_to_load = if let Some(key_info) = app_state.ui_state.database_browser.keys.first() {
                        app_state.selected_key = Some(key_info.name.clone());
                        Some(key_info.name.clone())
                    } else {
                        None
                    };
                    // Update scrollbar state
                    app_state.update_scrollbar_state(None);
                    // Load the key value if we have a selected key
                    if let Some(key_name) = key_name_to_load {
                        let _ = Self::load_key_value(app_state, &key_name).await;
                    }
                }
            }
            (_, KeyCode::End) => {
                if matches!(app_state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser) {
                    let keys_len = app_state.ui_state.database_browser.keys.len();
                    if keys_len > 0 {
                        app_state.ui_state.database_browser.selected_key_index = keys_len - 1;
                        // Adjust scroll offset to show the last key - use proper calculation
                        let visible_count = crate::app::AppState::get_visible_key_count();
                        if keys_len > visible_count {
                            app_state.ui_state.database_browser.scroll_offset = keys_len - visible_count;
                        } else {
                            app_state.ui_state.database_browser.scroll_offset = 0;
                        }
                        // Update selected key and get name for loading
                        let key_name_to_load = if let Some(key_info) = app_state.ui_state.database_browser.keys.last() {
                            app_state.selected_key = Some(key_info.name.clone());
                            Some(key_info.name.clone())
                        } else {
                            None
                        };
                        // Update scrollbar state
                        app_state.update_scrollbar_state(None);
                        // Load the key value if we have a selected key
                        if let Some(key_name) = key_name_to_load {
                            let _ = Self::load_key_value(app_state, &key_name).await;
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
    
    /// Open external editor for editing string values
    async fn open_external_editor(app_state: &mut AppState) -> AppResult<()> {
        use std::env;
        use std::fs;
        use std::process::Command;
        use tokio::process::Command as TokioCommand;
        
        // Get the current string value and key name first
        let (current_value, key_name) = {
            let current_value = if let Some(crate::redis::value_types::RedisValue::String(s)) = &app_state.ui_state.key_viewer.value {
                s.clone()
            } else {
                return Err(crate::error::AppError::Generic("No string value to edit".to_string()));
            };
            
            let key_name = app_state.ui_state.key_viewer.current_key
                .as_ref()
                .ok_or_else(|| crate::error::AppError::Generic("No key selected".to_string()))?
                .clone();
                
            (current_value, key_name)
        };
        
        // Get editor from environment
        let editor = env::var("EDITOR")
            .or_else(|_| env::var("VISUAL"))
            .unwrap_or_else(|_| "vim".to_string());
        
        // Create temporary file
        let temp_file = format!("/tmp/rudis_edit_{}.txt", key_name.replace([':', '/', '\\'], "_"));
        
        // Write current value to temp file
        fs::write(&temp_file, &current_value)
            .map_err(|e| crate::error::AppError::Generic(format!("Failed to create temp file: {}", e)))?;
        
        // Show status message
        app_state.set_status(format!("Opening {} for editing...", editor));
        
        // Save terminal state and exit raw mode for the editor
        let _guard = {
            use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
            use crossterm::execute;
            use std::io::stdout;
            
            // Save current screen buffer and disable raw mode
            execute!(stdout(), LeaveAlternateScreen)
                .map_err(|e| crate::error::AppError::Generic(format!("Failed to leave alternate screen: {}", e)))?;
            disable_raw_mode()
                .map_err(|e| crate::error::AppError::Generic(format!("Failed to disable raw mode: {}", e)))?;
            
            // Custom guard to restore terminal state when dropped
            struct TerminalStateGuard;
            impl Drop for TerminalStateGuard {
                fn drop(&mut self) {
                    use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};
                    use crossterm::execute;
                    use std::io::stdout;
                    
                    // Restore raw mode and alternate screen
                    let _ = enable_raw_mode();
                    let _ = execute!(stdout(), EnterAlternateScreen);
                }
            }
            TerminalStateGuard
        };
        
        // Launch editor
        let mut child = TokioCommand::new(&editor)
            .arg(&temp_file)
            .spawn()
            .map_err(|e| crate::error::AppError::Generic(format!("Failed to launch editor: {}", e)))?;
        
        // Wait for editor to exit
        let exit_status = child.wait().await
            .map_err(|e| crate::error::AppError::Generic(format!("Editor process error: {}", e)))?;
        
        // Guard will restore terminal state when it goes out of scope
        drop(_guard);
        
        // Force complete terminal redraw after returning from editor
        {
            use crossterm::{terminal, cursor, execute, queue};
            use std::io::{stdout, Write};
            
            let mut stdout = stdout();
            
            // Comprehensive terminal reset
            let _ = queue!(
                stdout,
                // Clear entire screen and scrollback
                terminal::Clear(terminal::ClearType::All),
                terminal::Clear(terminal::ClearType::Purge),
                // Reset cursor to top-left
                cursor::MoveTo(0, 0),
                // Hide cursor temporarily
                cursor::Hide,
                // Show cursor again
                cursor::Show
            );
            
            // Force immediate flush to ensure all commands are executed
            let _ = stdout.flush();
        }
        
        // Request full redraw to ensure TUI framework redraws everything
        app_state.request_full_redraw();
        
        if exit_status.success() {
            // Read the edited content
            match fs::read_to_string(&temp_file) {
                Ok(new_content) => {
                    // Check if content changed
                    if new_content != current_value {
                        // Update the value in Redis
                        if let Some(connection) = app_state.get_active_connection_mut() {
                            match connection.set_string(&key_name, &new_content).await {
                                Ok(_result) => {
                                    // Update the local value
                                    app_state.ui_state.key_viewer.value = Some(
                                        crate::redis::value_types::RedisValue::String(new_content)
                                    );
                                    app_state.set_status(format!("Updated key: {}", key_name));
                                }
                                Err(err) => {
                                    app_state.set_status(format!("Failed to update key: {}", err));
                                }
                            }
                        } else {
                            app_state.set_status("No active connection".to_string());
                        }
                    } else {
                        app_state.set_status("No changes made".to_string());
                    }
                }
                Err(err) => {
                    app_state.set_status(format!("Failed to read edited file: {}", err));
                }
            }
        } else {
            app_state.set_status("Editor exited with error".to_string());
        }
        
        // Clean up temp file
        let _ = fs::remove_file(&temp_file);
        
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
            key_viewer.metadata = Some(crate::app::state_core::KeyMetadata {
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