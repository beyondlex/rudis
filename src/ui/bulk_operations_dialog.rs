use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::utils::bulk_operations::{BulkOperation, BulkProgress};

/// Bulk operations dialog state
#[derive(Debug, Clone)]
pub struct BulkOperationsDialog {
    /// Whether the dialog is open
    pub is_open: bool,
    /// Selected keys for bulk operation
    pub selected_keys: Vec<String>,
    /// Available operations
    pub available_operations: Vec<BulkOperation>,
    /// Currently selected operation index
    pub operation_index: usize,
    /// Currently focused field
    pub focused_field: BulkDialogField,
    /// Operation parameters
    pub operation_params: BulkOperationParams,
    /// Progress information during execution
    pub progress: Option<BulkProgress>,
    /// Whether operation is currently running
    pub is_running: bool,
}

/// Fields in the bulk operations dialog
#[derive(Debug, Clone, PartialEq)]
pub enum BulkDialogField {
    KeySelection,
    Operation,
    Parameters,
    Buttons,
}

/// Parameters for different bulk operations
#[derive(Debug, Clone)]
pub struct BulkOperationParams {
    /// TTL value for SetTtl operation
    pub ttl_value: String,
    /// Target database for Copy operation
    pub target_db: String,
    /// Pattern for Rename operation
    pub rename_pattern: String,
    /// Replacement for Rename operation
    pub rename_replacement: String,
    /// Value for SetValue operation
    pub set_value: String,
    /// Amount for Increment operation
    pub increment_amount: String,
    /// Suffix for AppendString operation
    pub append_suffix: String,
    /// Currently editing field
    pub editing_field: ParameterField,
}

/// Parameter fields for editing
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterField {
    None,
    TtlValue,
    TargetDb,
    RenamePattern,
    RenameReplacement,
    SetValue,
    IncrementAmount,
    AppendSuffix,
}

impl Default for BulkOperationsDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            selected_keys: Vec::new(),
            available_operations: vec![
                BulkOperation::Delete,
                BulkOperation::SetTtl(3600),
                BulkOperation::RemoveTtl,
                BulkOperation::Copy { target_db: 1 },
                BulkOperation::Rename { 
                    pattern: "old_".to_string(), 
                    replacement: "new_".to_string() 
                },
                BulkOperation::SetValue { value: "".to_string() },
                BulkOperation::Increment { amount: 1 },
                BulkOperation::AppendString { suffix: "".to_string() },
            ],
            operation_index: 0,
            focused_field: BulkDialogField::KeySelection,
            operation_params: BulkOperationParams::default(),
            progress: None,
            is_running: false,
        }
    }
}

impl Default for BulkOperationParams {
    fn default() -> Self {
        Self {
            ttl_value: "3600".to_string(),
            target_db: "1".to_string(),
            rename_pattern: "old_".to_string(),
            rename_replacement: "new_".to_string(),
            set_value: String::new(),
            increment_amount: "1".to_string(),
            append_suffix: String::new(),
            editing_field: ParameterField::None,
        }
    }
}

impl BulkOperationsDialog {
    /// Open the bulk operations dialog with selected keys
    pub fn open(&mut self, selected_keys: Vec<String>) {
        self.is_open = true;
        self.selected_keys = selected_keys;
        self.operation_index = 0;
        self.focused_field = BulkDialogField::KeySelection;
        self.progress = None;
        self.is_running = false;
    }
    
    /// Close the dialog
    pub fn close(&mut self) {
        self.is_open = false;
        self.selected_keys.clear();
        self.progress = None;
        self.is_running = false;
    }
    
    /// Move to next field
    pub fn next_field(&mut self) {
        if self.is_running {
            return;
        }
        
        self.focused_field = match self.focused_field {
            BulkDialogField::KeySelection => BulkDialogField::Operation,
            BulkDialogField::Operation => BulkDialogField::Parameters,
            BulkDialogField::Parameters => BulkDialogField::Buttons,
            BulkDialogField::Buttons => BulkDialogField::KeySelection,
        };
    }
    
    /// Select next operation
    pub fn next_operation(&mut self) {
        if self.is_running {
            return;
        }
        
        self.operation_index = (self.operation_index + 1) % self.available_operations.len();
    }
    
    /// Select previous operation
    pub fn prev_operation(&mut self) {
        if self.is_running {
            return;
        }
        
        self.operation_index = if self.operation_index == 0 {
            self.available_operations.len() - 1
        } else {
            self.operation_index - 1
        };
    }
    
    /// Get current operation
    pub fn get_current_operation(&self) -> Option<BulkOperation> {
        self.available_operations.get(self.operation_index).cloned()
    }
    
    /// Start operation execution
    pub fn start_execution(&mut self) {
        if !self.selected_keys.is_empty() && !self.is_running {
            self.is_running = true;
            self.progress = Some(BulkProgress::new(self.selected_keys.len()));
        }
    }
    
    /// Update execution progress
    pub fn update_progress(&mut self, progress: BulkProgress) {
        self.progress = Some(progress.clone());
        if progress.is_complete {
            self.is_running = false;
        }
    }
    
    /// Add character to parameter field
    pub fn add_param_char(&mut self, ch: char) {
        match self.operation_params.editing_field {
            ParameterField::TtlValue => self.operation_params.ttl_value.push(ch),
            ParameterField::TargetDb => self.operation_params.target_db.push(ch),
            ParameterField::RenamePattern => self.operation_params.rename_pattern.push(ch),
            ParameterField::RenameReplacement => self.operation_params.rename_replacement.push(ch),
            ParameterField::SetValue => self.operation_params.set_value.push(ch),
            ParameterField::IncrementAmount => self.operation_params.increment_amount.push(ch),
            ParameterField::AppendSuffix => self.operation_params.append_suffix.push(ch),
            ParameterField::None => {}
        }
    }
    
    /// Remove character from parameter field
    pub fn backspace_param(&mut self) {
        match self.operation_params.editing_field {
            ParameterField::TtlValue => { self.operation_params.ttl_value.pop(); }
            ParameterField::TargetDb => { self.operation_params.target_db.pop(); }
            ParameterField::RenamePattern => { self.operation_params.rename_pattern.pop(); }
            ParameterField::RenameReplacement => { self.operation_params.rename_replacement.pop(); }
            ParameterField::SetValue => { self.operation_params.set_value.pop(); }
            ParameterField::IncrementAmount => { self.operation_params.increment_amount.pop(); }
            ParameterField::AppendSuffix => { self.operation_params.append_suffix.pop(); }
            ParameterField::None => {}
        }
    }
    
    /// Render the bulk operations dialog
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.is_open {
            return;
        }
        
        let dialog_width = 80.min(area.width.saturating_sub(4));
        let dialog_height = 25.min(area.height.saturating_sub(4));
        
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
        frame.render_widget(Clear, dialog_area);
        
        let dialog_block = Block::default()
            .title("Bulk Operations")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black).fg(Color::White))
            .border_style(Style::default().fg(Color::Cyan));
        
        frame.render_widget(dialog_block, dialog_area);
        
        let inner_area = dialog_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });
        
        if self.is_running {
            self.render_progress_view(frame, inner_area);
        } else {
            self.render_setup_view(frame, inner_area);
        }
    }
    
    /// Render the operation setup view
    fn render_setup_view(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),  // Key selection
                Constraint::Length(6),  // Operation selection
                Constraint::Length(6),  // Parameters
                Constraint::Length(3),  // Buttons
                Constraint::Min(0),     // Help
            ])
            .split(area);
        
        // Key selection
        self.render_key_selection(frame, layout[0]);
        
        // Operation selection
        self.render_operation_selection(frame, layout[1]);
        
        // Parameters
        self.render_parameters(frame, layout[2]);
        
        // Buttons
        self.render_buttons(frame, layout[3]);
        
        // Help text
        self.render_help(frame, layout[4]);
    }
    
    /// Render the progress view during execution
    fn render_progress_view(&self, frame: &mut Frame, area: Rect) {
        if let Some(ref progress) = self.progress {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Progress bar
                    Constraint::Length(3),  // Current operation
                    Constraint::Length(3),  // Statistics
                    Constraint::Min(0),     // Errors
                ])
                .split(area);
            
            // Progress bar
            let progress_ratio = progress.progress_percentage() / 100.0;
            let progress_bar = Gauge::default()
                .block(Block::default().title("Progress").borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Green))
                .percent((progress.progress_percentage() as u16).min(100))
                .label(format!("{:.1}%", progress.progress_percentage()));
            
            frame.render_widget(progress_bar, layout[0]);
            
            // Current operation
            let current_op = Paragraph::new(format!("Current: {}", progress.current_operation))
                .block(Block::default().title("Status").borders(Borders::ALL))
                .style(Style::default().fg(Color::White));
            
            frame.render_widget(current_op, layout[1]);
            
            // Statistics
            let stats_text = format!(
                "Completed: {} | Successful: {} | Failed: {} | Total: {}",
                progress.completed, progress.successful, progress.failed, progress.total
            );
            
            let stats = Paragraph::new(stats_text)
                .block(Block::default().title("Statistics").borders(Borders::ALL))
                .style(Style::default().fg(Color::White));
            
            frame.render_widget(stats, layout[2]);
            
            // Errors (if any)
            if !progress.errors.is_empty() {
                let error_text = progress.errors.join("\n");
                let errors = Paragraph::new(error_text)
                    .block(Block::default().title("Errors").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Red))
                    .wrap(Wrap { trim: true });
                
                frame.render_widget(errors, layout[3]);
            }
        }
    }
    
    /// Render key selection section
    fn render_key_selection(&self, frame: &mut Frame, area: Rect) {
        let key_count = self.selected_keys.len();
        let key_text = if key_count <= 5 {
            self.selected_keys.join(", ")
        } else {
            format!("{} keys selected: {}, ... (+{} more)", 
                   key_count, 
                   self.selected_keys[..2].join(", "),
                   key_count - 2)
        };
        
        let key_style = if self.focused_field == BulkDialogField::KeySelection {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        let key_selection = Paragraph::new(key_text)
            .block(Block::default()
                .title(format!("Selected Keys ({})", key_count))
                .borders(Borders::ALL)
                .border_style(key_style))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        
        frame.render_widget(key_selection, area);
    }
    
    /// Render operation selection section
    fn render_operation_selection(&self, frame: &mut Frame, area: Rect) {
        let op_items: Vec<ListItem> = self.available_operations.iter()
            .enumerate()
            .map(|(i, op)| {
                let style = if i == self.operation_index {
                    Style::default().bg(Color::DarkGray).fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };
                
                let marker = if i == self.operation_index { ">> " } else { "   " };
                let description = crate::utils::BulkOperationsManager::get_operation_description(op);
                ListItem::new(format!("{}{}", marker, description)).style(style)
            })
            .collect();
        
        let op_style = if self.focused_field == BulkDialogField::Operation {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        let operation_list = List::new(op_items)
            .block(Block::default()
                .title("Operation")
                .borders(Borders::ALL)
                .border_style(op_style))
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(operation_list, area);
    }
    
    /// Render parameters section
    fn render_parameters(&self, frame: &mut Frame, area: Rect) {
        let param_text = match self.get_current_operation() {
            Some(BulkOperation::SetTtl(_)) => {
                format!("TTL (seconds): {}", self.operation_params.ttl_value)
            }
            Some(BulkOperation::Copy { .. }) => {
                format!("Target Database: {}", self.operation_params.target_db)
            }
            Some(BulkOperation::Rename { .. }) => {
                format!("Pattern: '{}' → Replacement: '{}'", 
                       self.operation_params.rename_pattern,
                       self.operation_params.rename_replacement)
            }
            Some(BulkOperation::SetValue { .. }) => {
                format!("Value: {}", self.operation_params.set_value)
            }
            Some(BulkOperation::Increment { .. }) => {
                format!("Amount: {}", self.operation_params.increment_amount)
            }
            Some(BulkOperation::AppendString { .. }) => {
                format!("Suffix: {}", self.operation_params.append_suffix)
            }
            _ => "No parameters required".to_string(),
        };
        
        let param_style = if self.focused_field == BulkDialogField::Parameters {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        let parameters = Paragraph::new(param_text)
            .block(Block::default()
                .title("Parameters")
                .borders(Borders::ALL)
                .border_style(param_style))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        
        frame.render_widget(parameters, area);
    }
    
    /// Render buttons section
    fn render_buttons(&self, frame: &mut Frame, area: Rect) {
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(area);
        
        let button_style = if self.focused_field == BulkDialogField::Buttons {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        // Cancel button
        let cancel_button = Paragraph::new("Cancel")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_style(button_style));
        
        frame.render_widget(cancel_button, button_layout[0]);
        
        // Preview button
        let preview_button = Paragraph::new("Preview")
            .style(Style::default().fg(Color::Blue))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_style(button_style));
        
        frame.render_widget(preview_button, button_layout[1]);
        
        // Execute button
        let execute_button = Paragraph::new("Execute")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_style(button_style));
        
        frame.render_widget(execute_button, button_layout[2]);
    }
    
    /// Render help text
    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = match self.focused_field {
            BulkDialogField::KeySelection => "Selected keys for bulk operation",
            BulkDialogField::Operation => "↑/↓: Select operation | Tab: Next field",
            BulkDialogField::Parameters => "Type values | Tab: Next field",
            BulkDialogField::Buttons => "←/→: Select button | Enter: Execute | Tab: Next field",
        };
        
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        
        frame.render_widget(help_paragraph, area);
    }
}