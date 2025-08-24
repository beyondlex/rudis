use crate::error::AppResult;
use super::state_core::AppState;

impl AppState {
    /// Open bulk operations dialog with selected keys
    pub fn open_bulk_operations_dialog(&mut self, selected_keys: Vec<String>) {
        self.ui_state.bulk_operations_dialog.open(selected_keys);
    }
    
    /// Execute bulk operation
    pub async fn execute_bulk_operation(&mut self) -> AppResult<()> {
        // Extract data before borrowing
        let (operation, keys) = {
            let dialog = &self.ui_state.bulk_operations_dialog;
            if !dialog.is_open || dialog.selected_keys.is_empty() {
                return Ok(());
            }
            
            let operation = match dialog.get_current_operation() {
                Some(op) => op,
                None => {
                    self.set_status("No operation selected".to_string());
                    return Ok(());
                }
            };
            
            // Validate operation
            if let Err(err) = crate::utils::BulkOperationsManager::validate_operation(
                &operation, 
                &dialog.selected_keys
            ) {
                self.set_status(format!("Validation failed: {}", err));
                return Ok(());
            }
            
            (operation, dialog.selected_keys.clone())
        };
        
        // Start progress bar
        let operation_name = crate::utils::BulkOperationsManager::get_operation_description(&operation);
        let progress_index = self.start_progress(
            crate::ui::progress_bar::ProgressType::BulkOperation,
            format!("Bulk Operation: {}", operation_name),
            keys.len() as u64,
            true
        );
        
        // Start execution
        self.ui_state.bulk_operations_dialog.start_execution();
        
        // Get connection
        let connection = match self.get_active_connection_mut() {
            Some(conn) => conn,
            None => {
                self.set_status("No active connection".to_string());
                self.remove_progress(progress_index);
                return Ok(());
            }
        };
        
        // Execute bulk operation
        let result: AppResult<crate::utils::bulk_operations::BulkOperationResult> = 
            crate::utils::BulkOperationsManager::execute_bulk_operation(
                connection,
                keys,
                operation.clone(),
                None, // No progress callback for now due to borrowing complexity
            ).await;
        
        match result {
            Ok(bulk_result) => {
                let final_status = format!(
                    "Completed: {} successful, {} failed in {:.2}s",
                    bulk_result.successful,
                    bulk_result.failed,
                    bulk_result.duration.as_secs_f64()
                );
                
                self.complete_progress(progress_index, final_status.clone());
                self.set_status(format!("Bulk operation completed: {}", final_status));
                
                // Close dialog on success
                self.ui_state.bulk_operations_dialog.close();
                
                // Refresh keys list if needed
                if matches!(operation, crate::utils::BulkOperation::Delete | 
                          crate::utils::BulkOperation::Rename { .. }) {
                    self.load_keys().await?;
                }
                
                // Remove progress bar after a short delay
                self.schedule_progress_removal(progress_index, 2000);
            }
            Err(err) => {
                self.complete_progress(progress_index, format!("Failed: {}", err));
                self.set_status(format!("Bulk operation failed: {}", err));
                self.ui_state.bulk_operations_dialog.close();
                
                // Remove progress bar after error display
                self.schedule_progress_removal(progress_index, 3000);
            }
        }
        
        Ok(())
    }
    
    /// Get selected keys for bulk operations
    pub fn get_selected_keys_for_bulk(&self) -> Vec<String> {
        // For now, just return current key if any
        // In a full implementation, you'd track multi-selection
        if let Some(key) = &self.selected_key {
            vec![key.clone()]
        } else {
            Vec::new()
        }
    }
}