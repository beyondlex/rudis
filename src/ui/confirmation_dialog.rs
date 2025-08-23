use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    prelude::Stylize,
    Frame,
};

/// Types of confirmation dialogs
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmationType {
    /// Confirm saving changes to a key value
    SaveChanges {
        key_name: String,
        old_value_summary: String,
        new_value_summary: String,
    },
    /// Confirm deleting a key
    DeleteKey {
        key_name: String,
        key_type: String,
    },
    /// Confirm discarding unsaved changes
    DiscardChanges {
        key_name: String,
    },
    /// Confirm large value edit (size warning)
    LargeValueEdit {
        key_name: String,
        size: usize,
    },
    /// Confirm binary data edit
    BinaryDataEdit {
        key_name: String,
        binary_info: String,
    },
    /// Confirm data type conversion
    TypeConversion {
        key_name: String,
        from_type: String,
        to_type: String,
    },
    /// Confirm bulk operation
    BulkOperation {
        operation: String,
        count: usize,
    },
}

/// Response from confirmation dialog
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmationResponse {
    Confirmed,
    Cancelled,
    Pending,
}

/// Confirmation dialog state
#[derive(Debug, Clone)]
pub struct ConfirmationDialog {
    /// Whether the dialog is open
    pub is_open: bool,
    /// Type of confirmation needed
    pub confirmation_type: Option<ConfirmationType>,
    /// Currently selected button (0 = Cancel, 1 = Confirm)
    pub selected_button: usize,
    /// Custom title for the dialog
    pub custom_title: Option<String>,
    /// Custom message for the dialog
    pub custom_message: Option<String>,
    /// Additional details to show
    pub details: Vec<String>,
    /// Whether to show details expanded
    pub show_details: bool,
}

impl Default for ConfirmationDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            confirmation_type: None,
            selected_button: 1, // Default to "Confirm" button
            custom_title: None,
            custom_message: None,
            details: Vec::new(),
            show_details: false,
        }
    }
}

impl ConfirmationDialog {
    /// Open a confirmation dialog
    pub fn open(&mut self, confirmation_type: ConfirmationType) {
        self.is_open = true;
        self.confirmation_type = Some(confirmation_type);
        self.selected_button = 1; // Default to confirm
        self.custom_title = None;
        self.custom_message = None;
        self.details.clear();
        self.show_details = false;
    }
    
    /// Open a custom confirmation dialog
    pub fn open_custom(&mut self, title: String, message: String, details: Vec<String>) {
        self.is_open = true;
        self.confirmation_type = None;
        self.custom_title = Some(title);
        self.custom_message = Some(message);
        self.show_details = !details.is_empty();
        self.details = details;
        self.selected_button = 1;
    }
    
    /// Close the dialog
    pub fn close(&mut self) {
        self.is_open = false;
        self.confirmation_type = None;
        self.selected_button = 1;
        self.custom_title = None;
        self.custom_message = None;
        self.details.clear();
        self.show_details = false;
    }
    
    /// Move to next button
    pub fn next_button(&mut self) {
        self.selected_button = (self.selected_button + 1) % 2;
    }
    
    /// Move to previous button
    pub fn prev_button(&mut self) {
        self.selected_button = if self.selected_button == 0 { 1 } else { 0 };
    }
    
    /// Toggle details display
    pub fn toggle_details(&mut self) {
        if !self.details.is_empty() {
            self.show_details = !self.show_details;
        }
    }
    
    /// Get the current response based on selected button
    pub fn get_response(&self) -> ConfirmationResponse {
        if !self.is_open {
            ConfirmationResponse::Pending
        } else if self.selected_button == 0 {
            ConfirmationResponse::Cancelled
        } else {
            ConfirmationResponse::Confirmed
        }
    }
    
    /// Get dialog title
    pub fn get_title(&self) -> String {
        if let Some(title) = &self.custom_title {
            title.clone()
        } else if let Some(ref confirmation_type) = self.confirmation_type {
            match confirmation_type {
                ConfirmationType::SaveChanges { .. } => "Confirm Save Changes".to_string(),
                ConfirmationType::DeleteKey { .. } => "Confirm Delete Key".to_string(),
                ConfirmationType::DiscardChanges { .. } => "Discard Changes".to_string(),
                ConfirmationType::LargeValueEdit { .. } => "Large Value Warning".to_string(),
                ConfirmationType::BinaryDataEdit { .. } => "Binary Data Warning".to_string(),
                ConfirmationType::TypeConversion { .. } => "Confirm Type Conversion".to_string(),
                ConfirmationType::BulkOperation { .. } => "Confirm Bulk Operation".to_string(),
            }
        } else {
            "Confirmation".to_string()
        }
    }
    
    /// Get dialog message
    pub fn get_message(&self) -> String {
        if let Some(message) = &self.custom_message {
            message.clone()
        } else if let Some(ref confirmation_type) = self.confirmation_type {
            match confirmation_type {
                ConfirmationType::SaveChanges { key_name, .. } => {
                    format!("Do you want to save changes to key '{}'?", key_name)
                }
                ConfirmationType::DeleteKey { key_name, key_type } => {
                    format!("Are you sure you want to delete {} key '{}'?\n\nThis action cannot be undone.", key_type, key_name)
                }
                ConfirmationType::DiscardChanges { key_name } => {
                    format!("You have unsaved changes to key '{}'.\n\nDo you want to discard these changes?", key_name)
                }
                ConfirmationType::LargeValueEdit { key_name, size } => {
                    format!("Key '{}' contains a large value ({}).\n\nEditing large values may impact performance.\nDo you want to continue?", 
                           key_name, format_size(*size))
                }
                ConfirmationType::BinaryDataEdit { key_name, binary_info } => {
                    format!("Key '{}' contains binary data ({}).\n\nEditing binary data as text may cause data corruption.\nDo you want to continue?", 
                           key_name, binary_info)
                }
                ConfirmationType::TypeConversion { key_name, from_type, to_type } => {
                    format!("Convert key '{}' from {} to {}?\n\nThis will change the data structure and may result in data loss.", 
                           key_name, from_type, to_type)
                }
                ConfirmationType::BulkOperation { operation, count } => {
                    format!("Perform '{}' on {} items?\n\nThis operation will affect multiple keys.", operation, count)
                }
            }
        } else {
            "Are you sure you want to proceed?".to_string()
        }
    }
    
    /// Get additional details for the confirmation
    pub fn get_details(&self) -> Vec<String> {
        if !self.details.is_empty() {
            return self.details.clone();
        }
        
        if let Some(ref confirmation_type) = self.confirmation_type {
            match confirmation_type {
                ConfirmationType::SaveChanges { old_value_summary, new_value_summary, .. } => {
                    vec![
                        "Changes:".to_string(),
                        format!("Old: {}", old_value_summary),
                        format!("New: {}", new_value_summary),
                    ]
                }
                ConfirmationType::LargeValueEdit { size, .. } => {
                    vec![
                        "Value Information:".to_string(),
                        format!("Size: {}", format_size(*size)),
                        "Consider using binary view mode for large data.".to_string(),
                    ]
                }
                ConfirmationType::BinaryDataEdit { binary_info, .. } => {
                    vec![
                        "Binary Data Detected:".to_string(),
                        binary_info.clone(),
                        "Consider using hex editor mode instead.".to_string(),
                    ]
                }
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }
    
    /// Render the confirmation dialog
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.is_open {
            return;
        }
        
        // Calculate dialog size
        let dialog_width = 60.min(area.width.saturating_sub(4));
        let dialog_height = 15.min(area.height.saturating_sub(4));
        
        // Center the dialog
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((area.width.saturating_sub(dialog_width)) / 2),
                Constraint::Length(dialog_width),
                Constraint::Min(0),
            ])
            .split(area);
        
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((area.height.saturating_sub(dialog_height)) / 2),
                Constraint::Length(dialog_height),
                Constraint::Min(0),
            ])
            .split(horizontal[1]);
        
        let dialog_area = vertical[1];
        
        // Clear the area
        frame.render_widget(Clear, dialog_area);
        
        // Dialog border
        let dialog_block = Block::default()
            .title(self.get_title())
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black).fg(Color::White))
            .border_style(Style::default().fg(Color::Yellow));
        
        frame.render_widget(dialog_block, dialog_area);
        
        // Inner area for content
        let inner_area = dialog_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });
        
        // Layout: message, details (optional), buttons
        let mut constraints = vec![Constraint::Min(3)]; // Message area
        
        if self.show_details && !self.get_details().is_empty() {
            constraints.push(Constraint::Length(5)); // Details area
        }
        
        constraints.push(Constraint::Length(3)); // Button area
        
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);
        
        // Render message
        let message = self.get_message();
        let message_paragraph = Paragraph::new(message)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        frame.render_widget(message_paragraph, layout[0]);
        
        let mut button_area_idx = 1;
        
        // Render details if shown
        if self.show_details && !self.get_details().is_empty() {
            let details = self.get_details();
            let details_text = details.join("\n");
            
            let details_block = Block::default()
                .title("Details")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Gray))
                .border_style(Style::default().fg(Color::Blue));
            
            let details_paragraph = Paragraph::new(details_text)
                .block(details_block)
                .style(Style::default().fg(Color::Cyan))
                .wrap(Wrap { trim: true });
                
            frame.render_widget(details_paragraph, layout[1]);
            button_area_idx = 2;
        }
        
        // Render buttons
        self.render_buttons(frame, layout[button_area_idx]);
        
        // Show help text
        let help_text = if !self.get_details().is_empty() {
            "←/→: Select | Enter: Confirm | Esc: Cancel | d: Toggle Details"
        } else {
            "←/→: Select | Enter: Confirm | Esc: Cancel"
        };
        
        if let Some(help_area) = layout.get(button_area_idx + 1) {
            let help_paragraph = Paragraph::new(help_text)
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            frame.render_widget(help_paragraph, *help_area);
        }
    }
    
    /// Render dialog buttons
    fn render_buttons(&self, frame: &mut Frame, area: Rect) {
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);
        
        // Cancel button
        let cancel_style = if self.selected_button == 0 {
            Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };
        
        let cancel_button = Paragraph::new("Cancel")
            .style(cancel_style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(cancel_button, button_layout[0]);
        
        // Confirm button
        let confirm_style = if self.selected_button == 1 {
            Style::default().bg(Color::Green).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        
        let confirm_button = Paragraph::new("Confirm")
            .style(confirm_style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(confirm_button, button_layout[1]);
    }
}

/// Format byte size in human-readable format
fn format_size(size: usize) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}