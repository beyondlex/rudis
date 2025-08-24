use crate::error::AppResult;
use super::state_core::AppState;

impl AppState {
    /// Rebuild tree view from current keys
    pub fn rebuild_tree_view(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        browser.key_tree.build_from_keys(&browser.keys);
        
        // Update selected index to match current key in tree
        if let Some(selected_key) = &self.selected_key {
            if let Some(tree_index) = browser.key_tree.find_key_index(selected_key) {
                browser.selected_key_index = tree_index;
            }
        }
        
        // Update scrollbar state after rebuilding tree
        self.update_scrollbar_state(None);
    }
    
    /// Toggle tree view mode
    pub fn toggle_tree_view(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        browser.use_tree_view = !browser.use_tree_view;
        
        if browser.use_tree_view {
            // Build tree from current keys
            browser.key_tree.build_from_keys(&browser.keys);
            
            // Update selected index to match current key in tree
            if let Some(selected_key) = &self.selected_key {
                if let Some(tree_index) = browser.key_tree.find_key_index(selected_key) {
                    browser.selected_key_index = tree_index;
                }
            }
        }
    }
    
    /// Toggle node expansion in tree view
    pub fn toggle_tree_node(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        if browser.use_tree_view {
            if browser.key_tree.toggle_node_at_index(browser.selected_key_index) {
                // Tree was rebuilt, may need to adjust selected index
                let visible_count = browser.key_tree.visible_count();
                if browser.selected_key_index >= visible_count && visible_count > 0 {
                    browser.selected_key_index = visible_count - 1;
                }
                
                // Update scroll offset if needed
                let display_count = 10;
                if browser.selected_key_index >= browser.scroll_offset + display_count {
                    browser.scroll_offset = browser.selected_key_index - display_count + 1;
                } else if browser.selected_key_index < browser.scroll_offset {
                    browser.scroll_offset = browser.selected_key_index;
                }
            }
        }
    }
}