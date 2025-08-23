use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use serde_json::Value;

/// JSON syntax highlighter with token-based highlighting
pub struct JsonHighlighter;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonToken {
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    String(String),
    Number(String),
    Boolean(String),
    Null,
    Whitespace(String),
    Invalid(String),
}

impl JsonHighlighter {
    /// Check if text looks like JSON
    pub fn is_json_like(text: &str) -> bool {
        let trimmed = text.trim();
        (trimmed.starts_with('{') && trimmed.ends_with('}')) ||
        (trimmed.starts_with('[') && trimmed.ends_with(']')) ||
        trimmed.starts_with('"') && trimmed.ends_with('"') ||
        trimmed.parse::<f64>().is_ok() ||
        trimmed == "true" || trimmed == "false" || trimmed == "null"
    }

    /// Validate and format JSON with proper indentation
    pub fn format_json(text: &str) -> Result<String, String> {
        match serde_json::from_str::<Value>(text) {
            Ok(value) => {
                match serde_json::to_string_pretty(&value) {
                    Ok(formatted) => Ok(formatted),
                    Err(e) => Err(format!("Failed to format JSON: {}", e)),
                }
            }
            Err(e) => Err(format!("Invalid JSON: {}", e)),
        }
    }

    /// Highlight JSON text with syntax coloring
    pub fn highlight_json(text: &str, max_lines: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        // First try to parse and validate
        let is_valid = serde_json::from_str::<Value>(text).is_ok();
        
        if !is_valid {
            // If invalid, show error highlighting
            return Self::highlight_invalid_json(text, max_lines);
        }
        
        // If valid, apply syntax highlighting
        let text_lines: Vec<&str> = text.lines().collect();
        let display_lines = if text_lines.len() > max_lines {
            text_lines.into_iter().take(max_lines - 1).collect::<Vec<_>>()
        } else {
            text_lines
        };
        
        for line in display_lines {
            lines.push(Self::highlight_json_line(line));
        }
        
        // Add truncation notice if needed
        if text.lines().count() > max_lines {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("... ({} more lines)", text.lines().count() - max_lines + 1),
                    Style::default().fg(Color::Gray)
                ),
            ]));
        }
        
        lines
    }

    /// Highlight a single JSON line
    fn highlight_json_line(line: &str) -> Line<'static> {
        let tokens = Self::tokenize_json_line(line);
        let mut spans = Vec::new();
        
        for token in tokens {
            let (text, style) = match token {
                JsonToken::LeftBrace | JsonToken::RightBrace => 
                    ("{".to_string(), Style::default().fg(Color::Yellow)),
                JsonToken::LeftBracket | JsonToken::RightBracket => 
                    ("[".to_string(), Style::default().fg(Color::Yellow)),
                JsonToken::Comma => 
                    (",".to_string(), Style::default().fg(Color::Gray)),
                JsonToken::Colon => 
                    (":".to_string(), Style::default().fg(Color::Gray)),
                JsonToken::String(s) => 
                    (s, Style::default().fg(Color::Green)),
                JsonToken::Number(n) => 
                    (n, Style::default().fg(Color::Cyan)),
                JsonToken::Boolean(b) => 
                    (b, Style::default().fg(Color::Magenta)),
                JsonToken::Null => 
                    ("null".to_string(), Style::default().fg(Color::Red)),
                JsonToken::Whitespace(w) => 
                    (w, Style::default()),
                JsonToken::Invalid(i) => 
                    (i, Style::default().fg(Color::Red).bg(Color::DarkGray)),
            };
            
            spans.push(Span::styled(text, style));
        }
        
        Line::from(spans)
    }

    /// Tokenize a JSON line for highlighting
    fn tokenize_json_line(line: &str) -> Vec<JsonToken> {
        let mut tokens = Vec::new();
        let mut chars = line.chars().peekable();
        let mut current_token = String::new();
        
        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    if !current_token.is_empty() {
                        tokens.push(JsonToken::Invalid(current_token.clone()));
                        current_token.clear();
                    }
                    tokens.push(JsonToken::LeftBrace);
                }
                '}' => {
                    if !current_token.is_empty() {
                        tokens.push(JsonToken::Invalid(current_token.clone()));
                        current_token.clear();
                    }
                    tokens.push(JsonToken::RightBrace);
                }
                '[' => {
                    if !current_token.is_empty() {
                        tokens.push(JsonToken::Invalid(current_token.clone()));
                        current_token.clear();
                    }
                    tokens.push(JsonToken::LeftBracket);
                }
                ']' => {
                    if !current_token.is_empty() {
                        tokens.push(JsonToken::Invalid(current_token.clone()));
                        current_token.clear();
                    }
                    tokens.push(JsonToken::RightBracket);
                }
                ',' => {
                    if !current_token.is_empty() {
                        tokens.push(Self::classify_token(&current_token));
                        current_token.clear();
                    }
                    tokens.push(JsonToken::Comma);
                }
                ':' => {
                    if !current_token.is_empty() {
                        tokens.push(Self::classify_token(&current_token));
                        current_token.clear();
                    }
                    tokens.push(JsonToken::Colon);
                }
                '"' => {
                    if !current_token.is_empty() {
                        tokens.push(JsonToken::Invalid(current_token.clone()));
                        current_token.clear();
                    }
                    // Parse string token
                    current_token.push(ch);
                    let mut escaped = false;
                    while let Some(string_ch) = chars.next() {
                        current_token.push(string_ch);
                        if string_ch == '"' && !escaped {
                            break;
                        }
                        escaped = string_ch == '\\' && !escaped;
                    }
                    tokens.push(JsonToken::String(current_token.clone()));
                    current_token.clear();
                }
                c if c.is_whitespace() => {
                    if !current_token.is_empty() {
                        tokens.push(Self::classify_token(&current_token));
                        current_token.clear();
                    }
                    // Collect consecutive whitespace
                    let mut whitespace = String::new();
                    whitespace.push(c);
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_whitespace() {
                            whitespace.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(JsonToken::Whitespace(whitespace));
                }
                _ => {
                    current_token.push(ch);
                }
            }
        }
        
        if !current_token.is_empty() {
            tokens.push(Self::classify_token(&current_token));
        }
        
        tokens
    }

    /// Classify a token based on its content
    fn classify_token(token: &str) -> JsonToken {
        if token == "true" || token == "false" {
            JsonToken::Boolean(token.to_string())
        } else if token == "null" {
            JsonToken::Null
        } else if token.parse::<f64>().is_ok() {
            JsonToken::Number(token.to_string())
        } else {
            JsonToken::Invalid(token.to_string())
        }
    }

    /// Highlight invalid JSON with error indication
    fn highlight_invalid_json(text: &str, max_lines: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        // Add error header
        lines.push(Line::from(vec![
            Span::styled("⚠ INVALID JSON", Style::default().fg(Color::Red).bg(Color::DarkGray)),
        ]));
        
        let text_lines: Vec<&str> = text.lines().collect();
        let display_lines = if text_lines.len() > max_lines - 1 {
            text_lines.into_iter().take(max_lines - 2).collect::<Vec<_>>()
        } else {
            text_lines
        };
        
        for line in display_lines {
            lines.push(Line::from(vec![
                Span::styled(line.to_string(), Style::default().fg(Color::Red)),
            ]));
        }
        
        // Add truncation notice if needed
        if text.lines().count() > max_lines - 1 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("... ({} more lines)", text.lines().count() - max_lines + 2),
                    Style::default().fg(Color::Gray)
                ),
            ]));
        }
        
        lines
    }

    /// Get JSON validation info
    pub fn get_json_info(text: &str) -> JsonInfo {
        match serde_json::from_str::<Value>(text) {
            Ok(value) => {
                let size = Self::calculate_json_size(&value);
                let depth = Self::calculate_json_depth(&value);
                JsonInfo {
                    is_valid: true,
                    error_message: None,
                    value_type: Self::get_json_type(&value),
                    size,
                    depth,
                    formatted_size: Self::format_size(text.len()),
                }
            }
            Err(e) => {
                JsonInfo {
                    is_valid: false,
                    error_message: Some(e.to_string()),
                    value_type: JsonValueType::Invalid,
                    size: 0,
                    depth: 0,
                    formatted_size: Self::format_size(text.len()),
                }
            }
        }
    }

    fn get_json_type(value: &Value) -> JsonValueType {
        match value {
            Value::Object(_) => JsonValueType::Object,
            Value::Array(_) => JsonValueType::Array,
            Value::String(_) => JsonValueType::String,
            Value::Number(_) => JsonValueType::Number,
            Value::Bool(_) => JsonValueType::Boolean,
            Value::Null => JsonValueType::Null,
        }
    }

    fn calculate_json_size(value: &Value) -> usize {
        match value {
            Value::Object(map) => map.len(),
            Value::Array(arr) => arr.len(),
            _ => 1,
        }
    }

    fn calculate_json_depth(value: &Value) -> usize {
        match value {
            Value::Object(map) => {
                1 + map.values()
                    .map(Self::calculate_json_depth)
                    .max()
                    .unwrap_or(0)
            }
            Value::Array(arr) => {
                1 + arr.iter()
                    .map(Self::calculate_json_depth)
                    .max()
                    .unwrap_or(0)
            }
            _ => 0,
        }
    }

    fn format_size(bytes: usize) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }
}

/// Information about JSON content
#[derive(Debug, Clone)]
pub struct JsonInfo {
    pub is_valid: bool,
    pub error_message: Option<String>,
    pub value_type: JsonValueType,
    pub size: usize,
    pub depth: usize,
    pub formatted_size: String,
}

#[derive(Debug, Clone)]
pub enum JsonValueType {
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
    Invalid,
}

impl std::fmt::Display for JsonValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonValueType::Object => write!(f, "Object"),
            JsonValueType::Array => write!(f, "Array"),
            JsonValueType::String => write!(f, "String"),
            JsonValueType::Number => write!(f, "Number"),
            JsonValueType::Boolean => write!(f, "Boolean"),
            JsonValueType::Null => write!(f, "Null"),
            JsonValueType::Invalid => write!(f, "Invalid"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_detection() {
        assert!(JsonHighlighter::is_json_like(r#"{"key": "value"}"#));
        assert!(JsonHighlighter::is_json_like(r#"[1, 2, 3]"#));
        assert!(JsonHighlighter::is_json_like("true"));
        assert!(JsonHighlighter::is_json_like("null"));
        assert!(JsonHighlighter::is_json_like("123.45"));
        assert!(!JsonHighlighter::is_json_like("plain text"));
    }

    #[test]
    fn test_json_validation() {
        let valid_json = r#"{"name": "test", "value": 123}"#;
        let info = JsonHighlighter::get_json_info(valid_json);
        assert!(info.is_valid);
        assert_eq!(info.value_type.to_string(), "Object");

        let invalid_json = r#"{"name": "test", "value":}"#;
        let info = JsonHighlighter::get_json_info(invalid_json);
        assert!(!info.is_valid);
        assert!(info.error_message.is_some());
    }
}