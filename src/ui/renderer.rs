use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Paragraph, Wrap, Scrollbar, ScrollbarOrientation, ScrollbarState},
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
        
        // Render dialog overlay if open
        if state.ui_state.connection_dialog.is_open {
            crate::ui::DialogRenderer::render_connection_dialog(state, frame, area);
        }
        
        // Render confirmation dialog if open
        if state.ui_state.confirmation_dialog.is_open {
            state.ui_state.confirmation_dialog.render(frame, area);
        }
        
        // Render export/import dialog if open
        if state.ui_state.export_import_dialog.is_open {
            state.ui_state.export_import_dialog.render(frame, area);
        }
        
        // Render bulk operations dialog if open
        if state.ui_state.bulk_operations_dialog.is_open {
            state.ui_state.bulk_operations_dialog.render(frame, area);
        }
        
        // Render progress bars if active
        if state.has_active_progress() {
            state.ui_state.progress_bar_manager.render(frame, area);
        }
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
                format!("Database {} - Search: {}", 
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
        
        // Create the main panel block
        let main_block = Block::bordered()
            .title(db_title)
            .border_style(border_style);
        let inner_area = main_block.inner(area);
        frame.render_widget(main_block, area);
        
        if state.active_connection.is_none() {
            // No connection - show message
            frame.render_widget(
                Paragraph::new("Select a connection\nto browse databases")
                    .style(style),
                inner_area,
            );
            return;
        }
        
        if browser_state.search_mode {
            // In search mode, show search instructions
            let search_content = format!("Search Mode\n\nPattern: {}\n\nType to search, Enter to apply\nEsc to cancel search", 
                if browser_state.filter_pattern.is_empty() { 
                    "<type pattern>"
                } else { 
                    &browser_state.filter_pattern 
                });
            frame.render_widget(
                Paragraph::new(search_content).style(style),
                inner_area,
            );
            return;
        }
        
        if browser_state.keys.is_empty() {
            // No keys - show loading or empty message
            let empty_content = if browser_state.loading {
                "Loading keys...".to_string()
            } else {
                "No keys found\n\nPress 'r' to refresh\nPress '/' to search".to_string()
            };
            frame.render_widget(
                Paragraph::new(empty_content).style(style),
                inner_area,
            );
            return;
        }
        
        // Calculate areas: keys area + help text area
        let help_text = if browser_state.use_tree_view {
            "r:Refresh /:Search t:List View Enter:Expand/Collapse del:Delete"
        } else {
            "r:Refresh /:Search t:Tree View del:Delete"
        };
        
        // Reserve space for help text (1 line) and potential "more keys" message (1 line)
        let reserved_lines = if !browser_state.scan_complete { 3 } else { 2 };
        let keys_area_height = inner_area.height.saturating_sub(reserved_lines);
        
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(keys_area_height), // Keys area
                Constraint::Min(0),                    // Help/status area
            ])
            .split(inner_area);
        
        let keys_area = layout[0];
        let help_area = layout[1];
        
        // Calculate how many keys can fit in the display area
        let available_lines = keys_area.height as usize;
        let keys_to_display = available_lines.min(10); // Max 10 keys, but respect available space
        
        // Render keys content
        let mut keys_content = String::new();
        
        if browser_state.use_tree_view {
            // Tree view display
            let visible_nodes = browser_state.key_tree.visible_nodes.iter()
                .enumerate()
                .skip(browser_state.scroll_offset)
                .take(keys_to_display);
            
            for (i, _node_path) in visible_nodes {
                let actual_index = browser_state.scroll_offset + i;
                let is_selected = actual_index == browser_state.selected_key_index;
                let marker = if is_selected { "> " } else { "  " };
                
                if let Some(display_info) = browser_state.key_tree.get_visible_node_info(actual_index) {
                    // Create indentation based on depth
                    let indent = "  ".repeat(display_info.depth);
                    
                    if display_info.is_key {
                        // This is an actual Redis key
                        let type_icon = if let Some(key_info) = &display_info.key_info {
                            match key_info.key_type.as_deref() {
                                Some("string") => "🔤",
                                Some("hash") => "📋",
                                Some("list") => "📜",
                                Some("set") => "📊",
                                Some("zset") => "📊",
                                Some("stream") => "🌊",
                                _ => "●",
                            }
                        } else {
                            "●"
                        };
                        
                        // Add TTL info if available
                        let ttl_info = if let Some(key_info) = &display_info.key_info {
                            match key_info.ttl {
                                Some(ttl) if ttl > 0 => format!(" ({}s)", ttl),
                                Some(-1) => " (no exp)".to_string(),
                                _ => String::new(),
                            }
                        } else {
                            String::new()
                        };
                        
                        // Truncate long key names
                        let display_name = if display_info.name.len() > 15 {
                            format!("{}...", &display_info.name[..12])
                        } else {
                            display_info.name.clone()
                        };
                        
                        // Add folder indicator if this key also has children
                        let folder_indicator = if display_info.has_children {
                            if display_info.is_expanded {
                                " 📂"
                            } else {
                                " 📁"
                            }
                        } else {
                            ""
                        };
                        
                        keys_content.push_str(&format!("{}{}{} {}{}{}\n", marker, indent, type_icon, display_name, ttl_info, folder_indicator));
                    } else {
                        // This is a folder/namespace node
                        let folder_icon = if display_info.is_expanded {
                            "📂"
                        } else {
                            "📁"
                        };
                        
                        keys_content.push_str(&format!("{}{}{} {}/\n", marker, indent, folder_icon, display_info.name));
                    }
                }
            }
        } else {
            // Flat list display
            let visible_keys = browser_state.keys.iter()
                .skip(browser_state.scroll_offset)
                .take(keys_to_display);
            
            for (i, key_info) in visible_keys.enumerate() {
                let actual_index = browser_state.scroll_offset + i;
                let is_selected = actual_index == browser_state.selected_key_index;
                let marker = if is_selected { "> " } else { "  " };
                
                // Key type icon
                let type_icon = match key_info.key_type.as_deref() {
                    Some("string") => "🔤",
                    Some("hash") => "📋",
                    Some("list") => "📜",
                    Some("set") => "📊",
                    Some("zset") => "📊",
                    Some("stream") => "🌊",
                    _ => "●",
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
                
                keys_content.push_str(&format!("{}{} {}{}\n", marker, type_icon, display_name, ttl_info));
            }
        }
        
        // Render keys content
        frame.render_widget(
            Paragraph::new(keys_content.trim_end()).style(style),
            keys_area,
        );
        
        // Render scrollbar if there are enough keys to scroll
        let total_items = if browser_state.use_tree_view {
            browser_state.key_tree.visible_count()
        } else {
            browser_state.keys.len()
        };
        
        if total_items > keys_to_display {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("┃"))
                .thumb_symbol("█");
                
            // Create scrollbar state with actual viewport size used in rendering
            // This ensures the scrollbar position accurately reflects the visual scroll state
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(total_items)
                .viewport_content_length(keys_to_display)
                .position(browser_state.scroll_offset);
                
            frame.render_stateful_widget(
                scrollbar,
                keys_area,
                &mut scrollbar_state,
            );
        }
        
        // Render help and status area
        let mut help_content = String::new();
        if !browser_state.scan_complete {
            help_content.push_str("[More keys available - scroll down]\n");
        }
        help_content.push_str(help_text);
        
        frame.render_widget(
            Paragraph::new(help_content).style(style),
            help_area,
        );
    }

    /// Renders the key viewer panel
    fn render_key_viewer_panel(state: &AppState, frame: &mut Frame, area: Rect) {
        let viewer_state = &state.ui_state.key_viewer;
        
        let viewer_title = if let Some(key_name) = &viewer_state.current_key {
            format!("Key Viewer - {}", key_name)
        } else {
            "Key Viewer".to_string()
        };
        
        let viewer_content = if viewer_state.loading {
            "Loading key value...".to_string()
        } else if let (Some(key_name), Some(value)) = (&viewer_state.current_key, &viewer_state.value) {
            // Use the value display component to render the content
            let display_lines = crate::ui::ValueDisplayComponent::render_value(
                key_name.clone(),
                value,
                viewer_state,
                20, // Max display items
            );
            
            // Convert lines to text for display
            display_lines.into_iter()
                .map(|line| {
                    line.spans.into_iter()
                        .map(|span| span.content.to_string())
                        .collect::<Vec<_>>()
                        .join("")
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "Select a key to view its content\n\nShortcuts:\n→ View selected key\nEnter: Refresh value\ne: Edit mode\nc: Copy value".to_string()
        };
        
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
                .style(style)
                .wrap(Wrap { trim: true }),
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