use super::state_core::AppState;

impl AppState {
    /// Start a progress operation
    pub fn start_progress(
        &mut self, 
        progress_type: crate::ui::progress_bar::ProgressType, 
        title: String, 
        total: u64,
        can_cancel: bool
    ) -> usize {
        let progress_bar = crate::ui::progress_bar::ProgressBar::new(
            progress_type,
            title,
            total,
            can_cancel
        );
        
        self.ui_state.progress_bar_manager.add_progress_bar(progress_bar)
    }
    
    /// Update progress
    pub fn update_progress(&mut self, index: usize, completed: u64, status: String) {
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.update(completed, status);
        }
    }
    
    /// Update bulk operation progress
    pub fn update_bulk_progress(
        &mut self, 
        index: usize, 
        completed: u64, 
        successful: u64, 
        failed: u64, 
        current_op: String
    ) {
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.update_bulk(completed, successful, failed, current_op);
        }
    }
    
    /// Complete progress operation
    pub fn complete_progress(&mut self, index: usize, final_status: String) {
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.complete(final_status);
        }
    }
    
    /// Hide progress bar
    pub fn hide_progress(&mut self, index: usize) {
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.hide();
        }
    }
    
    /// Remove progress bar
    pub fn remove_progress(&mut self, index: usize) {
        self.ui_state.progress_bar_manager.remove_progress_bar(index);
    }
    
    /// Check if any progress bars are visible
    pub fn has_active_progress(&self) -> bool {
        self.ui_state.progress_bar_manager.get_active()
            .map(|pb| pb.is_visible)
            .unwrap_or(false)
    }
    
    /// Schedule progress bar removal after delay
    pub fn schedule_progress_removal(&mut self, index: usize, delay_ms: u64) {
        // Mark the progress as completed and allow manual dismissal
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.can_cancel = true; // Allow manual dismissal
            
            // For completed operations, just hide immediately if they're done
            if progress_bar.progress >= 1.0 {
                progress_bar.hide();
            }
        }
    }
    
    /// Dismiss active progress bars (for Esc key)
    pub fn dismiss_progress_bars(&mut self) {
        self.ui_state.progress_bar_manager.hide_all();
    }
}