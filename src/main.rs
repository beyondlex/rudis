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
            
            // Handle events with timeout to allow for periodic updates
            let event_timeout = Duration::from_millis(100);
            
            // Check for crossterm events (user input)
            if crossterm::event::poll(Duration::from_millis(0))
                .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))? {
                self.handle_crossterm_events().await?;
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
            Style::default().bg(Color::DarkGray).fg(Color::White)
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
        let db_title = "Database Browser";
        let db_content = if self.state.active_connection.is_some() {
            "Database: 0\n\nPress 'Enter' to\nbrowse keys"
        } else {
            "Select a connection\nto browse databases"
        };
        
        // Style based on focus
        let is_db_focused = matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::DatabaseBrowser);
        let db_style = if is_db_focused {
            Style::default().bg(Color::DarkGray).fg(Color::White)
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
            Style::default().bg(Color::DarkGray).fg(Color::White)
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
            Style::default().bg(Color::DarkGray).fg(Color::White)
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
                    connection.status = status;
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
            // Quit application
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.state.quit();
            }
            
            // Panel navigation
            (_, KeyCode::Tab) => {
                self.state.next_panel();
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.state.previous_panel();
            }
            
            // Command execution
            (_, KeyCode::Enter) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::CommandInput) {
                    self.execute_command().await?;
                }
            }
            
            // Character input for command panel (only when not in dialog)
            (_, KeyCode::Char(ch)) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::CommandInput) {
                    self.state.ui_state.command_input.input.push(ch);
                } else if ch == 'c' {
                    // Handle 'c' for connection dialog if not in command input mode
                    self.state.open_connection_dialog();
                }
            }
            
            // Backspace for command panel
            (_, KeyCode::Backspace) => {
                if matches!(self.state.ui_state.focused_panel, crate::app::FocusedPanel::CommandInput) {
                    self.state.ui_state.command_input.input.pop();
                }
            }
            
            // View switching
            (_, KeyCode::Char('1')) => {
                self.state.set_view(ViewMode::ConnectionList);
            }
            (_, KeyCode::Char('2')) => {
                self.state.set_view(ViewMode::DatabaseBrowser);
            }
            (_, KeyCode::Char('3')) => {
                self.state.set_view(ViewMode::KeyViewer);
            }
            (_, KeyCode::Char('4')) => {
                self.state.set_view(ViewMode::CommandInterface);
            }
            
            // Help
            (_, KeyCode::Char('?')) => {
                self.state.set_view(ViewMode::Help);
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
}
