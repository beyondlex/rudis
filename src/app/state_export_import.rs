use crate::error::AppResult;
use super::state_core::AppState;

impl AppState {
    /// Open export dialog for current key
    pub fn open_export_dialog(&mut self) {
        if let Some(key_name) = &self.ui_state.key_viewer.current_key {
            let default_path = format!("{}.json", key_name.replace(':', "_"));
            self.ui_state.export_import_dialog.open_export(default_path);
        }
    }
    
    /// Open import dialog
    pub fn open_import_dialog(&mut self) {
        let default_path = "import_data.json".to_string();
        self.ui_state.export_import_dialog.open_import(default_path);
    }
    
    /// Export current key value
    pub async fn export_current_key(&mut self) -> AppResult<()> {
        // Extract data first to avoid borrowing conflicts
        let (key_name, format) = {
            let dialog = &self.ui_state.export_import_dialog;
            if !dialog.is_open {
                return Ok(());
            }
            
            let key_name = match &self.ui_state.key_viewer.current_key {
                Some(key) => key.clone(),
                None => {
                    self.set_status("No key selected for export".to_string());
                    return Ok(());
                }
            };
            
            if self.ui_state.key_viewer.value.is_none() {
                self.set_status("No value loaded for export".to_string());
                return Ok(());
            }
            
            (key_name, dialog.selected_format.clone())
        };
        
        // Start progress for export operation
        let progress_index = self.start_progress(
            crate::ui::progress_bar::ProgressType::Transfer,
            format!("Exporting Key: {}", key_name),
            1, // Single key export
            false
        );
        
        self.update_progress(progress_index, 0, "Preparing export data...".to_string());
        
        let value = self.ui_state.key_viewer.value.as_ref().unwrap();
        let ttl = self.ui_state.key_viewer.metadata.as_ref()
            .and_then(|m| m.ttl)
            .filter(|&t| t > 0);
        
        // Simulate export processing with progress
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        match crate::utils::DataExporter::export_value(
            &key_name,
            value,
            ttl,
            &format,
        ) {
            Ok(exported_data) => {
                self.update_progress(progress_index, 1, "Writing export file...".to_string());
                
                // Simulate file writing delay
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                
                let final_status = format!(
                    "Exported '{}' to {} format",
                    key_name, format
                );
                
                self.complete_progress(progress_index, final_status.clone());
                let file_path = self.ui_state.export_import_dialog.file_path.clone();
                self.set_status(format!(
                    "Exported key '{}' to {} format (would save to '{}')",
                    key_name, format, file_path
                ));
                
                self.ui_state.export_import_dialog.close();
                
                // Remove progress bar after delay
                self.schedule_progress_removal(progress_index, 2000);
            }
            Err(err) => {
                let error_msg = format!("Export failed: {}", err);
                self.complete_progress(progress_index, error_msg.clone());
                self.set_status(error_msg);
                
                // Remove progress bar after error delay
                self.schedule_progress_removal(progress_index, 3000);
            }
        }
        
        Ok(())
    }
    
    /// Import data from file
    pub async fn import_data(&mut self) -> AppResult<()> {
        let dialog = &self.ui_state.export_import_dialog;
        if !dialog.is_open {
            return Ok(());
        }
        
        // In a real implementation, you would read from file here
        // For now, we'll just show a placeholder message
        self.set_status(format!(
            "Would import from '{}' using {} format",
            dialog.file_path, dialog.selected_format
        ));
        self.ui_state.export_import_dialog.close();
        
        Ok(())
    }
} 