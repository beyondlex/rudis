use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Paragraph},
    prelude::Stylize,
};

use crate::app::{AppState, FocusedPanel};

/// UI renderer for the main application interface
pub struct AppRenderer;

impl AppRenderer {
    /// Renders the main application UI with all panels
    pub fn render(state: &AppState, frame: &mut Frame) {
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
        
        // Render all components
        Self::render_header(frame, main_layout[0]);
        Self::render_body_panels(state, frame, main_layout[1]);
        Self::render_command_panel(state, frame, main_layout[2]);
        Self::render_footer(state, frame, main_layout[3]);
    }

    /// Renders the application header
    fn render_header(frame: &mut Frame, area: Rect) {
        let title = Line::from("RUDIS - Redis TUI Client v0.1.0")
            .bold()
            .blue()
            .centered();
        frame.render_widget(
            Paragraph::new("")
                .block(Block::bordered().title(title))
                .style(Style::default().bg(Color::DarkGray)),
            area,
        );
    }

    /// Renders the main body panels (connections, database browser, key viewer)
    fn render_body_panels(state: &AppState, frame: &mut Frame, area: Rect) {
        // Create body layout: 3 horizontal panels
        let body_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Connections
                Constraint::Percentage(35), // Database/Keys
                Constraint::Percentage(40), // Key Viewer
            ])
            .split(area);

        Self::render_connections_panel(state, frame, body_layout[0]);
        Self::render_database_browser_panel(state, frame, body_layout[1]);
        Self::render_key_viewer_panel(state, frame, body_layout[2]);
    }

    /// Renders the connections panel
    fn render_connections_panel(state: &AppState, frame: &mut Frame, area: Rect) {
        let connections_count = state.connections.len();
        let connections_title = format!("Connections ({})", connections_count);
        
        let connections_content = if state.connections.is_empty() {
            "No connections\n\nPress 'c' to add\na new connection".to_string()
        } else {
            // List existing connections
            let mut content = String::new();
            for (id, connection) in &state.connections {
                let status_icon = match connection.status {
                    crate::redis::ConnectionStatus::Connected => "●",
                    crate::redis::ConnectionStatus::Connecting => "◐",
                    crate::redis::ConnectionStatus::Disconnected => "○",
                    crate::redis::ConnectionStatus::Failed(_) => "✗",
                    crate::redis::ConnectionStatus::Lost => "⚠",
                };
                let is_active = state.active_connection.as_ref() == Some(id);
                let marker = if is_active { "> " } else { "  " };
                content.push_str(&format!("{}{} {}\n", marker, status_icon, connection.config.name));
            }
            content.push_str("\nPress 'c' to add connection");
            content
        };
        
        // Style based on focus
        let is_focused = matches!(state.ui_state.focused_panel, FocusedPanel::ConnectionList);
        let style = if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        frame.render_widget(
            Paragraph::new(connections_content)
                .block(Block::bordered()
                    .title(connections_title)
                    .border_style(border_style))
                .style(style),
            area,
        );
    }

    /// Renders the database browser panel
    fn render_database_browser_panel(state: &AppState, frame: &mut Frame, area: Rect) {
        let browser_state = &state.ui_state.database_browser;
        let db_title = if state.active_connection.is_some() {
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
        
        let db_content = if state.active_connection.is_some() {
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
        let is_focused = matches!(state.ui_state.focused_panel, FocusedPanel::DatabaseBrowser);
        let style = if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        frame.render_widget(
            Paragraph::new(db_content)
                .block(Block::bordered()
                    .title(db_title)
                    .border_style(border_style))
                .style(style),
            area,
        );
    }

    /// Renders the key viewer panel
    fn render_key_viewer_panel(state: &AppState, frame: &mut Frame, area: Rect) {
        let viewer_title = "Key Viewer";
        let viewer_content = "Select a key\nto view its content";
        
        // Style based on focus
        let is_focused = matches!(state.ui_state.focused_panel, FocusedPanel::KeyViewer);
        let style = if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        frame.render_widget(
            Paragraph::new(viewer_content)
                .block(Block::bordered()
                    .title(viewer_title)
                    .border_style(border_style))
                .style(style),
            area,
        );
    }

    /// Renders the command input panel
    fn render_command_panel(state: &AppState, frame: &mut Frame, area: Rect) {
        let command_title = "Command Input (Redis CLI)";
        let command_state = &state.ui_state.command_input;
        let command_content = if command_state.input.is_empty() {
            "Type Redis commands here... (e.g., INFO, PING, GET mykey)"
        } else {
            &command_state.input
        };
        
        // Style based on focus
        let is_focused = matches!(state.ui_state.focused_panel, FocusedPanel::CommandInput);
        let style = if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        frame.render_widget(
            Paragraph::new(command_content)
                .block(Block::bordered()
                    .title(command_title)
                    .border_style(border_style))
                .style(style),
            area,
        );
    }

    /// Renders the footer with status and shortcuts
    fn render_footer(state: &AppState, frame: &mut Frame, area: Rect) {
        let status_text = state.status_message
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Ready");
        
        // Show current focused panel
        let focused_panel_name = match state.ui_state.focused_panel {
            FocusedPanel::ConnectionList => "Connections",
            FocusedPanel::DatabaseBrowser => "Database",
            FocusedPanel::KeyViewer => "Key Viewer",
            FocusedPanel::CommandInput => "Command",
        };
        
        let footer_text = format!(
            "{} | Focus: {} | q:Quit c:Connect Tab:Navigate ?:Help",
            status_text, focused_panel_name
        );
        
        frame.render_widget(
            Paragraph::new(footer_text)
                .block(Block::bordered())
                .style(Style::default().bg(Color::DarkGray)),
            area,
        );
    }
}