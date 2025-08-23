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
        
        // Create main layout: header + body + footer
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Body
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
        let connections_title = "Connections (0)";
        let connections_content = if self.state.connections.is_empty() {
            "No connections\n\nPress 'c' to add\na new connection"
        } else {
            "• localhost:6379\n  Status: Disconnected"
        };
        
        frame.render_widget(
            Paragraph::new(connections_content)
                .block(Block::bordered().title(connections_title))
                .style(Style::default()),
            body_layout[0],
        );
        
        // Render database browser panel
        let db_title = "Database Browser";
        let db_content = "Select a connection\nto browse databases";
        
        frame.render_widget(
            Paragraph::new(db_content)
                .block(Block::bordered().title(db_title))
                .style(Style::default()),
            body_layout[1],
        );
        
        // Render key viewer panel
        let viewer_title = "Key Viewer";
        let viewer_content = "Select a key\nto view its content";
        
        frame.render_widget(
            Paragraph::new(viewer_content)
                .block(Block::bordered().title(viewer_title))
                .style(Style::default()),
            body_layout[2],
        );
        
        // Render footer with status and shortcuts
        let status_text = self.state.status_message
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Ready");
        
        let footer_text = format!(
            "{} | q:Quit c:Connect Tab:Navigate ?:Help",
            status_text
        );
        
        frame.render_widget(
            Paragraph::new(footer_text)
                .block(Block::bordered())
                .style(Style::default().bg(Color::DarkGray)),
            main_layout[2],
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
            
            // Connection management
            (_, KeyCode::Char('c')) => {
                self.state.set_status("Connection setup not yet implemented".to_string());
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
}
