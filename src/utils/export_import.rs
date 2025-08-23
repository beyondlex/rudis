use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::redis::value_types::RedisValue;
use crate::error::AppResult;

/// Supported export/import formats
#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    /// JSON format (human-readable)
    Json,
    /// YAML format (human-readable)
    Yaml,
    /// CSV format (for tabular data like hashes)
    Csv,
    /// Raw binary data
    Raw,
    /// Redis protocol format (RESP)
    Redis,
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportFormat::Json => write!(f, "JSON"),
            ExportFormat::Yaml => write!(f, "YAML"),
            ExportFormat::Csv => write!(f, "CSV"),
            ExportFormat::Raw => write!(f, "Raw"),
            ExportFormat::Redis => write!(f, "Redis"),
        }
    }
}

/// Export data structure for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    /// Metadata about the export
    pub metadata: ExportMetadata,
    /// Exported values
    pub values: Vec<ExportedValue>,
}

/// Metadata for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    /// Export timestamp
    pub timestamp: String,
    /// Redis database number
    pub database: u8,
    /// Export format
    pub format: String,
    /// Total number of keys exported
    pub key_count: usize,
    /// Export tool version
    pub version: String,
}

/// Individual exported value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedValue {
    /// Key name
    pub key: String,
    /// Value type
    pub value_type: String,
    /// Value data (format depends on type)
    pub value: serde_json::Value,
    /// TTL in seconds (-1 for no expiry)
    pub ttl: Option<i64>,
    /// Key metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// Export/Import manager
pub struct DataExporter;

impl DataExporter {
    /// Export a single Redis value to the specified format
    pub fn export_value(
        key: &str,
        value: &RedisValue,
        ttl: Option<i64>,
        format: &ExportFormat,
    ) -> AppResult<String> {
        let exported_value = Self::convert_to_exported_value(key, value, ttl)?;
        
        match format {
            ExportFormat::Json => Self::export_to_json(&[exported_value]),
            ExportFormat::Yaml => Self::export_to_yaml(&[exported_value]),
            ExportFormat::Csv => Self::export_to_csv(&[exported_value]),
            ExportFormat::Raw => Self::export_to_raw(&exported_value),
            ExportFormat::Redis => Self::export_to_redis(&exported_value),
        }
    }
    
    /// Export multiple Redis values
    pub fn export_values(
        values: &[(String, RedisValue, Option<i64>)],
        database: u8,
        format: &ExportFormat,
    ) -> AppResult<String> {
        let mut exported_values = Vec::new();
        
        for (key, value, ttl) in values {
            let exported_value = Self::convert_to_exported_value(key, value, *ttl)?;
            exported_values.push(exported_value);
        }
        
        match format {
            ExportFormat::Json => Self::export_to_json_with_metadata(&exported_values, database, format),
            ExportFormat::Yaml => Self::export_to_yaml_with_metadata(&exported_values, database, format),
            ExportFormat::Csv => Self::export_to_csv(&exported_values),
            ExportFormat::Raw => {
                // For raw format, concatenate all values
                let mut result = String::new();
                for value in &exported_values {
                    result.push_str(&Self::export_to_raw(value)?);
                    result.push('\n');
                }
                Ok(result)
            }
            ExportFormat::Redis => {
                let mut result = String::new();
                for value in &exported_values {
                    result.push_str(&Self::export_to_redis(value)?);
                    result.push('\n');
                }
                Ok(result)
            }
        }
    }
    
    /// Import data from string in the specified format
    pub fn import_data(data: &str, format: &ExportFormat) -> AppResult<Vec<ExportedValue>> {
        match format {
            ExportFormat::Json => Self::import_from_json(data),
            ExportFormat::Yaml => Self::import_from_yaml(data),
            ExportFormat::Csv => Self::import_from_csv(data),
            ExportFormat::Raw => Err(crate::error::AppError::Generic("Raw format import not supported".to_string())),
            ExportFormat::Redis => Self::import_from_redis(data),
        }
    }
    
    /// Convert Redis value to exportable format
    fn convert_to_exported_value(
        key: &str,
        value: &RedisValue,
        ttl: Option<i64>,
    ) -> AppResult<ExportedValue> {
        let (value_type, value_data) = match value {
            RedisValue::String(s) => ("string", serde_json::Value::String(s.clone())),
            RedisValue::Hash(fields) => {
                let map: HashMap<String, String> = fields.iter().cloned().collect();
                ("hash", serde_json::to_value(map)?)
            }
            RedisValue::List(elements) => ("list", serde_json::to_value(elements)?),
            RedisValue::Set(members) => ("set", serde_json::to_value(members)?),
            RedisValue::ZSet(members) => {
                let map: HashMap<String, f64> = members.iter()
                    .map(|(member, score)| (member.clone(), *score))
                    .collect();
                ("zset", serde_json::to_value(map)?)
            }
            RedisValue::Stream(entries) => {
                let stream_data: Vec<HashMap<String, serde_json::Value>> = entries.iter()
                    .map(|entry| {
                        let mut entry_map = HashMap::new();
                        entry_map.insert("id".to_string(), serde_json::Value::String(entry.id.clone()));
                        
                        let fields_map: HashMap<String, String> = entry.fields.iter().cloned().collect();
                        entry_map.insert("fields".to_string(), serde_json::to_value(fields_map).unwrap_or_default());
                        entry_map
                    })
                    .collect();
                ("stream", serde_json::to_value(stream_data)?)
            }
            RedisValue::Unknown(type_name) => {
                return Err(crate::error::AppError::Generic(format!("Cannot export unknown type: {}", type_name)));
            }
        };
        
        Ok(ExportedValue {
            key: key.to_string(),
            value_type: value_type.to_string(),
            value: value_data,
            ttl,
            metadata: None,
        })
    }
    
    /// Export to JSON format
    fn export_to_json(values: &[ExportedValue]) -> AppResult<String> {
        Ok(serde_json::to_string_pretty(values)?)
    }
    
    /// Export to JSON with metadata
    fn export_to_json_with_metadata(
        values: &[ExportedValue],
        database: u8,
        format: &ExportFormat,
    ) -> AppResult<String> {
        let export_data = ExportData {
            metadata: ExportMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                database,
                format: format.to_string(),
                key_count: values.len(),
                version: "rudis-0.1.0".to_string(),
            },
            values: values.to_vec(),
        };
        
        Ok(serde_json::to_string_pretty(&export_data)?)
    }
    
    /// Export to YAML format
    fn export_to_yaml(values: &[ExportedValue]) -> AppResult<String> {
        Ok(serde_yaml::to_string(values)?)
    }
    
    /// Export to YAML with metadata
    fn export_to_yaml_with_metadata(
        values: &[ExportedValue],
        database: u8,
        format: &ExportFormat,
    ) -> AppResult<String> {
        let export_data = ExportData {
            metadata: ExportMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                database,
                format: format.to_string(),
                key_count: values.len(),
                version: "rudis-0.1.0".to_string(),
            },
            values: values.to_vec(),
        };
        
        Ok(serde_yaml::to_string(&export_data)?)
    }
    
    /// Export to CSV format (mainly for hash-like data)
    fn export_to_csv(values: &[ExportedValue]) -> AppResult<String> {
        let mut csv_content = String::new();
        csv_content.push_str("key,type,field,value,ttl\n");
        
        for exported_value in values {
            match exported_value.value_type.as_str() {
                "string" => {
                    if let Some(string_value) = exported_value.value.as_str() {
                        csv_content.push_str(&format!(
                            "{},{},value,\"{}\",{}\n",
                            exported_value.key,
                            exported_value.value_type,
                            string_value.replace('"', "\"\""), // Escape quotes
                            exported_value.ttl.unwrap_or(-1)
                        ));
                    }
                }
                "hash" => {
                    if let Some(object) = exported_value.value.as_object() {
                        for (field, value) in object {
                            csv_content.push_str(&format!(
                                "{},{},{},\"{}\",{}\n",
                                exported_value.key,
                                exported_value.value_type,
                                field,
                                value.as_str().unwrap_or("").replace('"', "\"\""),
                                exported_value.ttl.unwrap_or(-1)
                            ));
                        }
                    }
                }
                "list" | "set" => {
                    if let Some(array) = exported_value.value.as_array() {
                        for (index, value) in array.iter().enumerate() {
                            csv_content.push_str(&format!(
                                "{},{},{},\"{}\",{}\n",
                                exported_value.key,
                                exported_value.value_type,
                                index,
                                value.as_str().unwrap_or("").replace('"', "\"\""),
                                exported_value.ttl.unwrap_or(-1)
                            ));
                        }
                    }
                }
                "zset" => {
                    if let Some(object) = exported_value.value.as_object() {
                        for (member, score) in object {
                            csv_content.push_str(&format!(
                                "{},{},{},{},{}\n",
                                exported_value.key,
                                exported_value.value_type,
                                member,
                                score.as_f64().unwrap_or(0.0),
                                exported_value.ttl.unwrap_or(-1)
                            ));
                        }
                    }
                }
                _ => {
                    csv_content.push_str(&format!(
                        "{},{},data,\"{}\",{}\n",
                        exported_value.key,
                        exported_value.value_type,
                        exported_value.value.to_string().replace('"', "\"\""),
                        exported_value.ttl.unwrap_or(-1)
                    ));
                }
            }
        }
        
        Ok(csv_content)
    }
    
    /// Export to raw format (just the value data)
    fn export_to_raw(value: &ExportedValue) -> AppResult<String> {
        match value.value_type.as_str() {
            "string" => Ok(value.value.as_str().unwrap_or("").to_string()),
            _ => Ok(value.value.to_string()),
        }
    }
    
    /// Export to Redis protocol format
    fn export_to_redis(value: &ExportedValue) -> AppResult<String> {
        let mut commands = Vec::new();
        
        match value.value_type.as_str() {
            "string" => {
                if let Some(string_value) = value.value.as_str() {
                    commands.push(format!("SET \"{}\" \"{}\"", value.key, string_value));
                }
            }
            "hash" => {
                if let Some(object) = value.value.as_object() {
                    for (field, field_value) in object {
                        commands.push(format!(
                            "HSET \"{}\" \"{}\" \"{}\"",
                            value.key,
                            field,
                            field_value.as_str().unwrap_or("")
                        ));
                    }
                }
            }
            "list" => {
                if let Some(array) = value.value.as_array() {
                    for element in array {
                        commands.push(format!(
                            "LPUSH \"{}\" \"{}\"",
                            value.key,
                            element.as_str().unwrap_or("")
                        ));
                    }
                }
            }
            "set" => {
                if let Some(array) = value.value.as_array() {
                    for member in array {
                        commands.push(format!(
                            "SADD \"{}\" \"{}\"",
                            value.key,
                            member.as_str().unwrap_or("")
                        ));
                    }
                }
            }
            "zset" => {
                if let Some(object) = value.value.as_object() {
                    for (member, score) in object {
                        commands.push(format!(
                            "ZADD \"{}\" {} \"{}\"",
                            value.key,
                            score.as_f64().unwrap_or(0.0),
                            member
                        ));
                    }
                }
            }
            _ => {
                return Err(crate::error::AppError::Generic(
                    format!("Redis export not supported for type: {}", value.value_type)
                ));
            }
        }
        
        // Add TTL command if needed
        if let Some(ttl) = value.ttl {
            if ttl > 0 {
                commands.push(format!("EXPIRE \"{}\" {}", value.key, ttl));
            }
        }
        
        Ok(commands.join("\n"))
    }
    
    /// Import from JSON format
    fn import_from_json(data: &str) -> AppResult<Vec<ExportedValue>> {
        // Try to parse as ExportData first, then fall back to array of ExportedValue
        if let Ok(export_data) = serde_json::from_str::<ExportData>(data) {
            Ok(export_data.values)
        } else {
            Ok(serde_json::from_str::<Vec<ExportedValue>>(data)?)
        }
    }
    
    /// Import from YAML format
    fn import_from_yaml(data: &str) -> AppResult<Vec<ExportedValue>> {
        // Try to parse as ExportData first, then fall back to array of ExportedValue
        if let Ok(export_data) = serde_yaml::from_str::<ExportData>(data) {
            Ok(export_data.values)
        } else {
            Ok(serde_yaml::from_str::<Vec<ExportedValue>>(data)?)
        }
    }
    
    /// Import from CSV format
    fn import_from_csv(data: &str) -> AppResult<Vec<ExportedValue>> {
        let mut reader = csv::Reader::from_reader(data.as_bytes());
        let mut values_map: HashMap<String, ExportedValue> = HashMap::new();
        
        for record in reader.records() {
            let record = record?;
            if record.len() < 5 {
                continue;
            }
            
            let key = record[0].to_string();
            let value_type = record[1].to_string();
            let field = record[2].to_string();
            let value = record[3].to_string();
            let ttl: Option<i64> = record[4].parse().ok().and_then(|t| if t == -1 { None } else { Some(t) });
            
            let entry = values_map.entry(key.clone()).or_insert_with(|| ExportedValue {
                key: key.clone(),
                value_type: value_type.clone(),
                value: serde_json::Value::Null,
                ttl,
                metadata: None,
            });
            
            // Build the value based on type
            match value_type.as_str() {
                "string" => {
                    entry.value = serde_json::Value::String(value);
                }
                "hash" => {
                    if entry.value.is_null() {
                        entry.value = serde_json::json!({});
                    }
                    if let Some(object) = entry.value.as_object_mut() {
                        object.insert(field, serde_json::Value::String(value));
                    }
                }
                "list" | "set" => {
                    if entry.value.is_null() {
                        entry.value = serde_json::Value::Array(Vec::new());
                    }
                    if let Some(array) = entry.value.as_array_mut() {
                        array.push(serde_json::Value::String(value));
                    }
                }
                "zset" => {
                    if entry.value.is_null() {
                        entry.value = serde_json::json!({});
                    }
                    if let Some(object) = entry.value.as_object_mut() {
                        if let Ok(score) = value.parse::<f64>() {
                            if let Some(number) = serde_json::Number::from_f64(score) {
                                object.insert(field, serde_json::Value::Number(number));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(values_map.into_values().collect())
    }
    
    /// Import from Redis protocol format
    fn import_from_redis(data: &str) -> AppResult<Vec<ExportedValue>> {
        // This is a simplified parser for basic Redis commands
        // In a full implementation, you might want to use a proper Redis protocol parser
        let mut values_map: HashMap<String, ExportedValue> = HashMap::new();
        
        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            Self::parse_redis_command(line, &mut values_map)?;
        }
        
        Ok(values_map.into_values().collect())
    }
    
    /// Parse a single Redis command
    fn parse_redis_command(
        command: &str,
        values_map: &mut HashMap<String, ExportedValue>,
    ) -> AppResult<()> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }
        
        match parts[0].to_uppercase().as_str() {
            "SET" if parts.len() >= 3 => {
                let key = parts[1].trim_matches('"');
                let value = parts[2..].join(" ").trim_matches('"').to_string();
                
                values_map.insert(key.to_string(), ExportedValue {
                    key: key.to_string(),
                    value_type: "string".to_string(),
                    value: serde_json::Value::String(value),
                    ttl: None,
                    metadata: None,
                });
            }
            "HSET" if parts.len() >= 4 => {
                let key = parts[1].trim_matches('"');
                let field = parts[2].trim_matches('"');
                let value = parts[3..].join(" ").trim_matches('"').to_string();
                
                let entry = values_map.entry(key.to_string()).or_insert_with(|| ExportedValue {
                    key: key.to_string(),
                    value_type: "hash".to_string(),
                    value: serde_json::json!({}),
                    ttl: None,
                    metadata: None,
                });
                
                if let Some(object) = entry.value.as_object_mut() {
                    object.insert(field.to_string(), serde_json::Value::String(value));
                }
            }
            // Add more command parsers as needed...
            _ => {}
        }
        
        Ok(())
    }
    
    /// Get supported formats for a given Redis value type
    pub fn get_supported_formats(value_type: &str) -> Vec<ExportFormat> {
        match value_type {
            "hash" => vec![
                ExportFormat::Json,
                ExportFormat::Yaml,
                ExportFormat::Csv,
                ExportFormat::Redis,
            ],
            "string" => vec![
                ExportFormat::Json,
                ExportFormat::Yaml,
                ExportFormat::Raw,
                ExportFormat::Redis,
            ],
            _ => vec![
                ExportFormat::Json,
                ExportFormat::Yaml,
                ExportFormat::Redis,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_string_export_import() {
        let value = RedisValue::String("Hello, World!".to_string());
        let exported = DataExporter::export_value("test:key", &value, None, &ExportFormat::Json).unwrap();
        
        let imported = DataExporter::import_data(&exported, &ExportFormat::Json).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].key, "test:key");
        assert_eq!(imported[0].value_type, "string");
    }
    
    #[test]
    fn test_hash_export_import() {
        let fields = vec![
            ("field1".to_string(), "value1".to_string()),
            ("field2".to_string(), "value2".to_string()),
        ];
        let value = RedisValue::Hash(fields);
        
        let exported = DataExporter::export_value("test:hash", &value, Some(3600), &ExportFormat::Json).unwrap();
        let imported = DataExporter::import_data(&exported, &ExportFormat::Json).unwrap();
        
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].value_type, "hash");
        assert_eq!(imported[0].ttl, Some(3600));
    }
}