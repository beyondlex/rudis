use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, Instant};

/// Progress bar types for different operations
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressType {
    /// Generic progress with percentage
    Generic,
    /// File transfer with bytes/sec
    Transfer,
    /// Bulk operations with success/failure counts
    BulkOperation,
    /// Data loading with item counts
    DataLoading,
    /// Network operation with timeout
    Network,
}

/// Progress bar state and configuration
#[derive(Debug, Clone)]
pub struct ProgressBar {
    /// Whether the progress bar is visible
    pub is_visible: bool,
    /// Type of progress operation
    pub progress_type: ProgressType,
    /// Current progress (0.0 to 1.0)
    pub progress: f64,
    /// Primary title/operation name
    pub title: String,
    /// Current status message
    pub status: String,
    /// Current operation description
    pub current_operation: String,
    /// Total items/bytes/operations
    pub total: u64,
    /// Completed items/bytes/operations
    pub completed: u64,
    /// Number of successful operations (bulk operations)
    pub successful: u64,
    /// Number of failed operations (bulk operations)
    pub failed: u64,
    /// Start time for rate calculation
    pub start_time: Instant,
    /// Whether the operation can be cancelled
    pub can_cancel: bool,
    /// Error messages
    pub errors: Vec<String>,
    /// Whether operation is paused
    pub is_paused: bool,
    /// Estimated time remaining
    pub eta: Option<Duration>,
    /// Additional metadata
    pub metadata: ProgressMetadata,
}

/// Additional metadata for progress tracking
#[derive(Debug, Clone, Default)]
pub struct ProgressMetadata {
    /// Transfer rate (bytes per second)
    pub transfer_rate: Option<f64>,
    /// Items processed per second
    pub items_per_second: Option<f64>,
    /// Memory usage during operation
    pub memory_usage: Option<u64>,
    /// Custom attributes
    pub custom_fields: Vec<(String, String)>,
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self {
            is_visible: false,
            progress_type: ProgressType::Generic,
            progress: 0.0,
            title: "Processing...".to_string(),
            status: "Starting...".to_string(),
            current_operation: String::new(),
            total: 0,
            completed: 0,
            successful: 0,
            failed: 0,
            start_time: Instant::now(),
            can_cancel: true,
            errors: Vec::new(),
            is_paused: false,
            eta: None,
            metadata: ProgressMetadata::default(),
        }
    }
}

impl ProgressBar {
    /// Create a new progress bar for a specific operation type
    pub fn new(progress_type: ProgressType, title: String, total: u64, can_cancel: bool) -> Self {
        Self {
            is_visible: true,
            progress_type,
            title,
            total,
            can_cancel,
            start_time: Instant::now(),
            ..Default::default()
        }
    }
    
    /// Show the progress bar
    pub fn show(&mut self) {
        self.is_visible = true;
        self.start_time = Instant::now();
    }
    
    /// Hide the progress bar
    pub fn hide(&mut self) {
        self.is_visible = false;
    }
    
    /// Update progress with completed count
    pub fn update(&mut self, completed: u64, status: String) {
        self.completed = completed;
        self.status = status;
        self.progress = if self.total > 0 {
            (completed as f64 / self.total as f64).min(1.0)
        } else {
            0.0
        };
        
        // Calculate rates and ETA
        self.calculate_rates();
    }
    
    /// Update progress for bulk operations
    pub fn update_bulk(&mut self, completed: u64, successful: u64, failed: u64, current_op: String) {
        self.completed = completed;
        self.successful = successful;
        self.failed = failed;
        self.current_operation = current_op;
        self.progress = if self.total > 0 {
            (completed as f64 / self.total as f64).min(1.0)
        } else {
            0.0
        };
        
        self.status = format!("Processed: {} | Success: {} | Failed: {}", completed, successful, failed);
        self.calculate_rates();
    }
    
    /// Set error messages
    pub fn set_errors(&mut self, errors: Vec<String>) {
        self.errors = errors;
    }
    
    /// Add a single error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
    
    /// Pause the operation
    pub fn pause(&mut self) {
        self.is_paused = true;
    }
    
    /// Resume the operation
    pub fn resume(&mut self) {
        self.is_paused = false;
    }
    
    /// Mark operation as complete
    pub fn complete(&mut self, final_status: String) {
        self.progress = 1.0;
        self.status = final_status;
        self.completed = self.total;
    }
    
    /// Calculate transfer rates and ETA
    fn calculate_rates(&mut self) {
        let elapsed = self.start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        
        if elapsed_secs > 0.0 {
            // Calculate items per second
            self.metadata.items_per_second = Some(self.completed as f64 / elapsed_secs);
            
            // Calculate ETA
            if self.progress > 0.0 && self.progress < 1.0 {
                let remaining_ratio = (1.0 - self.progress) / self.progress;
                let estimated_remaining = elapsed_secs * remaining_ratio;
                self.eta = Some(Duration::from_secs_f64(estimated_remaining));
            }
        }
    }
    
    /// Get formatted progress percentage
    pub fn get_percentage(&self) -> String {
        format!("{:.1}%", self.progress * 100.0)
    }
    
    /// Get formatted transfer rate
    pub fn get_transfer_rate(&self) -> String {
        match self.metadata.transfer_rate {
            Some(rate) => {
                if rate >= 1_000_000.0 {
                    format!("{:.1} MB/s", rate / 1_000_000.0)
                } else if rate >= 1_000.0 {
                    format!("{:.1} KB/s", rate / 1_000.0)
                } else {
                    format!("{:.0} B/s", rate)
                }
            }
            None => "-- B/s".to_string(),
        }
    }
    
    /// Get formatted items per second
    pub fn get_items_rate(&self) -> String {
        match self.metadata.items_per_second {
            Some(rate) => {
                if rate >= 1000.0 {
                    format!("{:.1}k items/s", rate / 1000.0)
                } else {
                    format!("{:.1} items/s", rate)
                }
            }
            None => "-- items/s".to_string(),
        }
    }
    
    /// Get formatted ETA
    pub fn get_eta(&self) -> String {
        match self.eta {
            Some(eta) => {
                let total_secs = eta.as_secs();
                let hours = total_secs / 3600;
                let minutes = (total_secs % 3600) / 60;
                let seconds = total_secs % 60;
                
                if hours > 0 {
                    format!("{}h{}m{}s", hours, minutes, seconds)
                } else if minutes > 0 {
                    format!("{}m{}s", minutes, seconds)
                } else {
                    format!("{}s", seconds)
                }
            }
            None => "--".to_string(),
        }
    }
    
    /// Render the progress bar
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.is_visible {
            return;
        }
        
        // Create overlay
        frame.render_widget(Clear, area);
        
        // Calculate dialog size
        let dialog_width = 80.min(area.width.saturating_sub(4));
        let dialog_height = match self.progress_type {
            ProgressType::BulkOperation => 18,
            ProgressType::Transfer => 15,
            _ => 12,
        }.min(area.height.saturating_sub(4));
        
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
        
        // Main dialog block
        let dialog_block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black).fg(Color::White))
            .border_style(Style::default().fg(Color::Cyan));
        
        frame.render_widget(dialog_block, dialog_area);
        
        // Inner area for content
        let inner_area = dialog_area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 2 });
        
        // Layout based on progress type
        match self.progress_type {
            ProgressType::BulkOperation => self.render_bulk_progress(frame, inner_area),
            ProgressType::Transfer => self.render_transfer_progress(frame, inner_area),
            _ => self.render_generic_progress(frame, inner_area),
        }
    }
    
    /// Render generic progress layout
    fn render_generic_progress(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Length(2), // Status
                Constraint::Length(2), // Stats
                Constraint::Min(1),    // Help/Cancel
            ])
            .split(area);
        
        // Progress bar
        let progress_bar = Gauge::default()
            .block(Block::default().title("Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green))
            .percent((self.progress * 100.0) as u16)
            .label(format!("{} | {}/{}", self.get_percentage(), self.completed, self.total));
        
        frame.render_widget(progress_bar, layout[0]);
        
        // Status
        let status = Paragraph::new(self.status.clone())
            .block(Block::default().title("Status").borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(status, layout[1]);
        
        // Stats
        let stats_text = format!("Rate: {} | ETA: {}", self.get_items_rate(), self.get_eta());
        let stats = Paragraph::new(stats_text)
            .block(Block::default().title("Statistics").borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        
        frame.render_widget(stats, layout[2]);
        
        // Help text
        let help_text = if self.can_cancel {
            "Press Esc to dismiss"
        } else {
            "Operation in progress..."
        };
        
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        
        frame.render_widget(help, layout[3]);
    }
    
    /// Render transfer progress layout
    fn render_transfer_progress(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Length(2), // Status
                Constraint::Length(2), // Transfer stats
                Constraint::Length(2), // Time stats
                Constraint::Min(1),    // Help/Cancel
            ])
            .split(area);
        
        // Progress bar
        let progress_bar = Gauge::default()
            .block(Block::default().title("Transfer Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Blue))
            .percent((self.progress * 100.0) as u16)
            .label(format!("{} | {} bytes", self.get_percentage(), self.completed));
        
        frame.render_widget(progress_bar, layout[0]);
        
        // Status
        let status = Paragraph::new(self.status.clone())
            .block(Block::default().title("Current File").borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(status, layout[1]);
        
        // Transfer stats
        let transfer_text = format!("Speed: {} | Total: {} bytes", self.get_transfer_rate(), self.total);
        let transfer_stats = Paragraph::new(transfer_text)
            .block(Block::default().title("Transfer").borders(Borders::ALL))
            .style(Style::default().fg(Color::Green));
        
        frame.render_widget(transfer_stats, layout[2]);
        
        // Time stats
        let elapsed = self.start_time.elapsed();
        let time_text = format!("Elapsed: {:.1}s | ETA: {}", elapsed.as_secs_f64(), self.get_eta());
        let time_stats = Paragraph::new(time_text)
            .block(Block::default().title("Time").borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));
        
        frame.render_widget(time_stats, layout[3]);
        
        // Help text
        let help_text = if self.can_cancel {
            "Press Esc to dismiss"
        } else {
            "Transfer in progress..."
        };
        
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        
        frame.render_widget(help, layout[4]);
    }
    
    /// Render bulk operation progress layout
    fn render_bulk_progress(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Length(2), // Current operation
                Constraint::Length(2), // Statistics
                Constraint::Length(2), // Rates
                Constraint::Length(4), // Errors (if any)
                Constraint::Min(1),    // Help/Cancel
            ])
            .split(area);
        
        // Progress bar
        let progress_color = if self.failed > 0 {
            Color::Yellow
        } else {
            Color::Green
        };
        
        let progress_bar = Gauge::default()
            .block(Block::default().title("Bulk Operation Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(progress_color))
            .percent((self.progress * 100.0) as u16)
            .label(format!("{} | {}/{}", self.get_percentage(), self.completed, self.total));
        
        frame.render_widget(progress_bar, layout[0]);
        
        // Current operation
        let current_op = Paragraph::new(self.current_operation.clone())
            .block(Block::default().title("Current Operation").borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(current_op, layout[1]);
        
        // Statistics
        let stats_text = format!(
            "Completed: {} | Successful: {} | Failed: {}",
            self.completed, self.successful, self.failed
        );
        let stats = Paragraph::new(stats_text)
            .block(Block::default().title("Results").borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(stats, layout[2]);
        
        // Rates
        let rates_text = format!("Rate: {} | ETA: {}", self.get_items_rate(), self.get_eta());
        let rates = Paragraph::new(rates_text)
            .block(Block::default().title("Performance").borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        
        frame.render_widget(rates, layout[3]);
        
        // Errors
        if !self.errors.is_empty() {
            let error_text = self.errors.join("\n");
            let errors = Paragraph::new(error_text)
                .block(Block::default().title("Recent Errors").borders(Borders::ALL))
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true });
            
            frame.render_widget(errors, layout[4]);
        }
        
        // Help text
        let help_text = if self.can_cancel {
            "Press Esc to dismiss"
        } else {
            "Bulk operation in progress..."
        };
        
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        
        frame.render_widget(help, layout[5]);
    }
}

/// Progress bar manager for handling multiple progress bars
#[derive(Debug, Default)]
pub struct ProgressBarManager {
    /// Currently active progress bars
    pub progress_bars: Vec<ProgressBar>,
    /// Currently visible progress bar index
    pub active_index: Option<usize>,
}

impl ProgressBarManager {
    /// Add a new progress bar
    pub fn add_progress_bar(&mut self, progress_bar: ProgressBar) -> usize {
        self.progress_bars.push(progress_bar);
        let index = self.progress_bars.len() - 1;
        self.active_index = Some(index);
        index
    }
    
    /// Remove a progress bar
    pub fn remove_progress_bar(&mut self, index: usize) {
        if index < self.progress_bars.len() {
            self.progress_bars.remove(index);
            
            // Update active index
            if self.progress_bars.is_empty() {
                self.active_index = None;
            } else if Some(index) == self.active_index {
                self.active_index = Some(0);
            } else if let Some(active) = self.active_index {
                if active > index {
                    self.active_index = Some(active - 1);
                }
            }
        }
    }
    
    /// Get the currently active progress bar
    pub fn get_active(&self) -> Option<&ProgressBar> {
        self.active_index
            .and_then(|index| self.progress_bars.get(index))
    }
    
    /// Get mutable reference to the currently active progress bar
    pub fn get_active_mut(&mut self) -> Option<&mut ProgressBar> {
        self.active_index
            .and_then(|index| self.progress_bars.get_mut(index))
    }
    
    /// Hide all progress bars
    pub fn hide_all(&mut self) {
        for progress_bar in &mut self.progress_bars {
            progress_bar.hide();
        }
    }
    
    /// Render the active progress bar
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if let Some(progress_bar) = self.get_active() {
            progress_bar.render(frame, area);
        }
    }
}