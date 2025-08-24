use ratatui::{DefaultTerminal, Frame};
use tokio::time::{timeout, Duration};
use crossterm::event;

use crate::app::state_core::AppState;
use crate::error::{AppError, AppResult};
use crate::events::{AppEvent, EventHandler};
use crate::ui::{AppRenderer, DialogRenderer};

/// The main application controller which manages the TUI and application flow
pub struct AppController {
    /// Application state
    pub state: AppState,
}

impl AppController {
    /// Construct a new instance of the application controller
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Run the application's main loop
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> AppResult<()> {
        // Take the event receiver from state
        let mut event_rx = self.state.event_rx.take()
            .ok_or_else(|| AppError::Generic("Event receiver not available".to_string()))?;

        self.state.running = true;
        self.state.set_status("RUDIS - Redis TUI Client Started".to_string());

        while self.state.running {
            // Check if a full redraw is needed (e.g., after external editor)
            if self.state.take_full_redraw_flag() {
                // Force the terminal backend to clear and redraw everything
                terminal.clear()?;
            }
            
            // Draw the UI
            terminal.draw(|frame| self.render(frame))?;
            
            // Handle events with shorter timeout for better responsiveness
            let event_timeout = Duration::from_millis(16); // ~60 FPS for smooth navigation
            
            // Check for crossterm events (user input) - prioritize immediate response
            if event::poll(Duration::from_millis(0))
                .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))? {
                EventHandler::handle_crossterm_events(&mut self.state).await?;
                continue; // Process input immediately without waiting
            }
            
            // Check for application events (async operations)
            match timeout(event_timeout, event_rx.recv()).await {
                Ok(Some(app_event)) => EventHandler::handle_app_event(&mut self.state, app_event).await?,
                Ok(None) => break, // Channel closed
                Err(_) => {}, // Timeout - continue loop
            }
        }
        
        Ok(())
    }

    /// Renders the user interface by delegating to the appropriate renderers
    fn render(&mut self, frame: &mut Frame) {
        // Render main application UI
        AppRenderer::render(&self.state, frame);
        
        // Render connection dialog if open
        if self.state.ui_state.connection_dialog.is_open {
            DialogRenderer::render_connection_dialog(&self.state, frame, frame.area());
        }
    }
}