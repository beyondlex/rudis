use serde::{Deserialize, Serialize};

/// Represents the value of a Redis key with its type-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedisValue {
    /// String value
    String(String),
    /// Hash value with field-value pairs
    Hash(Vec<(String, String)>),
    /// List value with indexed elements
    List(Vec<String>),
    /// Set value with unique members
    Set(Vec<String>),
    /// Sorted set value with scored members
    ZSet(Vec<(String, f64)>),
    /// Stream value with entries
    Stream(Vec<StreamEntry>),
    /// Unknown or unsupported type
    Unknown(String),
}

impl RedisValue {
    /// Get the Redis type name for this value
    pub fn type_name(&self) -> &'static str {
        match self {
            RedisValue::String(_) => "string",
            RedisValue::Hash(_) => "hash",
            RedisValue::List(_) => "list",
            RedisValue::Set(_) => "set",
            RedisValue::ZSet(_) => "zset",
            RedisValue::Stream(_) => "stream",
            RedisValue::Unknown(_) => "unknown",
        }
    }

    /// Get a human-readable description of the value
    pub fn description(&self) -> String {
        match self {
            RedisValue::String(s) => {
                if s.len() <= 100 {
                    format!("String: \"{}\"", s)
                } else {
                    format!("String: \"{}...\" ({} chars)", &s[..97], s.len())
                }
            }
            RedisValue::Hash(fields) => {
                format!("Hash: {} fields", fields.len())
            }
            RedisValue::List(elements) => {
                format!("List: {} elements", elements.len())
            }
            RedisValue::Set(members) => {
                format!("Set: {} members", members.len())
            }
            RedisValue::ZSet(members) => {
                format!("Sorted Set: {} members", members.len())
            }
            RedisValue::Stream(entries) => {
                format!("Stream: {} entries", entries.len())
            }
            RedisValue::Unknown(type_name) => {
                format!("Unknown type: {}", type_name)
            }
        }
    }

    /// Check if the value is empty
    pub fn is_empty(&self) -> bool {
        match self {
            RedisValue::String(s) => s.is_empty(),
            RedisValue::Hash(fields) => fields.is_empty(),
            RedisValue::List(elements) => elements.is_empty(),
            RedisValue::Set(members) => members.is_empty(),
            RedisValue::ZSet(members) => members.is_empty(),
            RedisValue::Stream(entries) => entries.is_empty(),
            RedisValue::Unknown(_) => true,
        }
    }

    /// Get the size/length of the value
    pub fn size(&self) -> usize {
        match self {
            RedisValue::String(s) => s.len(),
            RedisValue::Hash(fields) => fields.len(),
            RedisValue::List(elements) => elements.len(),
            RedisValue::Set(members) => members.len(),
            RedisValue::ZSet(members) => members.len(),
            RedisValue::Stream(entries) => entries.len(),
            RedisValue::Unknown(_) => 0,
        }
    }
}

/// Represents a Redis stream entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEntry {
    /// Entry ID (timestamp-sequence)
    pub id: String,
    /// Field-value pairs in the entry
    pub fields: Vec<(String, String)>,
}

impl StreamEntry {
    /// Format the entry ID for display
    pub fn formatted_id(&self) -> String {
        // Parse timestamp from ID and format nicely
        if let Some(dash_pos) = self.id.find('-') {
            let timestamp_str = &self.id[..dash_pos];
            if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                // Convert milliseconds to seconds for display
                let seconds = timestamp / 1000;
                let dt = chrono::DateTime::from_timestamp(seconds as i64, 0);
                if let Some(dt) = dt {
                    return format!("{} ({})", self.id, dt.format("%Y-%m-%d %H:%M:%S"));
                }
            }
        }
        self.id.clone()
    }
}

/// Value pagination information for large datasets
#[derive(Debug, Clone)]
pub struct ValuePage {
    /// Current page number (0-based)
    pub page: usize,
    /// Number of items per page
    pub page_size: usize,
    /// Total number of items
    pub total_items: usize,
    /// Whether there are more pages
    pub has_more: bool,
}

impl ValuePage {
    /// Create a new value page
    pub fn new(page: usize, page_size: usize, total_items: usize) -> Self {
        let has_more = (page + 1) * page_size < total_items;
        Self {
            page,
            page_size,
            total_items,
            has_more,
        }
    }

    /// Get the start index for this page
    pub fn start_index(&self) -> usize {
        self.page * self.page_size
    }

    /// Get the end index for this page
    pub fn end_index(&self) -> usize {
        ((self.page + 1) * self.page_size).min(self.total_items)
    }

    /// Get total number of pages
    pub fn total_pages(&self) -> usize {
        (self.total_items + self.page_size - 1) / self.page_size
    }
}

/// Configuration for value display and editing
#[derive(Debug, Clone)]
pub struct ValueDisplayConfig {
    /// Maximum number of items to show per page
    pub page_size: usize,
    /// Maximum length of string values to display inline
    pub max_inline_length: usize,
    /// Whether to show binary data as hex
    pub show_binary_as_hex: bool,
    /// Whether to enable JSON syntax highlighting
    pub enable_json_highlighting: bool,
}

impl Default for ValueDisplayConfig {
    fn default() -> Self {
        Self {
            page_size: 50,
            max_inline_length: 200,
            show_binary_as_hex: true,
            enable_json_highlighting: true,
        }
    }
}

/// Helper functions for value formatting and display
pub mod formatting {
    use super::*;

    /// Format a string value for display, handling special characters and length
    pub fn format_string_value(value: &str, max_length: usize) -> String {
        let display_value = if value.len() > max_length {
            format!("{}...", &value[..max_length.saturating_sub(3)])
        } else {
            value.to_string()
        };

        // Escape special characters for display
        display_value
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Check if a string value looks like JSON
    pub fn is_json_like(value: &str) -> bool {
        let trimmed = value.trim();
        (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    }

    /// Check if a string value contains binary data
    pub fn is_binary_data(value: &str) -> bool {
        value.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    }

    /// Format binary data as hex string
    pub fn format_as_hex(data: &[u8]) -> String {
        data.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .chunks(16)
            .map(|chunk| chunk.join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    }
}