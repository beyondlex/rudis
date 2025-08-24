use super::state_core::AppState;

impl AppState {
    /// Open confirmation dialog for saving changes
    pub fn confirm_save_changes(&mut self, key_name: String, old_value: String, new_value: String) {
        let old_summary = Self::create_value_summary(&old_value);
        let new_summary = Self::create_value_summary(&new_value);
        
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::SaveChanges {
            key_name,
            old_value_summary: old_summary,
            new_value_summary: new_summary,
        });
    }
    
    /// Open confirmation dialog for deleting a key
    pub fn confirm_delete_key(&mut self, key_name: String, key_type: String) {
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::DeleteKey {
            key_name,
            key_type,
        });
    }
    
    /// Open confirmation dialog for discarding changes
    pub fn confirm_discard_changes(&mut self, key_name: String) {
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::DiscardChanges {
            key_name,
        });
    }
    
    /// Open confirmation dialog for large value edit
    pub fn confirm_large_value_edit(&mut self, key_name: String, size: usize) {
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::LargeValueEdit {
            key_name,
            size,
        });
    }
    
    /// Open confirmation dialog for binary data edit
    pub fn confirm_binary_data_edit(&mut self, key_name: String, binary_info: String) {
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::BinaryDataEdit {
            key_name,
            binary_info,
        });
    }
    
    /// Check if edit requires confirmation and show dialog if needed
    pub fn validate_edit_and_confirm(&mut self, key_name: &str) -> bool {
        let viewer_state = &self.ui_state.key_viewer;
        
        // Check for large value
        if viewer_state.edit_buffer.len() > 1024 * 1024 {  // 1MB
            self.confirm_large_value_edit(key_name.to_string(), viewer_state.edit_buffer.len());
            return false; // Need confirmation
        }
        
        // Check for binary data
        if viewer_state.has_binary_data() {
            let binary_info = crate::ui::BinaryViewer::analyze_data(viewer_state.edit_buffer.as_bytes());
            let info_text = format!("{} null bytes, {} control chars", 
                                  binary_info.null_bytes, binary_info.control_chars);
            self.confirm_binary_data_edit(key_name.to_string(), info_text);
            return false; // Need confirmation
        }
        
        // Check validation errors
        let validation = viewer_state.validate_edit_buffer();
        if let crate::ui::ValidationResult::Error(msg) = validation {
            self.set_status(format!("Validation error: {}", msg));
            return false; // Cannot save invalid data
        }
        
        true // No confirmation needed
    }
    
    /// Handle confirmation dialog response
    pub fn handle_confirmation_response(&mut self) -> Option<crate::ui::ConfirmationResponse> {
        if self.ui_state.confirmation_dialog.is_open {
            let response = self.ui_state.confirmation_dialog.get_response();
            if response != crate::ui::ConfirmationResponse::Pending {
                self.ui_state.confirmation_dialog.close();
                Some(response)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Close confirmation dialog
    pub fn close_confirmation_dialog(&mut self) {
        self.ui_state.confirmation_dialog.close();
    }
    
    /// Create a summary of a value for display in confirmations
    fn create_value_summary(value: &str) -> String {
        if value.len() <= 50 {
            value.to_string()
        } else {
            format!("{}... ({} chars)", &value[..47], value.len())
        }
    }
}