use crate::error::AppResult;
use crate::app::states::KeyInfo;
use super::state_core::AppState;

impl AppState {
    /// Load available databases for active connection
    pub async fn load_databases(&mut self) -> AppResult<()> {
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.get_databases().await {
                Ok(databases) => {
                    self.ui_state.database_browser.databases = databases;
                    self.set_status(format!("Found {} databases", self.ui_state.database_browser.databases.len()));
                }
                Err(err) => {
                    self.set_status(format!("Failed to load databases: {}", err));
                }
            }
        }
        Ok(())
    }
    
    /// Select a database
    pub async fn select_database(&mut self, db_num: u8) -> AppResult<()> {
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.select_database(db_num).await {
                Ok(()) => {
                    self.ui_state.database_browser.selected_database = db_num;
                    self.selected_database = Some(db_num);
                    // Clear current keys and reset scanning
                    self.ui_state.database_browser.keys.clear();
                    self.ui_state.database_browser.scan_cursor = 0;
                    self.ui_state.database_browser.scan_complete = false;
                    self.ui_state.database_browser.selected_key_index = 0;
                    // Load keys for the new database silently
                    self.load_keys_silent().await?;
                    self.set_status(format!("Selected database {}", db_num));
                }
                Err(err) => {
                    self.set_status(format!("Failed to select database {}: {}", db_num, err));
                }
            }
        }
        Ok(())
    }
    
    /// Load keys from current database with progress dialog
    pub async fn load_keys(&mut self) -> AppResult<()> {
        self.load_keys_internal(true).await
    }
    
    /// Load keys silently without progress dialog (for initial connection)
    pub async fn load_keys_silent(&mut self) -> AppResult<()> {
        self.load_keys_internal(false).await
    }
    
    /// Internal method to load keys with optional progress dialog
    async fn load_keys_internal(&mut self, show_progress: bool) -> AppResult<()> {
        if self.ui_state.database_browser.loading {
            return Ok(()); // Already loading
        }
        
        self.ui_state.database_browser.loading = true;
        
        // Conditionally start progress for key scanning
        let progress_index = if show_progress {
            Some(self.start_progress(
                crate::ui::progress_bar::ProgressType::DataLoading,
                "Loading Keys".to_string(),
                0, // Unknown total initially
                false // Cannot cancel Redis SCAN
            ))
        } else {
            None
        };
        
        // Extract values to avoid borrowing conflicts
        let pattern = if self.ui_state.database_browser.filter_pattern.is_empty() {
            "*".to_string()
        } else {
            format!("*{}*", self.ui_state.database_browser.filter_pattern)
        };
        
        let scan_cursor = self.ui_state.database_browser.scan_cursor;
        let keys_per_page = self.config.preferences.keys_per_page;
        
        if let Some(progress_index) = progress_index {
            self.update_progress(progress_index, 0, "Starting key scan...".to_string());
        }
        
        // Perform scan operation
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.scan_keys(scan_cursor, &pattern, keys_per_page).await {
                Ok((new_cursor, key_names)) => {
                    // Update scan state
                    self.ui_state.database_browser.scan_cursor = new_cursor;
                    if new_cursor == 0 {
                        self.ui_state.database_browser.scan_complete = true;
                    }
                    
                    if let Some(progress_index) = progress_index {
                        self.update_progress(
                            progress_index, 
                            key_names.len() as u64, 
                            format!("Processing {} keys...", key_names.len())
                        );
                    }
                    
                    if !key_names.is_empty() {
                        // Create KeyInfo without type information
                        let mut key_infos = Vec::new();
                        for key_name in key_names {
                            let key_info = KeyInfo {
                                name: key_name,
                                key_type: None, // Will be loaded separately
                                ttl: None,      // Will be loaded separately
                                size: None,
                                matches_filter: true,
                            };
                            key_infos.push(key_info);
                        }
                        
                        // Append new keys to existing ones
                        self.ui_state.database_browser.keys.extend(key_infos);
                        
                        // Rebuild tree view if enabled
                        if self.ui_state.database_browser.use_tree_view {
                            self.rebuild_tree_view();
                        }
                        
                        let final_status = format!(
                            "Loaded {} keys", 
                            self.ui_state.database_browser.keys.len()
                        );
                        
                        if let Some(progress_index) = progress_index {
                            self.complete_progress(progress_index, final_status.clone());
                        }
                        self.set_status(final_status);
                        
                        // Load types and TTLs for the first few keys asynchronously
                        self.load_key_details().await?;
                    } else {
                        if let Some(progress_index) = progress_index {
                            self.complete_progress(progress_index, "No keys found".to_string());
                        }
                        self.set_status("No keys found".to_string());
                    }
                }
                Err(err) => {
                    let error_msg = format!("Failed to scan keys: {}", err);
                    if let Some(progress_index) = progress_index {
                        self.complete_progress(progress_index, error_msg.clone());
                    }
                    self.set_status(error_msg);
                }
            }
        } else {
            if let Some(progress_index) = progress_index {
                self.complete_progress(progress_index, "No active connection".to_string());
            }
        }
        
        self.ui_state.database_browser.loading = false;
        
        // Update scrollbar state after loading keys
        self.update_scrollbar_state(None);
        
        // Schedule progress bar removal if we showed one
        if let Some(progress_index) = progress_index {
            self.schedule_progress_removal(progress_index, 1500);
        }
        
        Ok(())
    }
    
    /// Load type and TTL information for keys that don't have it yet
    pub async fn load_key_details(&mut self) -> AppResult<()> {
        // Load details for up to 10 keys at a time to avoid blocking UI
        let mut keys_to_process = Vec::new();
        let mut indices_to_update = Vec::new();
        
        for (idx, key_info) in self.ui_state.database_browser.keys.iter().enumerate() {
            if key_info.key_type.is_none() && keys_to_process.len() < 10 {
                keys_to_process.push(key_info.name.clone());
                indices_to_update.push(idx);
            }
        }
        
        if keys_to_process.is_empty() {
            return Ok(());
        }
        
        // Load key information
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.get_keys_info(&keys_to_process).await {
                Ok(key_infos_data) => {
                    let mut types_loaded = 0;
                    let mut ttls_loaded = 0;
                    
                    // Update the key information
                    for ((_, key_type, ttl), &idx) in key_infos_data.iter().zip(indices_to_update.iter()) {
                        if let Some(key_info) = self.ui_state.database_browser.keys.get_mut(idx) {
                            key_info.key_type = key_type.clone();
                            key_info.ttl = *ttl;
                            
                            if key_type.is_some() {
                                types_loaded += 1;
                            }
                            if ttl.is_some() {
                                ttls_loaded += 1;
                            }
                        }
                    }
                    
                    if types_loaded > 0 || ttls_loaded > 0 {
                        self.set_status(format!(
                            "Loaded details: {} types, {} TTLs", 
                            types_loaded, ttls_loaded
                        ));
                    }
                }
                Err(err) => {
                    self.set_status(format!("Failed to load key details: {}", err));
                }
            }
        }
        
        Ok(())
    }
    
    /// Load more keys (pagination)
    pub async fn load_more_keys(&mut self) -> AppResult<()> {
        if !self.ui_state.database_browser.scan_complete {
            self.load_keys().await?
        }
        Ok(())
    }
    
    /// Schedule key loading without blocking UI - for responsive navigation
    pub fn schedule_key_loading(&mut self) -> AppResult<()> {
        if !self.ui_state.database_browser.loading && !self.ui_state.database_browser.scan_complete {
            // Send an async event to load more keys in the background
            let _ = self.event_tx.send(crate::events::AppEvent::RefreshData);
        }
        Ok(())
    }
}