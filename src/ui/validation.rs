use std::fmt;

/// Validation result for string values
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Valid,
    Warning(String),
    Error(String),
}

impl ValidationResult {
    /// Check if the validation result is valid
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }
    
    /// Check if the validation result has warnings
    pub fn has_warning(&self) -> bool {
        matches!(self, ValidationResult::Warning(_))
    }
    
    /// Check if the validation result has errors
    pub fn has_error(&self) -> bool {
        matches!(self, ValidationResult::Error(_))
    }
    
    /// Get the message if any
    pub fn message(&self) -> Option<&str> {
        match self {
            ValidationResult::Valid => None,
            ValidationResult::Warning(msg) | ValidationResult::Error(msg) => Some(msg),
        }
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationResult::Valid => write!(f, "Valid"),
            ValidationResult::Warning(msg) => write!(f, "Warning: {}", msg),
            ValidationResult::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// String value validator
pub struct StringValidator {
    /// Maximum allowed size in bytes
    pub max_size: usize,
    /// Maximum allowed lines
    pub max_lines: usize,
    /// Whether to validate UTF-8 encoding
    pub require_utf8: bool,
}

impl Default for StringValidator {
    fn default() -> Self {
        Self {
            max_size: 512 * 1024 * 1024, // 512MB default Redis string limit
            max_lines: 1000,
            require_utf8: true,
        }
    }
}

impl StringValidator {
    /// Create a new string validator with custom limits
    pub fn new(max_size: usize, max_lines: usize, require_utf8: bool) -> Self {
        Self {
            max_size,
            max_lines,
            require_utf8,
        }
    }
    
    /// Validate a string value
    pub fn validate(&self, value: &str) -> ValidationResult {
        // Check UTF-8 encoding
        if self.require_utf8 && !value.is_utf8() {
            return ValidationResult::Error("Invalid UTF-8 encoding".to_string());
        }
        
        // Check size limit
        if value.len() > self.max_size {
            return ValidationResult::Error(format!(
                "Value too large: {} bytes (max: {} bytes)",
                value.len(),
                self.max_size
            ));
        }
        
        // Check line count
        let line_count = value.lines().count();
        if line_count > self.max_lines {
            return ValidationResult::Error(format!(
                "Too many lines: {} (max: {})",
                line_count,
                self.max_lines
            ));
        }
        
        // Check for warnings
        let mut warnings = Vec::new();
        
        // Large size warning
        if value.len() > 1024 * 1024 {
            warnings.push(format!("Large value: {} bytes", value.len()));
        }
        
        // Many lines warning
        if line_count > 100 {
            warnings.push(format!("Many lines: {}", line_count));
        }
        
        // Binary data warning (if contains null bytes or non-printable chars)
        if self.contains_binary_data(value) {
            warnings.push("Contains binary data".to_string());
        }
        
        // Control characters warning
        if self.contains_control_chars(value) {
            warnings.push("Contains control characters".to_string());
        }
        
        if !warnings.is_empty() {
            ValidationResult::Warning(warnings.join("; "))
        } else {
            ValidationResult::Valid
        }
    }
    
    /// Check if string contains binary data
    fn contains_binary_data(&self, value: &str) -> bool {
        value.bytes().any(|b| b == 0 || (b < 32 && b != b'\t' && b != b'\n' && b != b'\r'))
    }
    
    /// Check if string contains control characters
    fn contains_control_chars(&self, value: &str) -> bool {
        value.chars().any(|c| c.is_control() && c != '\t' && c != '\n' && c != '\r')
    }
}

/// Helper trait for UTF-8 validation
trait Utf8Validator {
    fn is_utf8(&self) -> bool;
}

impl Utf8Validator for str {
    fn is_utf8(&self) -> bool {
        // str is always valid UTF-8 in Rust
        true
    }
}

impl Utf8Validator for [u8] {
    fn is_utf8(&self) -> bool {
        std::str::from_utf8(self).is_ok()
    }
}

/// JSON value validator
pub struct JsonValidator;

impl JsonValidator {
    /// Validate if string is valid JSON
    pub fn validate_json(value: &str) -> ValidationResult {
        match serde_json::from_str::<serde_json::Value>(value) {
            Ok(_) => ValidationResult::Valid,
            Err(e) => ValidationResult::Error(format!("Invalid JSON: {}", e)),
        }
    }
    
    /// Check if string looks like JSON
    pub fn is_json_like(value: &str) -> bool {
        let trimmed = value.trim();
        (trimmed.starts_with('{') && trimmed.ends_with('}')) ||
        (trimmed.starts_with('[') && trimmed.ends_with(']'))
    }
    
    /// Validate JSON and return detailed information
    pub fn validate_json_detailed(value: &str) -> (ValidationResult, Option<JsonInfo>) {
        if !Self::is_json_like(value) {
            return (ValidationResult::Valid, None);
        }
        
        match serde_json::from_str::<serde_json::Value>(value) {
            Ok(json_value) => {
                let info = JsonInfo::from_value(&json_value);
                (ValidationResult::Valid, Some(info))
            }
            Err(e) => (
                ValidationResult::Error(format!("Invalid JSON: {}", e)),
                None
            )
        }
    }
}

/// Information about JSON content
#[derive(Debug, Clone)]
pub struct JsonInfo {
    pub value_type: JsonValueType,
    pub key_count: Option<usize>,
    pub array_length: Option<usize>,
    pub nesting_level: usize,
}

#[derive(Debug, Clone)]
pub enum JsonValueType {
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
}

impl JsonInfo {
    fn from_value(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Object(map) => JsonInfo {
                value_type: JsonValueType::Object,
                key_count: Some(map.len()),
                array_length: None,
                nesting_level: Self::calculate_nesting_level(value),
            },
            serde_json::Value::Array(arr) => JsonInfo {
                value_type: JsonValueType::Array,
                key_count: None,
                array_length: Some(arr.len()),
                nesting_level: Self::calculate_nesting_level(value),
            },
            serde_json::Value::String(_) => JsonInfo {
                value_type: JsonValueType::String,
                key_count: None,
                array_length: None,
                nesting_level: 0,
            },
            serde_json::Value::Number(_) => JsonInfo {
                value_type: JsonValueType::Number,
                key_count: None,
                array_length: None,
                nesting_level: 0,
            },
            serde_json::Value::Bool(_) => JsonInfo {
                value_type: JsonValueType::Boolean,
                key_count: None,
                array_length: None,
                nesting_level: 0,
            },
            serde_json::Value::Null => JsonInfo {
                value_type: JsonValueType::Null,
                key_count: None,
                array_length: None,
                nesting_level: 0,
            },
        }
    }
    
    fn calculate_nesting_level(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Object(map) => {
                1 + map.values()
                    .map(Self::calculate_nesting_level)
                    .max()
                    .unwrap_or(0)
            }
            serde_json::Value::Array(arr) => {
                1 + arr.iter()
                    .map(Self::calculate_nesting_level)
                    .max()
                    .unwrap_or(0)
            }
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_validation() {
        let validator = StringValidator::default();
        
        // Valid string
        assert!(validator.validate("Hello, World!").is_valid());
        
        // Large string warning
        let large_string = "a".repeat(2 * 1024 * 1024);
        assert!(validator.validate(&large_string).has_warning());
        
        // Too large string
        let validator_small = StringValidator::new(100, 10, true);
        assert!(validator_small.validate(&large_string).has_error());
    }
    
    #[test]
    fn test_json_validation() {
        // Valid JSON
        assert!(JsonValidator::validate_json(r#"{"key": "value"}"#).is_valid());
        
        // Invalid JSON
        assert!(JsonValidator::validate_json(r#"{"key": value}"#).has_error());
        
        // Not JSON-like
        assert!(JsonValidator::validate_json("Hello, World!").is_valid());
    }
}