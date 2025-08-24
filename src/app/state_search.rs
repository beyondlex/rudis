use crate::error::AppResult;
use super::state_core::AppState;

impl AppState {
    /// Set filter pattern for key search
    pub async fn set_key_filter(&mut self, pattern: String) -> AppResult<()> {
        self.ui_state.database_browser.filter_pattern = pattern;
        // Reset scanning and reload keys with new filter
        self.ui_state.database_browser.keys.clear();
        self.ui_state.database_browser.scan_cursor = 0;
        self.ui_state.database_browser.scan_complete = false;
        self.ui_state.database_browser.selected_key_index = 0;
        self.load_keys().await
    }
    
    /// Enter search mode for key filtering
    pub fn enter_search_mode(&mut self) {
        self.ui_state.database_browser.search_mode = true;
        self.ui_state.database_browser.filter_pattern.clear();
    }
    
    /// Exit search mode
    pub fn exit_search_mode(&mut self) {
        self.ui_state.database_browser.search_mode = false;
        if !self.ui_state.database_browser.filter_pattern.is_empty() {
            // Clear filter and reload all keys
            self.ui_state.database_browser.filter_pattern.clear();
            // Reset scanning state
            self.ui_state.database_browser.keys.clear();
            self.ui_state.database_browser.scan_cursor = 0;
            self.ui_state.database_browser.scan_complete = false;
            self.ui_state.database_browser.selected_key_index = 0;
        }
    }
    
    /// Add character to search pattern
    pub fn add_search_char(&mut self, ch: char) {
        if self.ui_state.database_browser.search_mode {
            self.ui_state.database_browser.filter_pattern.push(ch);
        }
    }
    
    /// Remove last character from search pattern
    pub fn backspace_search(&mut self) {
        if self.ui_state.database_browser.search_mode {
            self.ui_state.database_browser.filter_pattern.pop();
        }
    }
    
    /// Apply current search filter
    pub async fn apply_search_filter(&mut self) -> AppResult<()> {
        if self.ui_state.database_browser.search_mode {
            // Reset scanning state and search with new pattern
            self.ui_state.database_browser.keys.clear();
            self.ui_state.database_browser.scan_cursor = 0;
            self.ui_state.database_browser.scan_complete = false;
            self.ui_state.database_browser.selected_key_index = 0;
            // Load keys with filter
            self.load_keys().await?;
            // Exit search mode after applying
            self.ui_state.database_browser.search_mode = false;
        }
        Ok(())
    }
}