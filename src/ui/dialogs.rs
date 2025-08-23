use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Paragraph, Clear},
    prelude::Stylize,
};

use crate::app::{AppState, ConnectionDialogField};

/// UI renderer for application dialogs
pub struct DialogRenderer;

impl DialogRenderer {
    /// Renders the connection creation dialog
    pub fn render_connection_dialog(state: &AppState, frame: &mut Frame, area: Rect) {
        // Calculate dialog size and position (centered)
        let dialog_width = 60;
        let dialog_height = 16;
        let x = (area.width.saturating_sub(dialog_width)) / 2;
        let y = (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect {
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
        
        Self::render_dialog_title(frame, dialog_layout[0]);
        Self::render_dialog_form(state, frame, dialog_layout[1]);
        Self::render_dialog_buttons(state, frame, dialog_layout[2]);
    }

    /// Renders the dialog title bar
    fn render_dialog_title(frame: &mut Frame, area: Rect) {
        let title = Line::from("New Redis Connection")
            .bold()
            .white()
            .centered();
        frame.render_widget(
            Paragraph::new("")
                .block(Block::bordered().title(title))
                .style(Style::default().bg(Color::Blue)),
            area,
        );
    }

    /// Renders the dialog form fields
    fn render_dialog_form(state: &AppState, frame: &mut Frame, area: Rect) {
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
            .split(area);
        
        let form = &state.ui_state.connection_dialog.form;
        let focused_field = &state.ui_state.connection_dialog.focused_field;
        
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
                .style(field_style(matches!(focused_field, ConnectionDialogField::Name))),
            form_layout[0],
        );
        
        frame.render_widget(
            Paragraph::new(format!("Host: {}", form.host))
                .style(field_style(matches!(focused_field, ConnectionDialogField::Host))),
            form_layout[1],
        );
        
        frame.render_widget(
            Paragraph::new(format!("Port: {}", form.port))
                .style(field_style(matches!(focused_field, ConnectionDialogField::Port))),
            form_layout[2],
        );
        
        frame.render_widget(
            Paragraph::new(format!("Password: {}", "*".repeat(form.password.len())))
                .style(field_style(matches!(focused_field, ConnectionDialogField::Password))),
            form_layout[3],
        );
        
        frame.render_widget(
            Paragraph::new(format!("Database: {}", form.database))
                .style(field_style(matches!(focused_field, ConnectionDialogField::Database))),
            form_layout[4],
        );
    }

    /// Renders the dialog buttons and help text
    fn render_dialog_buttons(state: &AppState, frame: &mut Frame, area: Rect) {
        let focused_field = &state.ui_state.connection_dialog.focused_field;
        
        let button_text = if matches!(focused_field, ConnectionDialogField::Buttons) {
            "[Save] [Cancel] - Enter:Select Tab:Navigate Esc:Cancel"
        } else {
            "Tab:Navigate Enter:Save Esc:Cancel"
        };
        
        frame.render_widget(
            Paragraph::new(button_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan)),
            area,
        );
    }
}