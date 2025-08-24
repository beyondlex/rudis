use crate::error::AppResult;
use super::state_core::AppState;
use crossterm::terminal;

impl AppState {
    /// Calculate the visible key count based on available display area
    pub fn get_visible_key_count() -> usize {
        // Calculate a more dynamic viewport size based on typical terminal constraints
        // This should roughly match what the renderer calculates
        
        // Get terminal size if available
        if let Ok(size) = crossterm::terminal::size() {
            let terminal_height = size.1 as usize;
            
            // Calculate available space for keys:
            // - 3 lines for header
            // - 4 lines for command input
            // - 3 lines for footer
            // - 3 lines for panel borders (database browser borders)
            // - 2-3 lines for help text and status messages
            let reserved_lines = 3 + 4 + 3 + 3 + 3; // Total: 16 lines
            
            if terminal_height > reserved_lines {
                let available_height = terminal_height - reserved_lines;
                // Use a reasonable portion for the keys area, but cap at 15 for usability
                available_height.min(15).max(3) // Minimum 3, maximum 15 keys visible
            } else {
                // Fallback for very small terminals
                3
            }
        } else {
            // Fallback when terminal size cannot be determined
            10
        }
    }
    
    /// Calculate the maximum scroll offset for the current content
    pub fn get_max_scroll_offset(&self) -> usize {
        let browser = &self.ui_state.database_browser;
        
        let total_items = if browser.use_tree_view {
            browser.key_tree.visible_count()
        } else {
            browser.keys.len()
        };
        
        let visible_items = Self::get_visible_key_count();
        
        if total_items > visible_items {
            total_items - visible_items
        } else {
            0
        }
    }
    
    /// Update scrollbar state based on current scroll position and total items
    /// Uses dynamic viewport size to match actual rendering
    pub fn update_scrollbar_state(&mut self, viewport_size: Option<usize>) {
        let browser = &mut self.ui_state.database_browser;
        
        let total_items = if browser.use_tree_view {
            browser.key_tree.visible_count()
        } else {
            browser.keys.len()
        };
        
        // Use provided viewport size or fallback to default
        let visible_items = viewport_size.unwrap_or_else(|| Self::get_visible_key_count());
        
        // Ensure scroll_offset is within valid bounds
        let max_scroll_offset = if total_items > visible_items {
            total_items - visible_items
        } else {
            0
        };
        
        // Clamp scroll_offset to valid range
        browser.scroll_offset = browser.scroll_offset.min(max_scroll_offset);
        
        browser.scrollbar_state = browser.scrollbar_state
            .content_length(total_items)
            .viewport_content_length(visible_items)
            .position(browser.scroll_offset);
    }
    
    /// Select next key in the browser - optimized for performance
    pub fn select_next_key(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        
        if browser.use_tree_view {
            // Tree view navigation
            let visible_count = browser.key_tree.visible_count();
            if visible_count > 0 {
                let old_index = browser.selected_key_index;
                browser.selected_key_index = (browser.selected_key_index + 1).min(visible_count - 1);
                
                if old_index != browser.selected_key_index {
                    // Adjust scroll offset if needed - use dynamic display count
                    let display_count = Self::get_visible_key_count();
                    let total_items = browser.key_tree.visible_count();
                    let max_scroll_offset = if total_items > display_count {
                        total_items - display_count
                    } else {
                        0
                    };
                    
                    if browser.selected_key_index >= browser.scroll_offset + display_count {
                        browser.scroll_offset = (browser.selected_key_index - display_count + 1).min(max_scroll_offset);
                    }
                    
                    // Update selected key from tree
                    if let Some(display_info) = browser.key_tree.get_visible_node_info(browser.selected_key_index) {
                        if display_info.is_key {
                            if let Some(key_info) = &display_info.key_info {
                                self.selected_key = Some(key_info.name.clone());
                            }
                        } else {
                            // For non-key nodes, clear selected key
                            self.selected_key = None;
                        }
                    }
                }
            }
        } else {
            // Flat list navigation
            if !browser.keys.is_empty() {
                let old_index = browser.selected_key_index;
                browser.selected_key_index = (browser.selected_key_index + 1).min(browser.keys.len() - 1);
                
                // Only update if index actually changed
                if old_index != browser.selected_key_index {
                    // Adjust scroll offset if needed - use dynamic display count
                    let display_count = Self::get_visible_key_count();
                    let total_items = browser.keys.len();
                    let max_scroll_offset = if total_items > display_count {
                        total_items - display_count
                    } else {
                        0
                    };
                    
                    if browser.selected_key_index >= browser.scroll_offset + display_count {
                        browser.scroll_offset = (browser.selected_key_index - display_count + 1).min(max_scroll_offset);
                    }
                    
                    // Update selected key - use reference to avoid cloning when possible
                    if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                        self.selected_key = Some(key_info.name.clone());
                    }
                }
            }
        }
        
        // Update scrollbar state after navigation
        self.update_scrollbar_state(None);
    }
    
    /// Select previous key in the browser - optimized for performance
    pub fn select_previous_key(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        
        if browser.use_tree_view {
            // Tree view navigation
            if browser.selected_key_index > 0 {
                let old_index = browser.selected_key_index;
                browser.selected_key_index -= 1;
                
                if old_index != browser.selected_key_index {
                    // Adjust scroll offset if needed
                    if browser.selected_key_index < browser.scroll_offset {
                        browser.scroll_offset = browser.selected_key_index;
                    }
                    
                    // Update selected key from tree
                    if let Some(display_info) = browser.key_tree.get_visible_node_info(browser.selected_key_index) {
                        if display_info.is_key {
                            if let Some(key_info) = &display_info.key_info {
                                self.selected_key = Some(key_info.name.clone());
                            }
                        } else {
                            // For non-key nodes, clear selected key
                            self.selected_key = None;
                        }
                    }
                }
            }
        } else {
            // Flat list navigation
            if browser.selected_key_index > 0 {
                let old_index = browser.selected_key_index;
                browser.selected_key_index -= 1;
                
                // Only update if index actually changed
                if old_index != browser.selected_key_index {
                    // Adjust scroll offset if needed
                    if browser.selected_key_index < browser.scroll_offset {
                        browser.scroll_offset = browser.selected_key_index;
                    }
                    
                    // Update selected key - use reference to avoid cloning when possible
                    if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                        self.selected_key = Some(key_info.name.clone());
                    }
                }
            }
        }
        
        // Update scrollbar state after navigation
        self.update_scrollbar_state(None);
    }
    
    /// Select key by offset for efficient page navigation
    pub fn select_key_by_offset(&mut self, offset: i32) {
        let browser = &mut self.ui_state.database_browser;
        if browser.keys.is_empty() {
            return;
        }
        
        let old_index = browser.selected_key_index;
        let new_index = if offset < 0 {
            browser.selected_key_index.saturating_sub((-offset) as usize)
        } else {
            (browser.selected_key_index + offset as usize).min(browser.keys.len() - 1)
        };
        
        if old_index != new_index {
            browser.selected_key_index = new_index;
            
            // Adjust scroll offset for the new position - use dynamic display count
            let display_count = Self::get_visible_key_count();
            let total_items = browser.keys.len();
            let max_scroll_offset = if total_items > display_count {
                total_items - display_count
            } else {
                0
            };
            
            if browser.selected_key_index >= browser.scroll_offset + display_count {
                browser.scroll_offset = (browser.selected_key_index - display_count + 1).min(max_scroll_offset);
            } else if browser.selected_key_index < browser.scroll_offset {
                browser.scroll_offset = browser.selected_key_index;
            }
            
            // Update selected key
            if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                self.selected_key = Some(key_info.name.clone());
            }
        }
        
        // Update scrollbar state after navigation
        self.update_scrollbar_state(None);
    }
    
    /// Get currently selected key info (works for both tree and flat view)
    pub fn get_selected_key_info(&self) -> Option<&crate::app::states::KeyInfo> {
        let browser = &self.ui_state.database_browser;
        
        if browser.use_tree_view {
            // In tree view, get key info from tree structure
            browser.key_tree.get_key_info_at_index(browser.selected_key_index)
        } else {
            // In flat view, get from keys vector
            browser.keys.get(browser.selected_key_index)
        }
    }
}