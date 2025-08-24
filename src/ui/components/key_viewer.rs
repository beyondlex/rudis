use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    prelude::Stylize,
};

use crate::redis::value_types::{RedisValue, StreamEntry};
use crate::app::states::KeyViewerState;
use crate::app::state_core::{HashEditMode, ListEditMode, SetEditMode, ZSetEditMode, StreamViewMode};

/// Component for rendering Redis values in the key viewer panel
pub struct ValueDisplayComponent;

impl ValueDisplayComponent {
    /// Render a Redis value with appropriate formatting based on its type
    pub fn render_value(
        key_name: String,
        value: &RedisValue,
        viewer_state: &KeyViewerState,
        max_display_items: usize,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        // Add header with key name and type
        lines.push(Line::from(vec![
            Span::styled("Key: ", Style::default().fg(Color::Cyan)),
            Span::styled(key_name, Style::default().fg(Color::White).bold()),
            Span::styled(" (", Style::default().fg(Color::Gray)),
            Span::styled(value.type_name(), Style::default().fg(Color::Yellow)),
            Span::styled(")", Style::default().fg(Color::Gray)),
        ]));
        
        lines.push(Line::from(""));
        
        // Render value content based on type
        match value {
            RedisValue::String(s) => {
                Self::render_string_value(s.clone(), &mut lines, viewer_state);
            }
            RedisValue::Hash(fields) => {
                Self::render_hash_value(fields.clone(), &mut lines, viewer_state, max_display_items);
            }
            RedisValue::List(elements) => {
                Self::render_list_value(elements.clone(), &mut lines, viewer_state, max_display_items);
            }
            RedisValue::Set(members) => {
                Self::render_set_value(members.clone(), &mut lines, viewer_state, max_display_items);
            }
            RedisValue::ZSet(members) => {
                Self::render_zset_value(members.clone(), &mut lines, viewer_state, max_display_items);
            }
            RedisValue::Stream(entries) => {
                Self::render_stream_value(entries.clone(), &mut lines, viewer_state, max_display_items);
            }
            RedisValue::Unknown(type_name) => {
                lines.push(Line::from(vec![
                    Span::styled("Unknown type: ", Style::default().fg(Color::Red)),
                    Span::styled(type_name.clone(), Style::default().fg(Color::White)),
                ]));
            }
        }
        
        lines
    }

    /// Render string value with line wrapping and edit mode support
    fn render_string_value(
        value: String,
        lines: &mut Vec<Line<'static>>,
        viewer_state: &KeyViewerState,
    ) {
        let display_value = if viewer_state.edit_mode {
            viewer_state.edit_buffer.clone()
        } else {
            value
        };
        
        // Analyze data for binary content
        let data_bytes = display_value.as_bytes();
        let binary_info = crate::ui::BinaryViewer::analyze_data(data_bytes);
        
        lines.push(Line::from(vec![
            Span::styled("Length: ", Style::default().fg(Color::Cyan)),
            Span::styled(display_value.len().to_string(), Style::default().fg(Color::White)),
            Span::styled(" characters", Style::default().fg(Color::Gray)),
        ]));
        
        // Show encoding information
        if let Some(encoding) = &binary_info.encoding {
            lines.push(Line::from(vec![
                Span::styled("Encoding: ", Style::default().fg(Color::Cyan)),
                Span::styled(encoding.clone(), Style::default().fg(Color::Yellow)),
            ]));
        }
        
        // Show binary content warning if applicable
        if binary_info.has_binary_content {
            lines.push(Line::from(vec![
                Span::styled("⚠ Binary data detected", Style::default().fg(Color::Yellow)),
                Span::styled(format!(" ({} null, {} ctrl)", binary_info.null_bytes, binary_info.control_chars),
                           Style::default().fg(Color::Gray)),
            ]));
        }
        
        if viewer_state.edit_mode {
            lines.push(Line::from(vec![
                Span::styled("Changes: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    if viewer_state.has_unsaved_changes { "Modified" } else { "None" },
                    if viewer_state.has_unsaved_changes {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Green)
                    }
                ),
            ]));
            
            // Show validation status
            let validation = viewer_state.validate_edit_buffer();
            match validation {
                crate::ui::validation::ValidationResult::Valid => {
                    lines.push(Line::from(vec![
                        Span::styled("Validation: ", Style::default().fg(Color::Cyan)),
                        Span::styled("✓ Valid", Style::default().fg(Color::Green)),
                    ]));
                }
                crate::ui::validation::ValidationResult::Warning(ref msg) => {
                    lines.push(Line::from(vec![
                        Span::styled("Validation: ", Style::default().fg(Color::Cyan)),
                        Span::styled(format!("⚠ {}", msg), Style::default().fg(Color::Yellow)),
                    ]));
                }
                crate::ui::validation::ValidationResult::Error(ref msg) => {
                    lines.push(Line::from(vec![
                        Span::styled("Validation: ", Style::default().fg(Color::Cyan)),
                        Span::styled(format!("✗ {}", msg), Style::default().fg(Color::Red)),
                    ]));
                }
            }
            
            // Show JSON validation if applicable and enabled
            if viewer_state.json_highlighting_enabled {
                if let Some(json_validation) = viewer_state.validate_json() {
                    match json_validation {
                        crate::ui::validation::ValidationResult::Valid => {
                            lines.push(Line::from(vec![
                                Span::styled("JSON: ", Style::default().fg(Color::Cyan)),
                                Span::styled("✓ Valid JSON", Style::default().fg(Color::Green)),
                            ]));
                        }
                        crate::ui::validation::ValidationResult::Error(ref msg) => {
                            lines.push(Line::from(vec![
                                Span::styled("JSON: ", Style::default().fg(Color::Cyan)),
                                Span::styled(format!("✗ {}", msg), Style::default().fg(Color::Red)),
                            ]));
                        }
                        _ => {}
                    }
                }
            }
        }
        
        lines.push(Line::from(""));
        
        if viewer_state.edit_mode {
            lines.push(Line::from(vec![
                Span::styled("EDIT MODE", Style::default().fg(Color::Black).bg(Color::Yellow)),
            ]));
            lines.push(Line::from(""));
            
            // Render the edit buffer with cursor
            Self::render_editable_text(display_value.clone(), viewer_state, lines);
            
        } else {
            // Check if we should use binary display mode
            let should_use_binary_mode = binary_info.has_binary_content || 
                viewer_state.binary_display_mode != crate::ui::binary_viewer::DisplayMode::Auto;
            
            if should_use_binary_mode {
                // Use binary viewer
                let binary_lines = crate::ui::BinaryViewer::display_data(
                    data_bytes,
                    viewer_state.binary_display_mode.clone(),
                    20, // max lines
                );
                lines.extend(binary_lines);
            } else {
                // Regular text display with optional JSON highlighting
                if viewer_state.json_highlighting_enabled && 
                   crate::ui::JsonHighlighter::is_json_like(&display_value) {
                    // Use JSON highlighter
                    let json_lines = crate::ui::JsonHighlighter::highlight_json(&display_value, 20);
                    lines.extend(json_lines);
                } else {
                    // Plain text display
                    for line in display_value.lines() {
                        lines.push(Line::from(line.to_string()));
                    }
                }
            }
        }
        
        lines.push(Line::from(""));
        
        if viewer_state.edit_mode {
            lines.push(Line::from(vec![
                Span::styled("Enter: Save | Esc: Cancel | Ctrl+E: Exit Edit", 
                           Style::default().fg(Color::Yellow)),
            ]));
            if viewer_state.has_unsaved_changes {
                lines.push(Line::from(vec![
                    Span::styled("⚠ Unsaved changes", Style::default().fg(Color::Red)),
                ]));
            }
        } else {
            let mut control_text = "e: External Editor ($EDITOR) | Enter: View Raw | Ctrl+C: Copy".to_string();
            
            // Add binary mode controls if binary data is detected
            if binary_info.has_binary_content {
                control_text.push_str(" | m: Toggle Display Mode");
            }
            
            // Add JSON controls if JSON-like content
            if crate::ui::JsonHighlighter::is_json_like(&display_value) {
                control_text.push_str(" | j: Toggle JSON Highlighting");
            }
            
            lines.push(Line::from(vec![
                Span::styled(control_text, Style::default().fg(Color::Gray)),
            ]));
        }
    }
    
    /// Render editable text with cursor position
    fn render_editable_text(
        text: String,
        viewer_state: &KeyViewerState,
        lines: &mut Vec<Line<'static>>,
    ) {
        let cursor_pos = viewer_state.edit_cursor_position;
        
        // Handle multi-line text
        let text_lines: Vec<&str> = text.lines().collect();
        
        if text_lines.is_empty() {
            // Empty text, just show cursor
            lines.push(Line::from(vec![
                Span::styled("|", Style::default().fg(Color::Yellow).bg(Color::Gray)),
            ]));
        } else {
            let mut char_count = 0;
            let mut cursor_found = false;
            
            for (line_idx, line) in text_lines.iter().enumerate() {
                let line_start = char_count;
                let line_end = char_count + line.len();
                
                // Check if cursor is in this line
                if !cursor_found && cursor_pos >= line_start && cursor_pos <= line_end {
                    // Cursor is in this line
                    let relative_pos = cursor_pos - line_start;
                    
                    let (before, after) = if relative_pos == 0 {
                        ("", *line)
                    } else if relative_pos >= line.len() {
                        (*line, "")
                    } else {
                        line.split_at(relative_pos)
                    };
                    
                    let mut spans = Vec::new();
                    
                    if !before.is_empty() {
                        spans.push(Span::styled(before.to_string(), Style::default().fg(Color::White)));
                    }
                    
                    // Add cursor
                    if relative_pos == line.len() {
                        spans.push(Span::styled("|", Style::default().fg(Color::Yellow).bg(Color::Gray)));
                    } else {
                        let cursor_char = after.chars().next().unwrap_or(' ');
                        spans.push(Span::styled(
                            cursor_char.to_string(),
                            Style::default().fg(Color::Black).bg(Color::Yellow)
                        ));
                        
                        if after.len() > 1 {
                            spans.push(Span::styled(after[1..].to_string(), Style::default().fg(Color::White)));
                        }
                    }
                    
                    lines.push(Line::from(spans));
                    cursor_found = true;
                } else {
                    // Regular line without cursor
                    lines.push(Line::from(vec![
                        Span::styled(line.to_string(), Style::default().fg(Color::White)),
                    ]));
                }
                
                // Account for the newline character (except for the last line)
                char_count = line_end;
                if line_idx < text_lines.len() - 1 {
                    char_count += 1; // +1 for newline
                }
            }
            
            // If cursor is at the very end, add it after the last line
            if !cursor_found && cursor_pos == text.len() {
                lines.push(Line::from(vec![
                    Span::styled("|", Style::default().fg(Color::Yellow).bg(Color::Gray)),
                ]));
            }
        }
    }

    /// Render hash value as field-value pairs with editing support
    fn render_hash_value(
        fields: Vec<(String, String)>,
        lines: &mut Vec<Line<'static>>,
        viewer_state: &KeyViewerState,
        max_display_items: usize,
    ) {
        lines.push(Line::from(vec![
            Span::styled("Fields: ", Style::default().fg(Color::Cyan)),
            Span::styled(fields.len().to_string(), Style::default().fg(Color::White)),
        ]));
        
        // Show edit mode status
        match viewer_state.hash_edit_mode {
            HashEditMode::Field => {
                lines.push(Line::from(vec![
                    Span::styled("EDIT FIELD NAME", Style::default().fg(Color::Black).bg(Color::Yellow)),
                ]));
            }
            HashEditMode::Value => {
                lines.push(Line::from(vec![
                    Span::styled("EDIT FIELD VALUE", Style::default().fg(Color::Black).bg(Color::Yellow)),
                ]));
            }
            HashEditMode::NewField => {
                lines.push(Line::from(vec![
                    Span::styled("ADD NEW FIELD", Style::default().fg(Color::Black).bg(Color::Green)),
                ]));
            }
            HashEditMode::None => {}
        }
        
        lines.push(Line::from(""));
        
        let start_idx = viewer_state.current_page * viewer_state.page_size;
        let end_idx = (start_idx + max_display_items).min(fields.len());
        
        // Show existing fields
        for (i, (field, value)) in fields.iter().enumerate().skip(start_idx).take(end_idx - start_idx) {
            let is_selected = viewer_state.hash_field_index == i;
            let is_editing_field = is_selected && viewer_state.hash_edit_mode == HashEditMode::Field;
            let is_editing_value = is_selected && viewer_state.hash_edit_mode == HashEditMode::Value;
            
            // Selection indicator
            let selection_marker = if is_selected { "> " } else { "  " };
            
            // Field display
            let field_display = if is_editing_field {
                format!("{}_", viewer_state.hash_field_buffer) // Show cursor
            } else {
                if field.len() > 20 {
                    format!("{}...", &field[..17])
                } else {
                    field.clone()
                }
            };
            
            // Value display
            let value_display = if is_editing_value {
                format!("{}_", viewer_state.hash_value_buffer) // Show cursor
            } else {
                if value.len() > 50 {
                    format!("{}...", &value[..47])
                } else {
                    value.clone()
                }
            };
            
            // Style based on selection and editing state
            let field_style = if is_editing_field {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else if is_selected {
                Style::default().fg(Color::Cyan).bold().bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Cyan).bold()
            };
            
            let value_style = if is_editing_value {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else if is_selected {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            
            lines.push(Line::from(vec![
                Span::styled(selection_marker, Style::default().fg(Color::Yellow)),
                Span::styled(format!("{:2}: ", i + 1), Style::default().fg(Color::Gray)),
                Span::styled(field_display, field_style),
                Span::styled(" = ", Style::default().fg(Color::Gray)),
                Span::styled(value_display, value_style),
            ]));
        }
        
        // Show new field editor if in NewField mode
        if viewer_state.hash_edit_mode == HashEditMode::NewField {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Green)),
                Span::styled("NEW: ", Style::default().fg(Color::Green).bold()),
                Span::styled(
                    format!("{}_", viewer_state.hash_field_buffer),
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                ),
                Span::styled(" = ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}_", viewer_state.hash_value_buffer),
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                ),
            ]));
        }
        
        lines.push(Line::from(""));
        
        // Show controls based on edit mode
        match viewer_state.hash_edit_mode {
            HashEditMode::None => {
                lines.push(Line::from(vec![
                    Span::styled("↑/↓: Select | Enter: Edit Value | f: Edit Field | n: New Field | Del: Delete", 
                               Style::default().fg(Color::Gray)),
                ]));
            }
            HashEditMode::Field | HashEditMode::Value => {
                lines.push(Line::from(vec![
                    Span::styled("Enter: Save | Esc: Cancel | Tab: Switch Field/Value", 
                               Style::default().fg(Color::Yellow)),
                ]));
                if viewer_state.has_unsaved_changes {
                    lines.push(Line::from(vec![
                        Span::styled("⚠ Unsaved changes", Style::default().fg(Color::Red)),
                    ]));
                }
            }
            HashEditMode::NewField => {
                lines.push(Line::from(vec![
                    Span::styled("Enter: Add Field | Esc: Cancel | Tab: Switch Field/Value", 
                               Style::default().fg(Color::Yellow)),
                ]));
            }
        }
        
        Self::add_pagination_info(lines, viewer_state, fields.len());
    }

    /// Render list value as indexed elements with editing support
    fn render_list_value(
        elements: Vec<String>,
        lines: &mut Vec<Line<'static>>,
        viewer_state: &KeyViewerState,
        max_display_items: usize,
    ) {
        lines.push(Line::from(vec![
            Span::styled("Elements: ", Style::default().fg(Color::Cyan)),
            Span::styled(elements.len().to_string(), Style::default().fg(Color::White)),
        ]));
        
        // Show edit mode status
        match viewer_state.list_edit_mode {
            ListEditMode::Element => {
                lines.push(Line::from(vec![
                    Span::styled("EDIT ELEMENT", Style::default().fg(Color::Black).bg(Color::Yellow)),
                ]));
            }
            ListEditMode::Insert => {
                lines.push(Line::from(vec![
                    Span::styled("INSERT ELEMENT", Style::default().fg(Color::Black).bg(Color::Green)),
                ]));
            }
            ListEditMode::Append => {
                lines.push(Line::from(vec![
                    Span::styled("APPEND ELEMENT", Style::default().fg(Color::Black).bg(Color::Green)),
                ]));
            }
            ListEditMode::None => {}
        }
        
        lines.push(Line::from(""));
        
        let start_idx = viewer_state.current_page * viewer_state.page_size;
        let end_idx = (start_idx + max_display_items).min(elements.len());
        
        // Show existing elements
        for (i, element) in elements.iter().enumerate().skip(start_idx).take(end_idx - start_idx) {
            let is_selected = viewer_state.list_element_index == i;
            let is_editing = is_selected && viewer_state.list_edit_mode == ListEditMode::Element;
            let is_insert_position = viewer_state.list_edit_mode == ListEditMode::Insert 
                && viewer_state.list_insert_index == Some(i);
            
            // Show insert editor above current element if this is insert position
            if is_insert_position {
                lines.push(Line::from(vec![
                    Span::styled("> ", Style::default().fg(Color::Green)),
                    Span::styled("INSERT: ", Style::default().fg(Color::Green).bold()),
                    Span::styled(
                        format!("{}_", viewer_state.list_element_buffer),
                        Style::default().fg(Color::Black).bg(Color::Yellow)
                    ),
                ]));
            }
            
            // Selection indicator
            let selection_marker = if is_selected { "> " } else { "  " };
            
            // Element display
            let element_display = if is_editing {
                format!("{}_", viewer_state.list_element_buffer) // Show cursor
            } else {
                if element.len() > 60 {
                    format!("{}...", &element[..57])
                } else {
                    element.clone()
                }
            };
            
            // Style based on selection and editing state
            let element_style = if is_editing {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else if is_selected {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            
            lines.push(Line::from(vec![
                Span::styled(selection_marker, Style::default().fg(Color::Yellow)),
                Span::styled(format!("[{}]: ", i), Style::default().fg(Color::Cyan)),
                Span::styled(element_display, element_style),
            ]));
        }
        
        // Show append editor if in Append mode
        if viewer_state.list_edit_mode == ListEditMode::Append {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Green)),
                Span::styled(format!("[{}]: ", elements.len()), Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{}_", viewer_state.list_element_buffer),
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                ),
            ]));
        }
        
        lines.push(Line::from(""));
        
        // Show controls based on edit mode
        match viewer_state.list_edit_mode {
            ListEditMode::None => {
                lines.push(Line::from(vec![
                    Span::styled("↑/↓: Select | Enter: Edit | i: Insert | a: Append | Del: Delete | ↑↓: Move", 
                               Style::default().fg(Color::Gray)),
                ]));
            }
            ListEditMode::Element | ListEditMode::Insert | ListEditMode::Append => {
                lines.push(Line::from(vec![
                    Span::styled("Enter: Save | Esc: Cancel", 
                               Style::default().fg(Color::Yellow)),
                ]));
                if viewer_state.has_unsaved_changes {
                    lines.push(Line::from(vec![
                        Span::styled("⚠ Unsaved changes", Style::default().fg(Color::Red)),
                    ]));
                }
            }
        }
        
        Self::add_pagination_info(lines, viewer_state, elements.len());
    }

    /// Render set value as unique members with editing support
    fn render_set_value(
        members: Vec<String>,
        lines: &mut Vec<Line<'static>>,
        viewer_state: &KeyViewerState,
        max_display_items: usize,
    ) {
        lines.push(Line::from(vec![
            Span::styled("Members: ", Style::default().fg(Color::Cyan)),
            Span::styled(members.len().to_string(), Style::default().fg(Color::White)),
        ]));
        
        // Show edit mode status
        match viewer_state.set_edit_mode {
            SetEditMode::Add => {
                lines.push(Line::from(vec![
                    Span::styled("ADD MEMBER", Style::default().fg(Color::Black).bg(Color::Green)),
                ]));
            }
            SetEditMode::Remove => {
                lines.push(Line::from(vec![
                    Span::styled("REMOVE MEMBER", Style::default().fg(Color::Black).bg(Color::Red)),
                ]));
            }
            SetEditMode::None => {}
        }
        
        lines.push(Line::from(""));
        
        let start_idx = viewer_state.current_page * viewer_state.page_size;
        let end_idx = (start_idx + max_display_items).min(members.len());
        
        // Show existing members
        for (i, member) in members.iter().enumerate().skip(start_idx).take(end_idx - start_idx) {
            let is_selected = viewer_state.set_member_index == i;
            let is_removing = is_selected && viewer_state.set_edit_mode == SetEditMode::Remove;
            
            // Selection indicator
            let selection_marker = if is_selected { "> " } else { "  " };
            
            // Member display
            let member_display = if member.len() > 60 {
                format!("{}...", &member[..57])
            } else {
                member.clone()
            };
            
            // Style based on selection and removal state
            let member_style = if is_removing {
                Style::default().fg(Color::White).bg(Color::Red) // Highlight for removal
            } else if is_selected {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            
            let mut spans = vec![
                Span::styled(selection_marker, Style::default().fg(Color::Yellow)),
                Span::styled("• ", Style::default().fg(Color::Cyan)),
                Span::styled(member_display, member_style),
            ];
            
            // Add removal indicator
            if is_removing {
                spans.push(Span::styled(" [CONFIRM DELETE?]", Style::default().fg(Color::Red).bold()));
            }
            
            lines.push(Line::from(spans));
        }
        
        // Show add member editor if in Add mode
        if viewer_state.set_edit_mode == SetEditMode::Add {
            lines.push(Line::from(""));
            
            // Check if member already exists
            let exists = if !viewer_state.set_member_buffer.is_empty() {
                viewer_state.set_member_exists(&viewer_state.set_member_buffer)
            } else {
                false
            };
            
            let add_style = if exists {
                Style::default().fg(Color::Black).bg(Color::Red) // Duplicate member
            } else {
                Style::default().fg(Color::Black).bg(Color::Yellow) // Normal input
            };
            
            let mut add_spans = vec![
                Span::styled("> ", Style::default().fg(Color::Green)),
                Span::styled("ADD: ", Style::default().fg(Color::Green).bold()),
                Span::styled(
                    format!("{}_", viewer_state.set_member_buffer),
                    add_style
                ),
            ];
            
            if exists {
                add_spans.push(Span::styled(" [ALREADY EXISTS]", Style::default().fg(Color::Red).bold()));
            }
            
            lines.push(Line::from(add_spans));
        }
        
        lines.push(Line::from(""));
        
        // Show controls based on edit mode
        match viewer_state.set_edit_mode {
            SetEditMode::None => {
                lines.push(Line::from(vec![
                    Span::styled("↑/↓: Select | a: Add Member | Del: Remove Member", 
                               Style::default().fg(Color::Gray)),
                ]));
            }
            SetEditMode::Add => {
                lines.push(Line::from(vec![
                    Span::styled("Enter: Add Member | Esc: Cancel", 
                               Style::default().fg(Color::Yellow)),
                ]));
                if viewer_state.has_unsaved_changes {
                    lines.push(Line::from(vec![
                        Span::styled("⚠ Type member name", Style::default().fg(Color::Gray)),
                    ]));
                }
            }
            SetEditMode::Remove => {
                lines.push(Line::from(vec![
                    Span::styled("Enter: Confirm Remove | Esc: Cancel", 
                               Style::default().fg(Color::Red)),
                ]));
            }
        }
        
        Self::add_pagination_info(lines, viewer_state, members.len());
    }

    /// Render sorted set value with scores and editing support
    fn render_zset_value(
        members: Vec<(String, f64)>,
        lines: &mut Vec<Line<'static>>,
        viewer_state: &KeyViewerState,
        max_display_items: usize,
    ) {
        lines.push(Line::from(vec![
            Span::styled("Members: ", Style::default().fg(Color::Cyan)),
            Span::styled(members.len().to_string(), Style::default().fg(Color::White)),
        ]));
        
        // Show score range if we have members
        if !members.is_empty() {
            let min_score = members.first().map(|(_, s)| *s).unwrap_or(0.0);
            let max_score = members.last().map(|(_, s)| *s).unwrap_or(0.0);
            lines.push(Line::from(vec![
                Span::styled("Score Range: ", Style::default().fg(Color::Cyan)),
                Span::styled(format!("{:.2} - {:.2}", min_score, max_score), Style::default().fg(Color::White)),
            ]));
        }
        
        // Show edit mode status
        match viewer_state.zset_edit_mode {
            ZSetEditMode::Add => {
                lines.push(Line::from(vec![
                    Span::styled("ADD MEMBER", Style::default().fg(Color::Black).bg(Color::Green)),
                ]));
            }
            ZSetEditMode::UpdateScore => {
                lines.push(Line::from(vec![
                    Span::styled("UPDATE SCORE", Style::default().fg(Color::Black).bg(Color::Yellow)),
                ]));
            }
            ZSetEditMode::Remove => {
                lines.push(Line::from(vec![
                    Span::styled("REMOVE MEMBER", Style::default().fg(Color::Black).bg(Color::Red)),
                ]));
            }
            ZSetEditMode::None => {}
        }
        
        lines.push(Line::from(""));
        
        let start_idx = viewer_state.current_page * viewer_state.page_size;
        let end_idx = (start_idx + max_display_items).min(members.len());
        
        // Show existing members
        for (i, (member, score)) in members.iter().enumerate().skip(start_idx).take(end_idx - start_idx) {
            let is_selected = viewer_state.zset_member_index == i;
            let is_removing = is_selected && viewer_state.zset_edit_mode == ZSetEditMode::Remove;
            let is_updating_score = is_selected && viewer_state.zset_edit_mode == ZSetEditMode::UpdateScore;
            
            // Selection indicator
            let selection_marker = if is_selected { "> " } else { "  " };
            
            // Member display
            let member_display = if member.len() > 35 {
                format!("{}...", &member[..32])
            } else {
                member.clone()
            };
            
            // Score display
            let score_display = if is_updating_score {
                format!("{}_", viewer_state.zset_score_buffer) // Show edit buffer with cursor
            } else {
                format!("{:8.2}", score)
            };
            
            // Style based on selection and editing state
            let member_style = if is_removing {
                Style::default().fg(Color::White).bg(Color::Red) // Highlight for removal
            } else if is_selected {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            
            let score_style = if is_updating_score {
                Style::default().fg(Color::Black).bg(Color::Yellow) // Editing score
            } else if is_selected {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Yellow)
            };
            
            let mut spans = vec![
                Span::styled(selection_marker, Style::default().fg(Color::Yellow)),
                Span::styled(score_display, score_style),
                Span::styled(" → ", Style::default().fg(Color::Gray)),
                Span::styled(member_display, member_style),
            ];
            
            // Add operation indicator
            if is_removing {
                spans.push(Span::styled(" [CONFIRM DELETE?]", Style::default().fg(Color::Red).bold()));
            } else if is_updating_score {
                spans.push(Span::styled(" [EDITING SCORE]", Style::default().fg(Color::Yellow).bold()));
            }
            
            lines.push(Line::from(spans));
        }
        
        // Show add member editor if in Add mode
        if viewer_state.zset_edit_mode == ZSetEditMode::Add {
            lines.push(Line::from(""));
            
            // Validate score input
            let score_valid = viewer_state.is_valid_score();
            let score_style = if score_valid || viewer_state.zset_score_buffer.is_empty() {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default().fg(Color::Black).bg(Color::Red) // Invalid score
            };
            
            let mut add_spans = vec![
                Span::styled("> ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{}_", viewer_state.zset_score_buffer),
                    score_style
                ),
                Span::styled(" → ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}_", viewer_state.zset_member_buffer),
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                ),
            ];
            
            if !score_valid && !viewer_state.zset_score_buffer.is_empty() {
                add_spans.push(Span::styled(" [INVALID SCORE]", Style::default().fg(Color::Red).bold()));
            }
            
            lines.push(Line::from(add_spans));
        }
        
        lines.push(Line::from(""));
        
        // Show controls based on edit mode
        match viewer_state.zset_edit_mode {
            ZSetEditMode::None => {
                lines.push(Line::from(vec![
                    Span::styled("↑/↓: Select | a: Add Member | s: Update Score | Del: Remove", 
                               Style::default().fg(Color::Gray)),
                ]));
            }
            ZSetEditMode::Add => {
                lines.push(Line::from(vec![
                    Span::styled("Tab: Switch Score/Member | Enter: Add | Esc: Cancel", 
                               Style::default().fg(Color::Yellow)),
                ]));
                if viewer_state.has_unsaved_changes {
                    lines.push(Line::from(vec![
                        Span::styled("⚠ Enter score (number) and member name", Style::default().fg(Color::Gray)),
                    ]));
                }
            }
            ZSetEditMode::UpdateScore => {
                lines.push(Line::from(vec![
                    Span::styled("Enter: Update Score | Esc: Cancel", 
                               Style::default().fg(Color::Yellow)),
                ]));
                if !viewer_state.is_valid_score() && !viewer_state.zset_score_buffer.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("⚠ Invalid score format", Style::default().fg(Color::Red)),
                    ]));
                }
            }
            ZSetEditMode::Remove => {
                lines.push(Line::from(vec![
                    Span::styled("Enter: Confirm Remove | Esc: Cancel", 
                               Style::default().fg(Color::Red)),
                ]));
            }
        }
        
        Self::add_pagination_info(lines, viewer_state, members.len());
    }

    /// Render stream value with entries and interactive field display
    fn render_stream_value(
        entries: Vec<StreamEntry>,
        lines: &mut Vec<Line<'static>>,
        viewer_state: &KeyViewerState,
        max_display_items: usize,
    ) {
        lines.push(Line::from(vec![
            Span::styled("Entries: ", Style::default().fg(Color::Cyan)),
            Span::styled(entries.len().to_string(), Style::default().fg(Color::White)),
        ]));
        
        // Show view mode
        let view_mode_text = match viewer_state.stream_view_mode {
            StreamViewMode::List => "List View",
            StreamViewMode::Detail => "Detail View",
        };
        lines.push(Line::from(vec![
            Span::styled("Mode: ", Style::default().fg(Color::Cyan)),
            Span::styled(view_mode_text, Style::default().fg(Color::Yellow)),
            Span::styled(" (Space: Toggle)", Style::default().fg(Color::Gray)),
        ]));
        
        lines.push(Line::from(""));
        
        match viewer_state.stream_view_mode {
            StreamViewMode::List => {
                Self::render_stream_list_view(&entries, lines, viewer_state, max_display_items);
            }
            StreamViewMode::Detail => {
                Self::render_stream_detail_view(&entries, lines, viewer_state);
            }
        }
        
        lines.push(Line::from(""));
        
        // Show controls based on view mode
        match viewer_state.stream_view_mode {
            StreamViewMode::List => {
                lines.push(Line::from(vec![
                    Span::styled("↑/↓: Select Entry | Enter/Space: Detail View | PgUp/PgDn: Page", 
                               Style::default().fg(Color::Gray)),
                ]));
            }
            StreamViewMode::Detail => {
                lines.push(Line::from(vec![
                    Span::styled("↑/↓: Navigate Entry | ←/→: Navigate Fields | Space: List View", 
                               Style::default().fg(Color::Gray)),
                ]));
            }
        }
        
        if viewer_state.stream_view_mode == StreamViewMode::List {
            Self::add_pagination_info(lines, viewer_state, entries.len());
        }
    }
    
    /// Render stream entries in list view
    fn render_stream_list_view(
        entries: &[StreamEntry],
        lines: &mut Vec<Line<'static>>,
        viewer_state: &KeyViewerState,
        max_display_items: usize,
    ) {
        let start_idx = viewer_state.current_page * viewer_state.page_size;
        let end_idx = (start_idx + max_display_items.saturating_div(2)).min(entries.len()); // 2 lines per entry
        
        for (i, entry) in entries.iter().enumerate().skip(start_idx).take(end_idx - start_idx) {
            let is_selected = viewer_state.stream_entry_index == i;
            
            // Selection indicator
            let selection_marker = if is_selected { "> " } else { "  " };
            
            // Format timestamp from entry ID
            let formatted_id = entry.formatted_id();
            
            // Entry header with ID and timestamp
            let header_style = if is_selected {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Yellow)
            };
            
            lines.push(Line::from(vec![
                Span::styled(selection_marker, Style::default().fg(Color::Yellow)),
                Span::styled("Entry: ", Style::default().fg(Color::Cyan)),
                Span::styled(formatted_id, header_style),
            ]));
            
            // Show field summary
            let field_summary = if entry.fields.len() <= 3 {
                entry.fields.iter()
                    .map(|(k, v)| {
                        let v_display = if v.len() > 20 { format!("{}...", &v[..17]) } else { v.clone() };
                        format!("{}={}", k, v_display)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                format!("{}={}, {}={}, ... ({} fields)", 
                       entry.fields[0].0, 
                       if entry.fields[0].1.len() > 15 { format!("{}...", &entry.fields[0].1[..12]) } else { entry.fields[0].1.clone() },
                       entry.fields[1].0,
                       if entry.fields[1].1.len() > 15 { format!("{}...", &entry.fields[1].1[..12]) } else { entry.fields[1].1.clone() },
                       entry.fields.len())
            };
            
            let field_style = if is_selected {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            
            lines.push(Line::from(vec![
                Span::styled("  Fields: ", Style::default().fg(Color::Gray)),
                Span::styled(field_summary, field_style),
            ]));
        }
    }
    
    /// Render detailed view of selected stream entry
    fn render_stream_detail_view(
        entries: &[StreamEntry],
        lines: &mut Vec<Line<'static>>,
        viewer_state: &KeyViewerState,
    ) {
        if let Some(entry) = entries.get(viewer_state.stream_entry_index) {
            // Entry header
            lines.push(Line::from(vec![
                Span::styled("Entry ID: ", Style::default().fg(Color::Cyan)),
                Span::styled(entry.id.clone(), Style::default().fg(Color::Yellow).bold()),
            ]));
            
            // Formatted timestamp
            let formatted_id = entry.formatted_id();
            if formatted_id != entry.id {
                lines.push(Line::from(vec![
                    Span::styled("Timestamp: ", Style::default().fg(Color::Cyan)),
                    Span::styled(formatted_id, Style::default().fg(Color::Gray)),
                ]));
            }
            
            lines.push(Line::from(vec![
                Span::styled("Fields: ", Style::default().fg(Color::Cyan)),
                Span::styled(entry.fields.len().to_string(), Style::default().fg(Color::White)),
            ]));
            
            lines.push(Line::from(""));
            
            // Show all fields with selection
            for (i, (field, value)) in entry.fields.iter().enumerate() {
                let is_selected_field = viewer_state.stream_field_index == i;
                let selection_marker = if is_selected_field { "> " } else { "  " };
                
                // Field name
                let field_style = if is_selected_field {
                    Style::default().fg(Color::Cyan).bg(Color::DarkGray).bold()
                } else {
                    Style::default().fg(Color::Cyan).bold()
                };
                
                lines.push(Line::from(vec![
                    Span::styled(selection_marker.to_string(), Style::default().fg(Color::Yellow)),
                    Span::styled(field.clone(), field_style),
                    Span::styled(":", Style::default().fg(Color::Gray)),
                ]));
                
                // Field value (potentially multi-line)
                let value_lines: Vec<&str> = value.lines().collect();
                for (line_idx, value_line) in value_lines.iter().enumerate() {
                    let prefix = if line_idx == 0 { "  → " } else { "    " };
                    
                    let value_style = if is_selected_field {
                        Style::default().fg(Color::White).bg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    
                    // Handle long lines
                    if value_line.len() > 80 {
                        let chunks: Vec<&str> = value_line.as_bytes()
                            .chunks(80)
                            .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                            .collect();
                        
                        for (chunk_idx, chunk) in chunks.iter().enumerate() {
                            let chunk_prefix = if chunk_idx == 0 { prefix } else { "    " };
                            lines.push(Line::from(vec![
                                Span::styled(chunk_prefix.to_string(), Style::default().fg(Color::Gray)),
                                Span::styled(chunk.to_string(), value_style),
                            ]));
                        }
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled(prefix.to_string(), Style::default().fg(Color::Gray)),
                            Span::styled(value_line.to_string(), value_style),
                        ]));
                    }
                }
                
                if i < entry.fields.len() - 1 {
                    lines.push(Line::from(""));
                }
            }
            
            lines.push(Line::from(""));
            
            // Navigation info
            let nav_info = format!("Entry {} of {} | Field {} of {}", 
                                 viewer_state.stream_entry_index + 1,
                                 entries.len(),
                                 viewer_state.stream_field_index + 1,
                                 entry.fields.len());
            lines.push(Line::from(vec![
                Span::styled(nav_info, Style::default().fg(Color::Gray)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("No entry selected", Style::default().fg(Color::Red)),
            ]));
        }
    }

    /// Add pagination information to the display
    fn add_pagination_info(
        lines: &mut Vec<Line<'static>>,
        viewer_state: &KeyViewerState,
        total_items: usize,
    ) {
        if total_items > viewer_state.page_size {
            lines.push(Line::from(""));
            
            let total_pages = (total_items + viewer_state.page_size - 1) / viewer_state.page_size;
            let current_page = viewer_state.current_page + 1;
            
            lines.push(Line::from(vec![
                Span::styled("Page ", Style::default().fg(Color::Gray)),
                Span::styled(current_page.to_string(), Style::default().fg(Color::Yellow)),
                Span::styled(" of ", Style::default().fg(Color::Gray)),
                Span::styled(total_pages.to_string(), Style::default().fg(Color::Yellow)),
                Span::styled(" | ", Style::default().fg(Color::Gray)),
                Span::styled("←/→: Navigate | PgUp/PgDn: Jump", Style::default().fg(Color::Gray)),
            ]));
        }
    }

}