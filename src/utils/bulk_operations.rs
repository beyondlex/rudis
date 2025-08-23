use std::collections::HashMap;
use tokio::time::{sleep, Duration};

use crate::error::AppResult;
use crate::redis::value_types::RedisValue;

/// Bulk operation types
#[derive(Debug, Clone, PartialEq)]
pub enum BulkOperation {
    /// Delete multiple keys
    Delete,
    /// Set TTL for multiple keys
    SetTtl(i64),
    /// Remove TTL (persist) for multiple keys
    RemoveTtl,
    /// Copy keys to different database
    Copy { target_db: u8 },
    /// Rename keys with pattern replacement
    Rename { pattern: String, replacement: String },
    /// Export multiple keys
    Export { format: crate::utils::export_import::ExportFormat },
    /// Set same value for multiple keys
    SetValue { value: String },
    /// Increment numeric values
    Increment { amount: i64 },
    /// Append to string values
    AppendString { suffix: String },
    /// Add elements to sets
    AddToSet { members: Vec<String> },
    /// Add fields to hashes
    AddToHash { fields: HashMap<String, String> },
}

/// Bulk operation progress
#[derive(Debug, Clone)]
pub struct BulkProgress {
    /// Total number of operations
    pub total: usize,
    /// Number of completed operations
    pub completed: usize,
    /// Number of successful operations
    pub successful: usize,
    /// Number of failed operations
    pub failed: usize,
    /// Current operation description
    pub current_operation: String,
    /// Whether the operation is complete
    pub is_complete: bool,
    /// Error messages for failed operations
    pub errors: Vec<String>,
}

impl BulkProgress {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            completed: 0,
            successful: 0,
            failed: 0,
            current_operation: "Starting...".to_string(),
            is_complete: false,
            errors: Vec::new(),
        }
    }
    
    pub fn update(&mut self, operation: String, success: bool, error: Option<String>) {
        self.completed += 1;
        self.current_operation = operation;
        
        if success {
            self.successful += 1;
        } else {
            self.failed += 1;
            if let Some(err) = error {
                self.errors.push(err);
            }
        }
        
        self.is_complete = self.completed >= self.total;
    }
    
    pub fn progress_percentage(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.completed as f64 / self.total as f64) * 100.0
        }
    }
}

/// Bulk operation result
#[derive(Debug, Clone)]
pub struct BulkOperationResult {
    /// Number of successful operations
    pub successful: usize,
    /// Number of failed operations
    pub failed: usize,
    /// List of errors that occurred
    pub errors: Vec<String>,
    /// Time taken for the operation
    pub duration: Duration,
}

/// Bulk operations manager
pub struct BulkOperationsManager;

impl BulkOperationsManager {
    /// Execute a single operation on a key (for progress tracking)
    pub async fn execute_single_operation(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
        operation: &BulkOperation,
    ) -> AppResult<()> {
        match operation {
            BulkOperation::Delete => {
                Self::delete_key(connection, key).await
            }
            BulkOperation::SetTtl(ttl) => {
                Self::set_key_ttl(connection, key, *ttl).await
            }
            BulkOperation::RemoveTtl => {
                Self::remove_key_ttl(connection, key).await
            }
            BulkOperation::Copy { target_db } => {
                Self::copy_key(connection, key, *target_db).await
            }
            BulkOperation::Rename { pattern, replacement } => {
                Self::rename_key(connection, key, pattern, replacement).await
            }
            BulkOperation::Export { format: _ } => {
                // Export is handled separately as it doesn't modify Redis
                Ok(())
            }
            BulkOperation::SetValue { value } => {
                Self::set_key_value(connection, key, value).await
            }
            BulkOperation::Increment { amount } => {
                Self::increment_key(connection, key, *amount).await
            }
            BulkOperation::AppendString { suffix } => {
                Self::append_to_key(connection, key, suffix).await
            }
            BulkOperation::AddToSet { members } => {
                Self::add_to_set(connection, key, members).await
            }
            BulkOperation::AddToHash { fields } => {
                Self::add_to_hash(connection, key, fields).await
            }
        }
    }
    
    /// Execute a bulk operation on multiple keys
    pub async fn execute_bulk_operation(
        connection: &mut crate::redis::RedisConnection,
        keys: Vec<String>,
        operation: BulkOperation,
        progress_callback: Option<Box<dyn Fn(&BulkProgress) + Send>>,
    ) -> AppResult<BulkOperationResult> {
        let start_time = std::time::Instant::now();
        let mut progress = BulkProgress::new(keys.len());
        
        // Send initial progress
        if let Some(ref callback) = progress_callback {
            callback(&progress);
        }
        
        let mut successful = 0;
        let mut failed = 0;
        let mut errors = Vec::new();
        
        for (index, key) in keys.iter().enumerate() {
            let operation_desc = format!("Processing {} ({}/{})", key, index + 1, keys.len());
            
            let result = match &operation {
                BulkOperation::Delete => {
                    Self::delete_key(connection, key).await
                }
                BulkOperation::SetTtl(ttl) => {
                    Self::set_key_ttl(connection, key, *ttl).await
                }
                BulkOperation::RemoveTtl => {
                    Self::remove_key_ttl(connection, key).await
                }
                BulkOperation::Copy { target_db } => {
                    Self::copy_key(connection, key, *target_db).await
                }
                BulkOperation::Rename { pattern, replacement } => {
                    Self::rename_key(connection, key, pattern, replacement).await
                }
                BulkOperation::Export { format: _ } => {
                    // Export is handled separately as it doesn't modify Redis
                    Ok(())
                }
                BulkOperation::SetValue { value } => {
                    Self::set_key_value(connection, key, value).await
                }
                BulkOperation::Increment { amount } => {
                    Self::increment_key(connection, key, *amount).await
                }
                BulkOperation::AppendString { suffix } => {
                    Self::append_to_key(connection, key, suffix).await
                }
                BulkOperation::AddToSet { members } => {
                    Self::add_to_set(connection, key, members).await
                }
                BulkOperation::AddToHash { fields } => {
                    Self::add_to_hash(connection, key, fields).await
                }
            };
            
            match result {
                Ok(_) => {
                    successful += 1;
                    progress.update(operation_desc, true, None);
                }
                Err(err) => {
                    failed += 1;
                    let error_msg = format!("Key '{}': {}", key, err);
                    errors.push(error_msg.clone());
                    progress.update(operation_desc, false, Some(error_msg));
                }
            }
            
            // Send progress update
            if let Some(ref callback) = progress_callback {
                callback(&progress);
            }
            
            // Small delay to prevent overwhelming Redis
            sleep(Duration::from_millis(10)).await;
        }
        
        let duration = start_time.elapsed();
        
        Ok(BulkOperationResult {
            successful,
            failed,
            errors,
            duration,
        })
    }
    
    /// Delete a single key
    async fn delete_key(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
    ) -> AppResult<()> {
        connection.execute_command("DEL", &[key]).await?;
        Ok(())
    }
    
    /// Set TTL for a key
    async fn set_key_ttl(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
        ttl: i64,
    ) -> AppResult<()> {
        connection.set_ttl(key, ttl).await
    }
    
    /// Remove TTL from a key (make it persistent)
    async fn remove_key_ttl(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
    ) -> AppResult<()> {
        connection.persist_key(key).await
    }
    
    /// Copy key to different database
    async fn copy_key(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
        target_db: u8,
    ) -> AppResult<()> {
        // For now, return an error as this requires complex Redis operations
        // In a full implementation, you would:
        // 1. Get the value and type of the key
        // 2. Switch to target database
        // 3. Recreate the key with same value and TTL
        // 4. Switch back to original database
        Err(crate::error::AppError::Generic(
            "Copy operation not yet implemented - requires value retrieval methods".to_string()
        ))
    }
    
    /// Rename key with pattern replacement
    async fn rename_key(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
        pattern: &str,
        replacement: &str,
    ) -> AppResult<()> {
        let new_key = key.replace(pattern, replacement);
        if new_key == key {
            return Err(crate::error::AppError::Generic(
                "Pattern replacement resulted in same key name".to_string()
            ));
        }
        
        connection.bulk_rename_key(key, &new_key).await
    }
    
    /// Set value for a key
    async fn set_key_value(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
        value: &str,
    ) -> AppResult<()> {
        connection.bulk_set_string(key, value).await
    }
    
    /// Increment a numeric key
    async fn increment_key(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
        amount: i64,
    ) -> AppResult<()> {
        connection.increment_key(key, amount).await
    }
    
    /// Append to a string key
    async fn append_to_key(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
        suffix: &str,
    ) -> AppResult<()> {
        connection.append_to_string(key, suffix).await
    }
    
    /// Add members to a set
    async fn add_to_set(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
        members: &[String],
    ) -> AppResult<()> {
        for member in members {
            connection.add_to_set(key, member).await?;
        }
        Ok(())
    }
    
    /// Add fields to a hash
    async fn add_to_hash(
        connection: &mut crate::redis::RedisConnection,
        key: &str,
        fields: &HashMap<String, String>,
    ) -> AppResult<()> {
        for (field, value) in fields {
            connection.bulk_set_hash_field(key, field, value).await?;
        }
        Ok(())
    }
    
    /// Filter keys by pattern
    pub fn filter_keys_by_pattern(keys: &[String], pattern: &str) -> Vec<String> {
        if pattern.is_empty() || pattern == "*" {
            return keys.to_vec();
        }
        
        let regex_pattern = Self::glob_to_regex(pattern);
        
        keys.iter()
            .filter(|key| regex_pattern.is_match(key))
            .cloned()
            .collect()
    }
    
    /// Convert glob pattern to regex
    fn glob_to_regex(pattern: &str) -> regex::Regex {
        let regex_pattern = pattern
            .replace("*", ".*")
            .replace("?", ".")
            .replace("[", "\\[")
            .replace("]", "\\]")
            .replace("(", "\\(")
            .replace(")", "\\)")
            .replace("+", "\\+")
            .replace("^", "\\^")
            .replace("$", "\\$")
            .replace(".", "\\.");
        
        regex::Regex::new(&format!("^{}$", regex_pattern))
            .unwrap_or_else(|_| regex::Regex::new(".*").unwrap())
    }
    
    /// Validate bulk operation
    pub fn validate_operation(
        operation: &BulkOperation,
        keys: &[String],
    ) -> Result<(), String> {
        if keys.is_empty() {
            return Err("No keys selected for bulk operation".to_string());
        }
        
        match operation {
            BulkOperation::SetTtl(ttl) => {
                if *ttl <= 0 {
                    return Err("TTL must be greater than 0".to_string());
                }
            }
            BulkOperation::Copy { target_db } => {
                if *target_db > 15 {
                    return Err("Target database must be between 0-15".to_string());
                }
            }
            BulkOperation::Rename { pattern, replacement } => {
                if pattern.is_empty() {
                    return Err("Pattern cannot be empty".to_string());
                }
                if replacement.is_empty() {
                    return Err("Replacement cannot be empty".to_string());
                }
            }
            BulkOperation::SetValue { value } => {
                if value.is_empty() {
                    return Err("Value cannot be empty".to_string());
                }
            }
            BulkOperation::Increment { amount: _ } => {
                // Always valid
            }
            BulkOperation::AppendString { suffix } => {
                if suffix.is_empty() {
                    return Err("Suffix cannot be empty".to_string());
                }
            }
            BulkOperation::AddToSet { members } => {
                if members.is_empty() {
                    return Err("No members to add".to_string());
                }
            }
            BulkOperation::AddToHash { fields } => {
                if fields.is_empty() {
                    return Err("No fields to add".to_string());
                }
            }
            _ => {
                // Other operations are always valid
            }
        }
        
        Ok(())
    }
    
    /// Get operation description
    pub fn get_operation_description(operation: &BulkOperation) -> String {
        match operation {
            BulkOperation::Delete => "Delete selected keys".to_string(),
            BulkOperation::SetTtl(ttl) => format!("Set TTL to {} seconds", ttl),
            BulkOperation::RemoveTtl => "Remove TTL (make persistent)".to_string(),
            BulkOperation::Copy { target_db } => format!("Copy to database {}", target_db),
            BulkOperation::Rename { pattern, replacement } => {
                format!("Rename: replace '{}' with '{}'", pattern, replacement)
            }
            BulkOperation::Export { format } => format!("Export as {}", format),
            BulkOperation::SetValue { value } => {
                format!("Set value to '{}'", if value.len() > 20 { 
                    format!("{}...", &value[..17]) 
                } else { 
                    value.clone() 
                })
            }
            BulkOperation::Increment { amount } => format!("Increment by {}", amount),
            BulkOperation::AppendString { suffix } => {
                format!("Append '{}'", if suffix.len() > 20 { 
                    format!("{}...", &suffix[..17]) 
                } else { 
                    suffix.clone() 
                })
            }
            BulkOperation::AddToSet { members } => {
                format!("Add {} members to sets", members.len())
            }
            BulkOperation::AddToHash { fields } => {
                format!("Add {} fields to hashes", fields.len())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_glob_to_regex() {
        let regex = BulkOperationsManager::glob_to_regex("user:*");
        assert!(regex.is_match("user:123"));
        assert!(regex.is_match("user:abc"));
        assert!(!regex.is_match("admin:123"));
        
        let regex = BulkOperationsManager::glob_to_regex("cache:?");
        assert!(regex.is_match("cache:1"));
        assert!(regex.is_match("cache:a"));
        assert!(!regex.is_match("cache:12"));
    }
    
    #[test]
    fn test_filter_keys_by_pattern() {
        let keys = vec![
            "user:1".to_string(),
            "user:2".to_string(),
            "admin:1".to_string(),
            "cache:data".to_string(),
        ];
        
        let filtered = BulkOperationsManager::filter_keys_by_pattern(&keys, "user:*");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"user:1".to_string()));
        assert!(filtered.contains(&"user:2".to_string()));
    }
    
    #[test]
    fn test_validate_operation() {
        let keys = vec!["key1".to_string(), "key2".to_string()];
        
        // Valid operations
        assert!(BulkOperationsManager::validate_operation(&BulkOperation::Delete, &keys).is_ok());
        assert!(BulkOperationsManager::validate_operation(&BulkOperation::SetTtl(3600), &keys).is_ok());
        
        // Invalid operations
        assert!(BulkOperationsManager::validate_operation(&BulkOperation::SetTtl(-1), &keys).is_err());
        assert!(BulkOperationsManager::validate_operation(&BulkOperation::Delete, &[]).is_err());
    }
}