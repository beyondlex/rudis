use color_eyre::{Result, eyre};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};
use tokio::time::{timeout, Duration};
use std::path::PathBuf;

// Application modules
mod app;
mod error;
mod redis;
mod events;
mod ui;
mod utils;

use app::{AppConfig, AppState, ViewMode};
use error::{AppError, AppResult};
use events::AppEvent;

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
    
    // Run application
    let result = App::new(config).run(terminal).await;
    
    // Restore terminal
    ratatui::restore();
    
    // Convert AppResult to color_eyre::Result
    match result {
        Ok(()) => Ok(()),
        Err(app_err) => Err(color_eyre::eyre::eyre!(app_err.to_string())),
    }
}

/// The main application which holds the state and logic of the application.
pub struct App {
    /// Application state
    state: AppState,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new(config: AppConfig) -> Self {
        Self {
            state: AppState::new(config),
        }
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> AppResult<()> {
        // Take the event receiver from state
        let mut event_rx = self.state.event_rx.take()
            .ok_or_else(|| AppError::Generic("Event receiver not available".to_string()))?;

        self.state.running = true;
        self.state.set_status("RUDIS - Redis TUI Client Started".to_string());

        while self.state.running {
            // Draw the UI
            terminal.draw(|frame| self.render(frame))?;
            
            // Handle events with shorter timeout for better responsiveness
            let event_timeout = Duration::from_millis(16); // ~60 FPS for smooth navigation
            
            // Check for crossterm events (user input) - prioritize immediate response
            if crossterm::event::poll(Duration::from_millis(0))
                .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))? {
                self.handle_crossterm_events().await?;
                continue; // Process input immediately without waiting
            }
            
            // Check for application events (async operations)
            match timeout(event_timeout, event_rx.recv()).await {
                Ok(Some(app_event)) => self.handle_app_event(app_event).await?,
                Ok(None) => break, // Channel closed
                Err(_) => {}, // Timeout - continue loop
            }
        }
        
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This creates a basic 3-panel layout: connections, browser, and viewer
    fn render(&mut self, frame: &mut Frame) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Style};
        
        let area = frame.area();
        
        // Create main layout: header + body + command + footer
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Body (main panels)
                Constraint::Length(4),  // Command input
                Constraint::Length(3),  // Footer
            ])
            .split(area);
        
        // Render header
        let title = Line::from("RUDIS - Redis TUI Client v0.1.0")
            .bold()
            .blue()
            .centered();
        frame.render_widget(
            Paragraph::new("")
                .block(Block::bordered().title(title))
                .style(Style::default().bg(Color::DarkGray)),
            main_layout[0],
        );
        
        // Create body layout: 3 horizontal panels
        let body_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Connections
                Constraint::Percentage(35), // Database/Keys
                Constraint::Percentage(40), // Key Viewer
            ])
            .split(main_layout[1]);
        
        // Render connections panel
        let connections_count = self.state.connections.len();
        let connections_title = format!("Connections ({})", connections_count);
        
        let connections_content = if self.state.connections.is_empty() {
            "No connections\n\nPress 'c' to add\na new connection".to_string()
        } else {
            // List existing connections
            let mut content = String::new();
            for (id, connection) in &self.state.connections {
                let status_icon = match connection.status {
                    crate::redis::ConnectionStatus::Connected => "●",
                    crate::redis::ConnectionStatus::Connecting => "◐",
                    crate::redis::ConnectionStatus::Disconnected => "○",
                    crate::redis::ConnectionStatus::Failed(_) => "✗",
                    crate::redis::ConnectionStatus::Lost => "⚠",
                };
                let is_active = self.state.active_connection.as_ref() == Some(id);
                let marker = if is_active { "> " } else { "  " };
                content.push_str(&format!("{}{} {}\n", marker, status_icon, connection.config.name));
            }
            content.push_str("\nPress 'c' to add connection");
            content
        };
        
        // Style based on focus
        let is_connections_focused = matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::ConnectionList);
        let connections_style = if is_connections_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };
        let connections_border_style = if is_connections_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        frame.render_widget(
            Paragraph::new(connections_content)
                .block(Block::bordered()
                    .title(connections_title)
                    .border_style(connections_border_style))
                .style(connections_style),
            body_layout[0],
        );
        
        // Render database browser panel
        let browser_state = &self.state.ui_state.database_browser;
        let db_title = if self.state.active_connection.is_some() {
            if browser_state.search_mode {
                format!("Database {} - Search: {}_", 
                    browser_state.selected_database,
                    browser_state.filter_pattern)
            } else if !browser_state.filter_pattern.is_empty() {
                format!("Database {} - Filtered: {} ({} keys)", 
                    browser_state.selected_database,
                    browser_state.filter_pattern,
                    browser_state.keys.len())
            } else {
                format!("Database {} ({} keys)", 
                    browser_state.selected_database,
                    browser_state.keys.len())
            }
        } else {
            "Database Browser".to_string()
        };
        
        let db_content = if self.state.active_connection.is_some() {
            if browser_state.search_mode {
                // In search mode, show search instructions
                format!("Search Mode\n\nPattern: {}\n\nType to search, Enter to apply\nEsc to cancel search", 
                    if browser_state.filter_pattern.is_empty() { 
                        "<type pattern>"
                    } else { 
                        &browser_state.filter_pattern 
                    })
            } else if browser_state.keys.is_empty() {
                if browser_state.loading {
                    "Loading keys...".to_string()
                } else {
                    "No keys found\n\nPress 'r' to refresh\nPress '/' to search".to_string()
                }
            } else {
                // Show list of keys
                let mut content = String::new();
                let visible_keys = browser_state.keys.iter()
                    .skip(browser_state.scroll_offset)
                    .take(10); // Show up to 10 keys at once
                
                for (i, key_info) in visible_keys.enumerate() {
                    let actual_index = browser_state.scroll_offset + i;
                    let is_selected = actual_index == browser_state.selected_key_index;
                    let marker = if is_selected { "> " } else { "  " };
                    
                    // Key type icon
                    let type_icon = match key_info.key_type.as_deref() {
                        Some("string") => "🔤", // 🔤
                        Some("hash") => "📋", // 📋
                        Some("list") => "📜", // 📜
                        Some("set") => "📊", // 📊
                        Some("zset") => "📊", // 📊
                        Some("stream") => "🌊", // 🌊
                        _ => "●", // ● (unknown)
                    };
                    
                    // Add TTL info if available
                    let ttl_info = match key_info.ttl {
                        Some(ttl) if ttl > 0 => format!(" ({}s)", ttl),
                        Some(-1) => " (no exp)".to_string(),
                        _ => String::new(),
                    };
                    
                    // Truncate long key names
                    let display_name = if key_info.name.len() > 20 {
                        format!("{}...", &key_info.name[..17])
                    } else {
                        key_info.name.clone()
                    };
                    
                    content.push_str(&format!("{}{} {}{}\n", marker, type_icon, display_name, ttl_info));
                }
                
                if !browser_state.scan_complete {
                    content.push_str("\n[More keys available - scroll down]");
                }
                
                content.push_str(&format!("\nr:Refresh /:Search del:Delete"));
                content
            }
        } else {
            "Select a connection\nto browse databases".to_string()
        };
        
        // Style based on focus
        let is_db_focused = matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser);
        let db_style = if is_db_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };
        let db_border_style = if is_db_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        frame.render_widget(
            Paragraph::new(db_content)
                .block(Block::bordered()
                    .title(db_title)
                    .border_style(db_border_style))
                .style(db_style),
            body_layout[1],
        );
        
        // Render key viewer panel
        let viewer_title = "Key Viewer";
        let viewer_content = "Select a key\nto view its content";
        
        // Style based on focus
        let is_viewer_focused = matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::KeyViewer);
        let viewer_style = if is_viewer_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };
        let viewer_border_style = if is_viewer_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        frame.render_widget(
            Paragraph::new(viewer_content)
                .block(Block::bordered()
                    .title(viewer_title)
                    .border_style(viewer_border_style))
                .style(viewer_style),
            body_layout[2],
        );
        
        // Render command input panel
        let command_title = "Command Input (Redis CLI)";
        let command_state = &self.state.ui_state.command_input;
        let command_content = if command_state.input.is_empty() {
            "Type Redis commands here... (e.g., INFO, PING, GET mykey)"
        } else {
            &command_state.input
        };
        
        // Style based on focus
        let is_command_focused = matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::CommandInput);
        let command_style = if is_command_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };
        let command_border_style = if is_command_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        frame.render_widget(
            Paragraph::new(command_content)
                .block(Block::bordered()
                    .title(command_title)
                    .border_style(command_border_style))
                .style(command_style),
            main_layout[2],
        );
        
        // Render footer with status and shortcuts
        let status_text = self.state.status_message
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Ready");
        
        // Show current focused panel
        let focused_panel_name = match self.state.ui_state.focused_panel {
            crate::app::FocusedPanel::ConnectionList => "Connections",
            crate::app::FocusedPanel::DatabaseBrowser => "Database",
            crate::app::FocusedPanel::KeyViewer => "Key Viewer",
            crate::app::FocusedPanel::CommandInput => "Command",
        };
        
        let footer_text = format!(
            "{} | Focus: {} | q:Quit c:Connect Tab:Navigate ?:Help",
            status_text, focused_panel_name
        );
        
        frame.render_widget(
            Paragraph::new(footer_text)
                .block(Block::bordered())
                .style(Style::default().bg(Color::DarkGray)),
            main_layout[3],
        );
        
        // Render connection dialog if open
        if self.state.ui_state.connection_dialog.is_open {
            self.render_connection_dialog(frame, area);
        }
    }

    /// Render the connection creation dialog
    fn render_connection_dialog(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::layout::{Constraint, Direction, Layout, Alignment};
        use ratatui::style::{Color, Style, Modifier};
        use ratatui::widgets::{Clear, Borders};
        
        // Calculate dialog size and position (centered)
        let dialog_width = 60;
        let dialog_height = 16;
        let x = (area.width.saturating_sub(dialog_width)) / 2;
        let y = (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = ratatui::layout::Rect {
            x,
            y,
            width: dialog_width,
            height: dialog_height,
        };
        
        // Clear the background
        frame.render_widget(Clear, dialog_area);
        
        // Create dialog layout
        let dialog_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(11), // Form fields
                Constraint::Length(2),  // Buttons
            ])
            .split(dialog_area);
        
        // Render dialog title
        let title = Line::from("New Redis Connection")
            .bold()
            .white()
            .centered();
        frame.render_widget(
            Paragraph::new("")
                .block(Block::bordered().title(title))
                .style(Style::default().bg(Color::Blue)),
            dialog_layout[0],
        );
        
        // Create form layout
        let form_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Name
                Constraint::Length(2), // Host
                Constraint::Length(2), // Port
                Constraint::Length(2), // Password
                Constraint::Length(2), // Database
                Constraint::Length(1), // Spacer
            ])
            .split(dialog_layout[1]);
        
        let form = &self.state.ui_state.connection_dialog.form;
        let focused_field = &self.state.ui_state.connection_dialog.focused_field;
        
        // Helper function to create field style
        let field_style = |is_focused: bool| {
            if is_focused {
                Style::default().bg(Color::Yellow).fg(Color::Black)
            } else {
                Style::default().bg(Color::Gray).fg(Color::White)
            }
        };
        
        // Render form fields
        frame.render_widget(
            Paragraph::new(format!("Name: {}", form.name))
                .style(field_style(matches!(focused_field, crate::app::ConnectionDialogField::Name))),
            form_layout[0],
        );
        
        frame.render_widget(
            Paragraph::new(format!("Host: {}", form.host))
                .style(field_style(matches!(focused_field, crate::app::ConnectionDialogField::Host))),
            form_layout[1],
        );
        
        frame.render_widget(
            Paragraph::new(format!("Port: {}", form.port))
                .style(field_style(matches!(focused_field, crate::app::ConnectionDialogField::Port))),
            form_layout[2],
        );
        
        frame.render_widget(
            Paragraph::new(format!("Password: {}", "*".repeat(form.password.len())))
                .style(field_style(matches!(focused_field, crate::app::ConnectionDialogField::Password))),
            form_layout[3],
        );
        
        frame.render_widget(
            Paragraph::new(format!("Database: {}", form.database))
                .style(field_style(matches!(focused_field, crate::app::ConnectionDialogField::Database))),
            form_layout[4],
        );
        
        // Render buttons
        let button_text = if matches!(focused_field, crate::app::ConnectionDialogField::Buttons) {
            "[Save] [Cancel] - Enter:Select Tab:Navigate Esc:Cancel"
        } else {
            "Tab:Navigate Enter:Save Esc:Cancel"
        };
        
        frame.render_widget(
            Paragraph::new(button_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan)),
            dialog_layout[2],
        );
    }

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(&mut self) -> AppResult<()> {
        match event::read().map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key).await?
            }
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles application events from async operations
    async fn handle_app_event(&mut self, event: AppEvent) -> AppResult<()> {
        match event {
            AppEvent::KeyPressed(key) => self.on_key_event(key).await?,
            AppEvent::ConnectionStatusChanged { connection_id, status } => {
                if let Some(connection) = self.state.connections.get_mut(&connection_id) {
                    connection.status = status.clone();
                    // If connection is now established, initialize database browser
                    if matches!(status, crate::redis::ConnectionStatus::Connected) {
                        self.initialize_database_browser().await?;
                    }
                }
            }
            AppEvent::RefreshData => {
                // Handle background key loading for responsive navigation
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser) {
                    let _ = self.state.load_more_keys().await;
                }
            }
            AppEvent::StatusMessage(msg) => {
                self.state.set_status(msg);
            }
            AppEvent::Error(err) => {
                self.state.set_status(format!("Error: {}", err));
            }
            AppEvent::Quit => {
                self.state.quit();
            }
            _ => {} // Handle other events in future phases
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    async fn on_key_event(&mut self, key: KeyEvent) -> AppResult<()> {
        // Handle connection dialog events first
        if self.state.ui_state.connection_dialog.is_open {
            return self.handle_dialog_key_event(key).await;
        }
        
        match (key.modifiers, key.code) {
            // Quit application or exit search mode
            (_, KeyCode::Esc | KeyCode::Char('q'))  => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser)
                    && self.state.ui_state.database_browser.search_mode {
                    // Exit search mode
                    self.state.exit_search_mode();
                } else {
                    // Quit application
                    self.state.quit();
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.state.quit();
            }
            
            // Panel navigation
            (_, KeyCode::Tab) => {
                self.state.next_panel();
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.state.previous_panel();
            }
            
            // Command execution and search application
            (_, KeyCode::Enter) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::CommandInput) {
                    self.execute_command().await?;
                } else if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser)
                    && self.state.ui_state.database_browser.search_mode {
                    // Apply search filter
                    self.state.apply_search_filter().await?;
                }
            }
            
            // Character input for command panel and special keys
            (_, KeyCode::Char(ch)) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::CommandInput) {
                    self.state.ui_state.command_input.input.push(ch);
                } else if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser) {
                    if self.state.ui_state.database_browser.search_mode {
                        // In search mode, add character to search pattern
                        self.state.add_search_char(ch);
                    } else {
                        // Handle special keys for database browser
                        match ch {
                            'c' => {
                                // Handle 'c' for connection dialog if not in command input mode
                                self.state.open_connection_dialog();
                            }
                            'r' => {
                                // Refresh keys in database browser
                                self.refresh_keys().await?;
                            }
                            '/' => {
                                // Enter search mode
                                self.state.enter_search_mode();
                            }
                            '1' => {
                                self.state.set_view(ViewMode::ConnectionList);
                            }
                            '2' => {
                                self.state.set_view(ViewMode::DatabaseBrowser);
                            }
                            '3' => {
                                self.state.set_view(ViewMode::KeyViewer);
                            }
                            '4' => {
                                self.state.set_view(ViewMode::CommandInterface);
                            }
                            '?' => {
                                self.state.set_view(ViewMode::Help);
                            }
                            _ => {}
                        }
                    }
                } else if ch == 'c' {
                    // Handle 'c' for connection dialog in other panels
                    self.state.open_connection_dialog();
                } else {
                    // Handle view switching in other panels
                    match ch {
                        '1' => {
                            self.state.set_view(ViewMode::ConnectionList);
                        }
                        '2' => {
                            self.state.set_view(ViewMode::DatabaseBrowser);
                        }
                        '3' => {
                            self.state.set_view(ViewMode::KeyViewer);
                        }
                        '4' => {
                            self.state.set_view(ViewMode::CommandInterface);
                        }
                        '?' => {
                            self.state.set_view(ViewMode::Help);
                        }
                        _ => {}
                    }
                }
            }
            
            // Arrow key navigation - optimized for responsiveness
            (_, KeyCode::Down) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser) {
                    self.state.select_next_key();
                    // Only load more keys if we're very close to the end to avoid blocking
                    let browser = &self.state.ui_state.database_browser;
                    if !browser.scan_complete && 
                       browser.selected_key_index >= browser.keys.len().saturating_sub(1) &&
                       !browser.loading {
                        // Schedule async loading without blocking current navigation
                        let _ = self.state.schedule_key_loading();
                    }
                }
            }
            (_, KeyCode::Up) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser) {
                    self.state.select_previous_key();
                }
            }
            
            // Page navigation in database browser - optimized
            (_, KeyCode::PageUp) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser) {
                    // Move up by 5 keys in one operation for better performance
                    self.state.select_key_by_offset(-5);
                }
            }
            (_, KeyCode::PageDown) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser) {
                    // Move down by 5 keys in one operation for better performance
                    self.state.select_key_by_offset(5);
                    // Schedule loading if needed without blocking
                    let browser = &self.state.ui_state.database_browser;
                    if !browser.scan_complete && 
                       browser.selected_key_index >= browser.keys.len().saturating_sub(3) &&
                       !browser.loading {
                        let _ = self.state.schedule_key_loading();
                    }
                }
            }
            
            // Home and End navigation
            (_, KeyCode::Home) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser) {
                    self.state.ui_state.database_browser.selected_key_index = 0;
                    self.state.ui_state.database_browser.scroll_offset = 0;
                    // Update selected key
                    if let Some(key_info) = self.state.ui_state.database_browser.keys.first() {
                        self.state.selected_key = Some(key_info.name.clone());
                    }
                }
            }
            (_, KeyCode::End) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser) {
                    let keys_len = self.state.ui_state.database_browser.keys.len();
                    if keys_len > 0 {
                        self.state.ui_state.database_browser.selected_key_index = keys_len - 1;
                        // Adjust scroll offset to show the last key
                        let visible_count = 10;
                        if keys_len > visible_count {
                            self.state.ui_state.database_browser.scroll_offset = keys_len - visible_count;
                        }
                        // Update selected key
                        if let Some(key_info) = self.state.ui_state.database_browser.keys.last() {
                            self.state.selected_key = Some(key_info.name.clone());
                        }
                    }
                }
            }
            
            // Delete key functionality
            (_, KeyCode::Delete) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser) {
                    self.delete_selected_key().await?;
                }
            }
            
            // Backspace for command panel and search mode
            (_, KeyCode::Backspace) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::CommandInput) {
                    self.state.ui_state.command_input.input.pop();
                } else if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser)
                    && self.state.ui_state.database_browser.search_mode {
                    self.state.backspace_search();
                }
            }
            
            // Clear status message on any other key
            _ => {
                self.state.clear_status();
            }
        }
        Ok(())
    }

    /// Handle key events when connection dialog is open
    async fn handle_dialog_key_event(&mut self, key: KeyEvent) -> AppResult<()> {
        match (key.modifiers, key.code) {
            // Close dialog
            (_, KeyCode::Esc) => {
                self.state.close_connection_dialog();
            }
            
            // Navigate fields
            (_, KeyCode::Tab) => {
                self.state.next_dialog_field();
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.state.previous_dialog_field();
            }
            
            // Save connection
            (_, KeyCode::Enter) => {
                if matches!(self.state.ui_state.connection_dialog.focused_field, crate::app::ConnectionDialogField::Buttons) {
                    // On buttons, Enter means Save
                    match self.state.create_connection_from_dialog().await {
                        Ok(()) => {
                            // Connection created successfully
                        }
                        Err(err) => {
                            self.state.set_status(format!("Connection failed: {}", err));
                        }
                    }
                } else {
                    // On form fields, Enter means save
                    match self.state.create_connection_from_dialog().await {
                        Ok(()) => {
                            // Connection created successfully
                        }
                        Err(err) => {
                            self.state.set_status(format!("Connection failed: {}", err));
                        }
                    }
                }
            }
            
            // Backspace
            (_, KeyCode::Backspace) => {
                self.state.backspace_dialog_field();
            }
            
            // Character input
            (_, KeyCode::Char(ch)) => {
                self.state.update_dialog_field(ch);
            }
            
            _ => {}
        }
        Ok(())
    }
    
    /// Execute Redis command from command input
    async fn execute_command(&mut self) -> AppResult<()> {
        let command_text = self.state.ui_state.command_input.input.trim().to_string();
        
        if command_text.is_empty() {
            return Ok(());
        }
        
        // Check if we have an active connection
        let has_connection = self.state.active_connection.is_some();
        
        if !has_connection {
            self.state.set_status("No active connection. Please connect to Redis first.".to_string());
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
            if let Some(connection) = self.state.get_active_connection_mut() {
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
                self.state.ui_state.command_input.results.push(command_result);
                
                // Show result in status
                let preview = if output.len() > 50 {
                    format!("{}...", &output[..50])
                } else {
                    output
                };
                self.state.set_status(format!("Result: {}", preview));
            }
            Err(err) => {
                // Add error to command history
                let command_result = crate::app::CommandResult {
                    command: command_text.clone(),
                    result: Err(err.to_string()),
                    timestamp: std::time::SystemTime::now(),
                };
                self.state.ui_state.command_input.results.push(command_result);
                
                self.state.set_status(format!("Error: {}", err));
            }
        }
        
        // Add to history and clear input
        self.state.ui_state.command_input.history.push(command_text);
        self.state.ui_state.command_input.input.clear();
        
        Ok(())
    }
    
    /// Refresh keys in the current database
    async fn refresh_keys(&mut self) -> AppResult<()> {
        // Reset scanning state
        self.state.ui_state.database_browser.keys.clear();
        self.state.ui_state.database_browser.scan_cursor = 0;
        self.state.ui_state.database_browser.scan_complete = false;
        self.state.ui_state.database_browser.selected_key_index = 0;
        
        // Load keys
        self.state.load_keys().await?;
        Ok(())
    }
    
    /// Initialize database browser for active connection
    async fn initialize_database_browser(&mut self) -> AppResult<()> {
        if self.state.active_connection.is_some() {
            // Load available databases
            self.state.load_databases().await?;
            
            // Select database 0 by default
            self.state.select_database(0).await?;
        }
        Ok(())
    }
    
    /// Delete the currently selected key
    async fn delete_selected_key(&mut self) -> AppResult<()> {
        if let Some(key_info) = self.state.get_selected_key_info() {
            let key_name = key_info.name.clone();
            
            if let Some(connection) = self.state.get_active_connection_mut() {
                match connection.delete_key(&key_name).await {
                    Ok(_) => {
                        // Remove key from local list
                        let browser = &mut self.state.ui_state.database_browser;
                        browser.keys.remove(browser.selected_key_index);
                        
                        // Adjust selection index if needed
                        if browser.selected_key_index >= browser.keys.len() && browser.selected_key_index > 0 {
                            browser.selected_key_index -= 1;
                        }
                        
                        // Update selected key
                        if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                            self.state.selected_key = Some(key_info.name.clone());
                        } else {
                            self.state.selected_key = None;
                        }
                        
                        self.state.set_status(format!("Deleted key: {}", key_name));
                    }
                    Err(err) => {
                        self.state.set_status(format!("Failed to delete key {}: {}", key_name, err));
                    }
                }
            }
        } else {
            self.state.set_status("No key selected".to_string());
        }
        Ok(())
    }
}
