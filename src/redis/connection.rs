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
    pub client: Client,
    
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
        let client = Client::open(connection_url)
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
        let args = vec![
            cursor.to_string(),
            "MATCH".to_string(),
            pattern.to_string(),
            "COUNT".to_string(),
            count.to_string(),
        ];
        
        let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let result = self.execute_command("SCAN", &cmd_args).await?;
        
        // Parse SCAN result (cursor, [keys])
        let lines: Vec<&str> = result.lines().collect();
        if lines.len() >= 2 {
            let new_cursor = lines[0].parse().unwrap_or(0);
            let keys: Vec<String> = lines[1..].iter().map(|s| s.to_string()).collect();
            Ok((new_cursor, keys))
        } else {
            Ok((0, Vec::new()))
        }
    }
}