use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::utils::export_import::ExportFormat;

/// Export/Import dialog state
#[derive(Debug, Clone)]
pub struct ExportImportDialog {
    pub is_open: bool,
    pub mode: ExportImportMode,
    pub selected_format: ExportFormat,
    pub file_path: String,
    pub path_cursor: usize,
    pub focused_field: ExportImportField,
    pub include_metadata: bool,
    pub include_ttl: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportImportMode {
    Export,
    Import,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportImportField {
    Format,
    FilePath,
    Options,
    Buttons,
}

impl Default for ExportImportDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            mode: ExportImportMode::Export,
            selected_format: ExportFormat::Json,
            file_path: String::new(),
            path_cursor: 0,
            focused_field: ExportImportField::Format,
            include_metadata: true,
            include_ttl: true,
        }
    }
}

impl ExportImportDialog {
    pub fn open_export(&mut self, default_path: String) {
        self.is_open = true;
        self.mode = ExportImportMode::Export;
        self.file_path = default_path;
        self.path_cursor = self.file_path.len();
        self.focused_field = ExportImportField::Format;
    }
    
    pub fn open_import(&mut self, default_path: String) {
        self.is_open = true;
        self.mode = ExportImportMode::Import;
        self.file_path = default_path;
        self.path_cursor = self.file_path.len();
        self.focused_field = ExportImportField::Format;
    }
    
    pub fn close(&mut self) {
        self.is_open = false;
        self.file_path.clear();
        self.path_cursor = 0;
    }
    
    pub fn next_field(&mut self) {
        self.focused_field = match self.focused_field {
            ExportImportField::Format => ExportImportField::FilePath,
            ExportImportField::FilePath => ExportImportField::Options,
            ExportImportField::Options => ExportImportField::Buttons,
            ExportImportField::Buttons => ExportImportField::Format,
        };
    }
    
    pub fn cycle_format(&mut self) {
        self.selected_format = match self.selected_format {
            ExportFormat::Json => ExportFormat::Yaml,
            ExportFormat::Yaml => ExportFormat::Csv,
            ExportFormat::Csv => ExportFormat::Raw,
            ExportFormat::Raw => ExportFormat::Redis,
            ExportFormat::Redis => ExportFormat::Json,
        };
    }
    
    pub fn add_path_char(&mut self, ch: char) {
        if self.focused_field == ExportImportField::FilePath {
            self.file_path.insert(self.path_cursor, ch);
            self.path_cursor += 1;
        }
    }
    
    pub fn backspace_path(&mut self) {
        if self.focused_field == ExportImportField::FilePath && self.path_cursor > 0 {
            self.path_cursor -= 1;
            self.file_path.remove(self.path_cursor);
        }
    }
    
    pub fn toggle_metadata(&mut self) {
        self.include_metadata = !self.include_metadata;
    }
    
    pub fn toggle_ttl(&mut self) {
        self.include_ttl = !self.include_ttl;
    }
    
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.is_open {
            return;
        }
        
        let dialog_width = 60.min(area.width.saturating_sub(4));
        let dialog_height = 15.min(area.height.saturating_sub(4));
        
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
        
        let title = match self.mode {
            ExportImportMode::Export => "Export Data",
            ExportImportMode::Import => "Import Data",
        };
        
        let dialog_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black).fg(Color::White))
            .border_style(Style::default().fg(Color::Cyan));
        
        frame.render_widget(dialog_block, dialog_area);
        
        let inner_area = dialog_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });
        
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Format
                Constraint::Length(3),  // File path
                Constraint::Length(3),  // Options
                Constraint::Length(3),  // Buttons
                Constraint::Min(0),     // Help
            ])
            .split(inner_area);
        
        // Format selector
        let format_style = if self.focused_field == ExportImportField::Format {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        let format_text = format!("Format: {} (Space to change)", self.selected_format);
        let format_para = Paragraph::new(format_text)
            .block(Block::default().borders(Borders::ALL).border_style(format_style))
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(format_para, layout[0]);
        
        // File path input
        let path_style = if self.focused_field == ExportImportField::FilePath {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        let path_text = if self.focused_field == ExportImportField::FilePath {
            let (before, after) = self.file_path.split_at(self.path_cursor);
            format!("{}|{}", before, after)
        } else {
            self.file_path.clone()
        };
        
        let path_para = Paragraph::new(path_text)
            .block(Block::default().title("File Path").borders(Borders::ALL).border_style(path_style))
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(path_para, layout[1]);
        
        // Options
        let options_style = if self.focused_field == ExportImportField::Options {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        let options_text = format!(
            "[{}] Metadata  [{}] TTL (m/t to toggle)",
            if self.include_metadata { "✓" } else { " " },
            if self.include_ttl { "✓" } else { " " }
        );
        
        let options_para = Paragraph::new(options_text)
            .block(Block::default().title("Options").borders(Borders::ALL).border_style(options_style))
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(options_para, layout[2]);
        
        // Buttons
        let button_style = if self.focused_field == ExportImportField::Buttons {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        let action_text = match self.mode {
            ExportImportMode::Export => "Export",
            ExportImportMode::Import => "Import",
        };
        
        let buttons_text = format!("Cancel | {} (Enter to execute)", action_text);
        let buttons_para = Paragraph::new(buttons_text)
            .block(Block::default().borders(Borders::ALL).border_style(button_style))
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
        
        frame.render_widget(buttons_para, layout[3]);
        
        // Help text
        let help_text = "Tab: Next field | Space: Toggle | Enter: Execute | Esc: Cancel";
        let help_para = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        
        frame.render_widget(help_para, layout[4]);
    }
}