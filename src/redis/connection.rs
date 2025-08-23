use redis::{Client, Connection, RedisResult};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use crate::app::config::ConnectionConfig;
use crate::error::{AppError, AppResult};

/// Connection status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Not connected
    Disconnected,
    /// Currently connecting
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection failed
    Failed(String),
    /// Connection lost
    Lost,
}

/// Redis connection wrapper with async support
pub struct RedisConnection {
    /// Connection configuration
    pub config: ConnectionConfig,
    
    /// Redis client instance
    pub client: redis::Client,
    
    /// Current connection status
    pub status: ConnectionStatus,
    
    /// Last successful ping time
    pub last_ping: Option<Instant>,
    
    /// Connection statistics
    pub stats: ConnectionStats,
    
    /// Async connection handle
    connection: Mutex<Option<redis::aio::Connection>>,
}

impl std::fmt::Debug for RedisConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisConnection")
            .field("config", &self.config)
            .field("status", &self.status)
            .field("last_ping", &self.last_ping)
            .field("stats", &self.stats)
            .field("connection", &"<async connection>")
            .finish()
    }
}

/// Connection statistics
#[derive(Debug, Default, Clone)]
pub struct ConnectionStats {
    /// Total number of commands executed
    pub commands_executed: u64,
    
    /// Total number of failed commands
    pub commands_failed: u64,
    
    /// Connection established timestamp
    pub connected_at: Option<Instant>,
    
    /// Total bytes sent
    pub bytes_sent: u64,
    
    /// Total bytes received
    pub bytes_received: u64,
}

impl RedisConnection {
    /// Create a new Redis connection
    pub fn new(config: ConnectionConfig) -> AppResult<Self> {
        let connection_url = Self::build_connection_url(&config)?;
        let client = redis::Client::open(connection_url)
            .map_err(AppError::Redis)?;

        Ok(Self {
            config,
            client,
            status: ConnectionStatus::Disconnected,
            last_ping: None,
            stats: ConnectionStats::default(),
            connection: Mutex::new(None),
        })
    }

    /// Build Redis connection URL from configuration
    fn build_connection_url(config: &ConnectionConfig) -> AppResult<String> {
        let mut url = String::new();
        
        // Protocol
        if config.ssl {
            url.push_str("rediss://");
        } else {
            url.push_str("redis://");
        }
        
        // Authentication
        if let Some(ref username) = config.username {
            url.push_str(username);
            if let Some(ref password) = config.password {
                url.push(':');
                url.push_str(password);
            }
            url.push('@');
        } else if let Some(ref password) = config.password {
            url.push(':');
            url.push_str(password);
            url.push('@');
        }
        
        // Host and port
        url.push_str(&config.host);
        url.push(':');
        url.push_str(&config.port.to_string());
        
        // Database
        url.push('/');
        url.push_str(&config.database.to_string());
        
        Ok(url)
    }

    /// Establish connection to Redis server
    pub async fn connect(&mut self) -> AppResult<()> {
        self.status = ConnectionStatus::Connecting;
        
        match self.client.get_async_connection().await {
            Ok(conn) => {
                *self.connection.lock().await = Some(conn);
                self.status = ConnectionStatus::Connected;
                self.stats.connected_at = Some(Instant::now());
                self.last_ping = Some(Instant::now());
                Ok(())
            }
            Err(err) => {
                self.status = ConnectionStatus::Failed(err.to_string());
                Err(AppError::Redis(err))
            }
        }
    }

    /// Disconnect from Redis server
    pub async fn disconnect(&mut self) {
        *self.connection.lock().await = None;
        self.status = ConnectionStatus::Disconnected;
        self.stats.connected_at = None;
        self.last_ping = None;
    }

    /// Check if connection is active
    pub fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected)
    }

    /// Ping the Redis server to check connectivity
    pub async fn ping(&mut self) -> AppResult<String> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            match redis::cmd("PING").query_async::<_, String>(conn).await {
                Ok(result) => {
                    self.last_ping = Some(Instant::now());
                    self.stats.commands_executed += 1;
                    Ok(result)
                }
                Err(err) => {
                    self.status = ConnectionStatus::Lost;
                    self.stats.commands_failed += 1;
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }

    /// Execute a Redis command
    pub async fn execute_command(&mut self, cmd: &str, args: &[&str]) -> AppResult<String> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            let mut redis_cmd = redis::cmd(cmd);
            for arg in args {
                redis_cmd.arg(*arg);
            }

            match redis_cmd.query_async::<_, redis::Value>(conn).await {
                Ok(value) => {
                    self.stats.commands_executed += 1;
                    Ok(self.format_redis_value(value))
                }
                Err(err) => {
                    self.stats.commands_failed += 1;
                    if err.is_connection_dropped() {
                        self.status = ConnectionStatus::Lost;
                    }
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }

    /// Format Redis value for display
    fn format_redis_value(&self, value: redis::Value) -> String {
        match value {
            redis::Value::Nil => "nil".to_string(),
            redis::Value::Int(i) => i.to_string(),
            redis::Value::Data(bytes) => {
                String::from_utf8(bytes).unwrap_or_else(|_| "<binary data>".to_string())
            }
            redis::Value::Bulk(values) => {
                let formatted: Vec<String> = values
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| format!("{}) {}", i + 1, self.format_redis_value(v)))
                    .collect();
                formatted.join("\n")
            }
            redis::Value::Status(status) => status,
            redis::Value::Okay => "OK".to_string(),
        }
    }

    /// Get server information
    pub async fn get_server_info(&mut self) -> AppResult<String> {
        self.execute_command("INFO", &[]).await
    }

    /// Get database size
    pub async fn get_database_size(&mut self) -> AppResult<usize> {
        let result = self.execute_command("DBSIZE", &[]).await?;
        result.parse().map_err(|_| {
            AppError::Generic("Failed to parse database size".to_string())
        })
    }

    /// Select database
    pub async fn select_database(&mut self, db: u8) -> AppResult<()> {
        self.execute_command("SELECT", &[&db.to_string()]).await?;
        Ok(())
    }

    /// Scan keys with pattern
    pub async fn scan_keys(&mut self, cursor: u64, pattern: &str, count: usize) -> AppResult<(u64, Vec<String>)> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            let args = vec![
                cursor.to_string(),
                "MATCH".to_string(),
                pattern.to_string(),
                "COUNT".to_string(),
                count.to_string(),
            ];
            
            let mut redis_cmd = redis::cmd("SCAN");
            for arg in &args {
                redis_cmd.arg(arg);
            }

            match redis_cmd.query_async::<_, redis::Value>(conn).await {
                Ok(redis::Value::Bulk(mut values)) if values.len() == 2 => {
                    // Extract cursor
                    let new_cursor = match values.remove(0) {
                        redis::Value::Data(bytes) => {
                            String::from_utf8(bytes).unwrap_or_default().parse().unwrap_or(0)
                        }
                        redis::Value::Int(i) => i as u64,
                        _ => 0,
                    };
                    
                    // Extract keys
                    let keys = match values.remove(0) {
                        redis::Value::Bulk(key_values) => {
                            key_values.into_iter().filter_map(|v| match v {
                                redis::Value::Data(bytes) => String::from_utf8(bytes).ok(),
                                _ => None,
                            }).collect()
                        }
                        _ => Vec::new(),
                    };
                    
                    self.stats.commands_executed += 1;
                    Ok((new_cursor, keys))
                }
                Ok(_) => {
                    self.stats.commands_executed += 1;
                    Ok((0, Vec::new()))
                }
                Err(err) => {
                    self.stats.commands_failed += 1;
                    if err.is_connection_dropped() {
                        self.status = ConnectionStatus::Lost;
                    }
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }
    
    /// Get list of available databases
    pub async fn get_databases(&mut self) -> AppResult<Vec<u8>> {
        // Get CONFIG GET databases to find max databases
        match self.execute_command("CONFIG", &["GET", "databases"]).await {
            Ok(result) => {
                // Parse the result to get database count
                let lines: Vec<&str> = result.lines().collect();
                let db_count = if lines.len() >= 2 {
                    lines[1].parse().unwrap_or(16)
                } else {
                    16
                };
                Ok((0..db_count).collect())
            }
            Err(_) => {
                // Fallback: assume standard 16 databases
                Ok((0..16).collect())
            }
        }
    }
    
    /// Get key type
    pub async fn get_key_type(&mut self, key: &str) -> AppResult<String> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            match redis::cmd("TYPE").arg(key).query_async::<_, String>(conn).await {
                Ok(result) => {
                    self.stats.commands_executed += 1;
                    Ok(result)
                }
                Err(err) => {
                    self.stats.commands_failed += 1;
                    if err.is_connection_dropped() {
                        self.status = ConnectionStatus::Lost;
                    }
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }
    
    /// Get key TTL
    pub async fn get_key_ttl(&mut self, key: &str) -> AppResult<i64> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            match redis::cmd("TTL").arg(key).query_async::<_, i64>(conn).await {
                Ok(result) => {
                    self.stats.commands_executed += 1;
                    Ok(result)
                }
                Err(err) => {
                    self.stats.commands_failed += 1;
                    if err.is_connection_dropped() {
                        self.status = ConnectionStatus::Lost;
                    }
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }
    
    /// Delete a key
    pub async fn delete_key(&mut self, key: &str) -> AppResult<String> {
        self.execute_command("DEL", &[key]).await
    }
    
    /// Rename a key
    pub async fn rename_key(&mut self, old_key: &str, new_key: &str) -> AppResult<String> {
        self.execute_command("RENAME", &[old_key, new_key]).await
    }
    
    /// Set key expiration
    pub async fn expire_key(&mut self, key: &str, seconds: u64) -> AppResult<String> {
        self.execute_command("EXPIRE", &[key, &seconds.to_string()]).await
    }
    
    /// Get key value (for string keys)
    pub async fn get_string_value(&mut self, key: &str) -> AppResult<String> {
        self.execute_command("GET", &[key]).await
    }
    
    /// Check if key exists
    pub async fn key_exists(&mut self, key: &str) -> AppResult<bool> {
        let result = self.execute_command("EXISTS", &[key]).await?;
        Ok(result.trim() == "1")
    }
    
    /// Get key information (type, TTL, size) for multiple keys efficiently
    pub async fn get_keys_info(&mut self, keys: &[String]) -> AppResult<Vec<(String, Option<String>, Option<i64>)>> {
        let mut results = Vec::new();
        
        for key in keys {
            let key_type = match self.get_key_type(key).await {
                Ok(t) => {
                    let t = t.trim();
                    if t == "none" {
                        None // Key doesn't exist
                    } else {
                        Some(t.to_string())
                    }
                }
                Err(_) => None,
            };
            
            let ttl = match self.get_key_ttl(key).await {
                Ok(t) => {
                    match t {
                        -2 => None, // Key doesn't exist
                        -1 => Some(-1), // No expiry
                        ttl if ttl > 0 => Some(ttl), // Has expiry
                        _ => None,
                    }
                }
                Err(_) => None,
            };
            
            results.push((key.clone(), key_type, ttl));
        }
        
        Ok(results)
    }
    
    /// Get memory usage of a key (Redis 4.0+)
    pub async fn get_key_memory_usage(&mut self, key: &str) -> AppResult<Option<usize>> {
        match self.execute_command("MEMORY", &["USAGE", key]).await {
            Ok(result) => {
                match result.trim().parse::<usize>() {
                    Ok(size) => Ok(Some(size)),
                    Err(_) => Ok(None),
                }
            }
            Err(_) => Ok(None), // Command might not be available in older Redis versions
        }
    }

    // ===== Phase 3: Data Type Value Retrieval Methods =====

    /// Get string value
    pub async fn get_string(&mut self, key: &str) -> AppResult<String> {
        self.execute_command("GET", &[key]).await
    }

    /// Set string value
    pub async fn set_string(&mut self, key: &str, value: &str) -> AppResult<String> {
        self.execute_command("SET", &[key, value]).await
    }

    /// Get all hash fields and values
    pub async fn get_hash_all(&mut self, key: &str) -> AppResult<Vec<(String, String)>> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            match redis::cmd("HGETALL").arg(key).query_async::<_, Vec<String>>(conn).await {
                Ok(result) => {
                    self.stats.commands_executed += 1;
                    // Convert flat array to key-value pairs
                    let mut pairs = Vec::new();
                    for chunk in result.chunks(2) {
                        if chunk.len() == 2 {
                            pairs.push((chunk[0].clone(), chunk[1].clone()));
                        }
                    }
                    Ok(pairs)
                }
                Err(err) => {
                    self.stats.commands_failed += 1;
                    if err.is_connection_dropped() {
                        self.status = ConnectionStatus::Lost;
                    }
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }

    /// Get hash field value
    pub async fn get_hash_field(&mut self, key: &str, field: &str) -> AppResult<Option<String>> {
        match self.execute_command("HGET", &[key, field]).await {
            Ok(result) => {
                if result.trim().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(result))
                }
            }
            Err(_) => Ok(None),
        }
    }

    /// Set hash field value
    pub async fn set_hash_field(&mut self, key: &str, field: &str, value: &str) -> AppResult<String> {
        self.execute_command("HSET", &[key, field, value]).await
    }

    /// Delete hash field
    pub async fn delete_hash_field(&mut self, key: &str, field: &str) -> AppResult<String> {
        self.execute_command("HDEL", &[key, field]).await
    }

    /// Get list elements with pagination
    pub async fn get_list_range(&mut self, key: &str, start: i64, stop: i64) -> AppResult<Vec<String>> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            match redis::cmd("LRANGE")
                .arg(key)
                .arg(start)
                .arg(stop)
                .query_async::<_, Vec<String>>(conn).await 
            {
                Ok(result) => {
                    self.stats.commands_executed += 1;
                    Ok(result)
                }
                Err(err) => {
                    self.stats.commands_failed += 1;
                    if err.is_connection_dropped() {
                        self.status = ConnectionStatus::Lost;
                    }
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }

    /// Get list length
    pub async fn get_list_length(&mut self, key: &str) -> AppResult<usize> {
        match self.execute_command("LLEN", &[key]).await {
            Ok(result) => {
                Ok(result.trim().parse().unwrap_or(0))
            }
            Err(err) => Err(err),
        }
    }

    /// Add element to list end
    pub async fn list_push(&mut self, key: &str, value: &str) -> AppResult<String> {
        self.execute_command("RPUSH", &[key, value]).await
    }

    /// Remove list element by index
    pub async fn list_set(&mut self, key: &str, index: i64, value: &str) -> AppResult<String> {
        self.execute_command("LSET", &[key, &index.to_string(), value]).await
    }

    /// Get set members with pagination
    pub async fn get_set_members(&mut self, key: &str, cursor: u64, count: usize) -> AppResult<(u64, Vec<String>)> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            let args = vec![
                key.to_string(),
                cursor.to_string(),
                "COUNT".to_string(),
                count.to_string(),
            ];
            
            let mut redis_cmd = redis::cmd("SSCAN");
            for arg in &args {
                redis_cmd.arg(arg);
            }

            match redis_cmd.query_async::<_, redis::Value>(conn).await {
                Ok(redis::Value::Bulk(mut values)) if values.len() == 2 => {
                    // Extract cursor
                    let new_cursor = match values.remove(0) {
                        redis::Value::Data(bytes) => {
                            String::from_utf8(bytes).unwrap_or_default().parse().unwrap_or(0)
                        }
                        redis::Value::Int(i) => i as u64,
                        _ => 0,
                    };
                    
                    // Extract members
                    let members = match values.remove(0) {
                        redis::Value::Bulk(member_values) => {
                            member_values.into_iter().filter_map(|v| match v {
                                redis::Value::Data(bytes) => String::from_utf8(bytes).ok(),
                                _ => None,
                            }).collect()
                        }
                        _ => Vec::new(),
                    };
                    
                    self.stats.commands_executed += 1;
                    Ok((new_cursor, members))
                }
                Ok(_) => {
                    self.stats.commands_executed += 1;
                    Ok((0, Vec::new()))
                }
                Err(err) => {
                    self.stats.commands_failed += 1;
                    if err.is_connection_dropped() {
                        self.status = ConnectionStatus::Lost;
                    }
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }

    /// Add member to set
    pub async fn set_add(&mut self, key: &str, member: &str) -> AppResult<String> {
        self.execute_command("SADD", &[key, member]).await
    }

    /// Remove member from set
    pub async fn set_remove(&mut self, key: &str, member: &str) -> AppResult<String> {
        self.execute_command("SREM", &[key, member]).await
    }

    /// Get sorted set members with scores
    pub async fn get_zset_range_with_scores(&mut self, key: &str, start: i64, stop: i64) -> AppResult<Vec<(String, f64)>> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            match redis::cmd("ZRANGE")
                .arg(key)
                .arg(start)
                .arg(stop)
                .arg("WITHSCORES")
                .query_async::<_, Vec<redis::Value>>(conn).await 
            {
                Ok(result) => {
                    self.stats.commands_executed += 1;
                    let mut members = Vec::new();
                    
                    // Parse alternating member/score pairs
                    for chunk in result.chunks(2) {
                        if chunk.len() == 2 {
                            let member = match &chunk[0] {
                                redis::Value::Data(bytes) => String::from_utf8(bytes.clone()).unwrap_or_default(),
                                redis::Value::Data(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_default(),
                                _ => continue,
                            };
                            
                            let score = match &chunk[1] {
                                redis::Value::Data(bytes) => {
                                    String::from_utf8(bytes.clone()).unwrap_or_default().parse().unwrap_or(0.0)
                                }
                                redis::Value::Data(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_default().parse().unwrap_or(0.0),
                                redis::Value::Int(i) => *i as f64,
                                _ => 0.0,
                            };
                            
                            members.push((member, score));
                        }
                    }
                    
                    Ok(members)
                }
                Err(err) => {
                    self.stats.commands_failed += 1;
                    if err.is_connection_dropped() {
                        self.status = ConnectionStatus::Lost;
                    }
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }

    /// Add member to sorted set with score
    pub async fn zset_add(&mut self, key: &str, score: f64, member: &str) -> AppResult<String> {
        self.execute_command("ZADD", &[key, &score.to_string(), member]).await
    }

    /// Remove member from sorted set
    pub async fn zset_remove(&mut self, key: &str, member: &str) -> AppResult<String> {
        self.execute_command("ZREM", &[key, member]).await
    }

    /// Get stream entries
    pub async fn get_stream_range(&mut self, key: &str, start: &str, end: &str, count: Option<usize>) -> AppResult<Vec<StreamEntry>> {
        if !self.is_connected() {
            return Err(AppError::Generic("Not connected to Redis".to_string()));
        }

        let mut conn_guard = self.connection.lock().await;
        if let Some(ref mut conn) = *conn_guard {
            let mut cmd = redis::cmd("XRANGE");
            cmd.arg(key).arg(start).arg(end);
            
            if let Some(c) = count {
                cmd.arg("COUNT").arg(c);
            }

            match cmd.query_async::<_, redis::Value>(conn).await {
                Ok(redis::Value::Bulk(entries)) => {
                    self.stats.commands_executed += 1;
                    let mut stream_entries = Vec::new();
                    
                    for entry in entries {
                        if let redis::Value::Bulk(entry_data) = entry {
                            if entry_data.len() >= 2 {
                                let id = match &entry_data[0] {
                                    redis::Value::Data(bytes) => String::from_utf8(bytes.clone()).unwrap_or_default(),
                                    redis::Value::Data(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_default(),
                                    _ => continue,
                                };
                                
                                let mut fields = Vec::new();
                                if let redis::Value::Bulk(field_data) = &entry_data[1] {
                                    for chunk in field_data.chunks(2) {
                                        if chunk.len() == 2 {
                                            let field = match &chunk[0] {
                                                redis::Value::Data(bytes) => String::from_utf8(bytes.clone()).unwrap_or_default(),
                                                redis::Value::Data(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_default(),
                                                _ => continue,
                                            };
                                            
                                            let value = match &chunk[1] {
                                                redis::Value::Data(bytes) => String::from_utf8(bytes.clone()).unwrap_or_default(),
                                                redis::Value::Data(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_default(),
                                                _ => String::new(),
                                            };
                                            
                                            fields.push((field, value));
                                        }
                                    }
                                }
                                
                                stream_entries.push(StreamEntry { id, fields });
                            }
                        }
                    }
                    
                    Ok(stream_entries)
                }
                Ok(_) => {
                    self.stats.commands_executed += 1;
                    Ok(Vec::new())
                }
                Err(err) => {
                    self.stats.commands_failed += 1;
                    if err.is_connection_dropped() {
                        self.status = ConnectionStatus::Lost;
                    }
                    Err(AppError::Redis(err))
                }
            }
        } else {
            self.status = ConnectionStatus::Lost;
            Err(AppError::Generic("Connection lost".to_string()))
        }
    }
}

/// Represents a Redis stream entry
#[derive(Debug, Clone)]
pub struct StreamEntry {
    pub id: String,
    pub fields: Vec<(String, String)>,
}

// Additional methods for bulk operations
impl RedisConnection {
    /// Get current database number
    pub fn get_current_database(&self) -> u8 {
        self.config.database
    }
    
    /// Delete a key (bulk operation variant)
    pub async fn bulk_delete_key(&mut self, key: &str) -> AppResult<()> {
        self.execute_command("DEL", &[key]).await?;
        Ok(())
    }
    
    /// Set TTL for a key
    pub async fn set_ttl(&mut self, key: &str, ttl: i64) -> AppResult<()> {
        self.execute_command("EXPIRE", &[key, &ttl.to_string()]).await?;
        Ok(())
    }
    
    /// Remove TTL (make key persistent)
    pub async fn persist_key(&mut self, key: &str) -> AppResult<()> {
        self.execute_command("PERSIST", &[key]).await?;
        Ok(())
    }
    
    /// Rename a key (bulk operation variant)
    pub async fn bulk_rename_key(&mut self, old_key: &str, new_key: &str) -> AppResult<()> {
        self.execute_command("RENAME", &[old_key, new_key]).await?;
        Ok(())
    }
    
    /// Set string value (bulk operation variant)
    pub async fn bulk_set_string(&mut self, key: &str, value: &str) -> AppResult<()> {
        self.execute_command("SET", &[key, value]).await?;
        Ok(())
    }
    
    /// Increment a numeric key
    pub async fn increment_key(&mut self, key: &str, amount: i64) -> AppResult<()> {
        self.execute_command("INCRBY", &[key, &amount.to_string()]).await?;
        Ok(())
    }
    
    /// Append to a string key
    pub async fn append_to_string(&mut self, key: &str, value: &str) -> AppResult<()> {
        self.execute_command("APPEND", &[key, value]).await?;
        Ok(())
    }
    
    /// Add member to set
    pub async fn add_to_set(&mut self, key: &str, member: &str) -> AppResult<()> {
        self.execute_command("SADD", &[key, member]).await?;
        Ok(())
    }
    
    /// Add member to sorted set
    pub async fn add_to_sorted_set(&mut self, key: &str, member: &str, score: f64) -> AppResult<()> {
        self.execute_command("ZADD", &[key, &score.to_string(), member]).await?;
        Ok(())
    }
    
    /// Set hash field (bulk operation variant)
    pub async fn bulk_set_hash_field(&mut self, key: &str, field: &str, value: &str) -> AppResult<()> {
        self.execute_command("HSET", &[key, field, value]).await?;
        Ok(())
    }
    
    /// Push to list
    pub async fn push_to_list(&mut self, key: &str, value: &str, to_front: bool) -> AppResult<()> {
        let cmd = if to_front { "LPUSH" } else { "RPUSH" };
        self.execute_command(cmd, &[key, value]).await?;
        Ok(())
    }
}