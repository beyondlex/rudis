use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    prelude::Stylize,
};

/// Binary data viewer and formatter
pub struct BinaryViewer;

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayMode {
    /// Show data as UTF-8 text with escape sequences for non-printable chars
    Text,
    /// Show data as hexadecimal dump with ASCII preview
    Hex,
    /// Show data as Base64 encoded string
    Base64,
    /// Auto-detect best display mode based on content
    Auto,
}

#[derive(Debug, Clone)]
pub struct BinaryInfo {
    /// Total size in bytes
    pub size: usize,
    /// Number of printable ASCII characters
    pub printable_chars: usize,
    /// Number of null bytes
    pub null_bytes: usize,
    /// Number of control characters
    pub control_chars: usize,
    /// Whether the data appears to be text
    pub is_likely_text: bool,
    /// Whether the data contains binary content
    pub has_binary_content: bool,
    /// Detected character encoding (if any)
    pub encoding: Option<String>,
}

impl BinaryViewer {
    /// Analyze binary data and provide information
    pub fn analyze_data(data: &[u8]) -> BinaryInfo {
        let size = data.len();
        let mut printable_chars = 0;
        let mut null_bytes = 0;
        let mut control_chars = 0;
        
        for &byte in data {
            match byte {
                0 => null_bytes += 1,
                1..=31 => {
                    if byte == b'\t' || byte == b'\n' || byte == b'\r' {
                        printable_chars += 1;
                    } else {
                        control_chars += 1;
                    }
                }
                32..=126 => printable_chars += 1,
                127 => control_chars += 1,
                128..=255 => {
                    // Extended ASCII or UTF-8 continuation bytes
                    // For simplicity, we'll count these as potentially printable
                    // in UTF-8 context, but flag as binary for pure ASCII analysis
                }
            }
        }
        
        let text_ratio = if size > 0 {
            printable_chars as f64 / size as f64
        } else {
            0.0
        };
        
        let is_likely_text = text_ratio > 0.7 && null_bytes == 0;
        let has_binary_content = null_bytes > 0 || control_chars > size / 10;
        
        // Simple encoding detection
        let encoding = if std::str::from_utf8(data).is_ok() {
            Some("UTF-8".to_string())
        } else if data.iter().all(|&b| b <= 127) {
            Some("ASCII".to_string())
        } else {
            None
        };
        
        BinaryInfo {
            size,
            printable_chars,
            null_bytes,
            control_chars,
            is_likely_text,
            has_binary_content,
            encoding,
        }
    }
    
    /// Determine the best display mode for the data
    pub fn auto_detect_mode(data: &[u8]) -> DisplayMode {
        let info = Self::analyze_data(data);
        
        if info.is_likely_text && !info.has_binary_content {
            DisplayMode::Text
        } else if info.size <= 1024 {
            // Small binary data - show as hex
            DisplayMode::Hex
        } else {
            // Large binary data - show as base64 for compactness
            DisplayMode::Base64
        }
    }
    
    /// Display binary data in the specified mode
    pub fn display_data(
        data: &[u8],
        mode: DisplayMode,
        max_lines: usize,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        let info = Self::analyze_data(data);
        
        // Add header with data information
        lines.push(Line::from(vec![
            Span::styled("Size: ", Style::default().fg(Color::Cyan)),
            Span::styled(Self::format_size(info.size), Style::default().fg(Color::White)),
            Span::styled(" | ", Style::default().fg(Color::Gray)),
            Span::styled("Encoding: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                info.encoding.unwrap_or_else(|| "Binary".to_string()),
                Style::default().fg(Color::Yellow)
            ),
        ]));
        
        // Add binary analysis info
        if info.has_binary_content {
            lines.push(Line::from(vec![
                Span::styled("Binary: ", Style::default().fg(Color::Cyan)),
                Span::styled(format!("{} null", info.null_bytes), Style::default().fg(Color::Red)),
                Span::styled(" | ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{} ctrl", info.control_chars), Style::default().fg(Color::Yellow)),
                Span::styled(" | ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{} print", info.printable_chars), Style::default().fg(Color::Green)),
            ]));
        }
        
        lines.push(Line::from(""));
        
        // Display mode indicator
        let actual_mode = if mode == DisplayMode::Auto {
            Self::auto_detect_mode(data)
        } else {
            mode
        };
        
        let mode_text = match actual_mode {
            DisplayMode::Text => "Text View",
            DisplayMode::Hex => "Hex View", 
            DisplayMode::Base64 => "Base64 View",
            DisplayMode::Auto => "Auto View",
        };
        
        lines.push(Line::from(vec![
            Span::styled("Mode: ", Style::default().fg(Color::Cyan)),
            Span::styled(mode_text, Style::default().fg(Color::Yellow)),
            Span::styled(" (m: Toggle Mode)", Style::default().fg(Color::Gray)),
        ]));
        
        lines.push(Line::from(""));
        
        // Display content based on mode
        let remaining_lines = max_lines.saturating_sub(lines.len());
        
        match actual_mode {
            DisplayMode::Text => {
                lines.extend(Self::display_as_text(data, remaining_lines));
            }
            DisplayMode::Hex => {
                lines.extend(Self::display_as_hex(data, remaining_lines));
            }
            DisplayMode::Base64 => {
                lines.extend(Self::display_as_base64(data, remaining_lines));
            }
            DisplayMode::Auto => {
                // This should not happen since we resolve Auto above
                lines.extend(Self::display_as_text(data, remaining_lines));
            }
        }
        
        lines
    }
    
    /// Display data as text with escape sequences
    fn display_as_text(data: &[u8], max_lines: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        match std::str::from_utf8(data) {
            Ok(text) => {
                let text_lines: Vec<&str> = text.lines().collect();
                let display_lines = if text_lines.len() > max_lines {
                    &text_lines[..max_lines - 1]
                } else {
                    &text_lines
                };
                
                for line in display_lines {
                    lines.push(Self::format_text_line(line));
                }
                
                if text_lines.len() > max_lines {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("... ({} more lines)", text_lines.len() - max_lines + 1),
                            Style::default().fg(Color::Gray)
                        ),
                    ]));
                }
            }
            Err(_) => {
                // Invalid UTF-8, show with escape sequences
                let mut current_line = Vec::new();
                let mut line_count = 0;
                
                for &byte in data {
                    if line_count >= max_lines - 1 {
                        break;
                    }
                    
                    match byte {
                        b'\n' => {
                            lines.push(Line::from(current_line));
                            current_line = Vec::new();
                            line_count += 1;
                        }
                        32..=126 => {
                            current_line.push(Span::styled(
                                char::from(byte).to_string(),
                                Style::default().fg(Color::White)
                            ));
                        }
                        _ => {
                            current_line.push(Span::styled(
                                format!("\\x{:02x}", byte),
                                Style::default().fg(Color::Yellow)
                            ));
                        }
                    }
                }
                
                if !current_line.is_empty() {
                    lines.push(Line::from(current_line));
                }
                
                if data.len() > 0 && line_count >= max_lines - 1 {
                    lines.push(Line::from(vec![
                        Span::styled("... (truncated)", Style::default().fg(Color::Gray)),
                    ]));
                }
            }
        }
        
        if lines.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("<empty>", Style::default().fg(Color::Gray)),
            ]));
        }
        
        lines
    }
    
    /// Format a text line with escape sequences for control characters
    fn format_text_line(line: &str) -> Line<'static> {
        let mut spans = Vec::new();
        
        for ch in line.chars() {
            match ch {
                '\t' => spans.push(Span::styled("\\t", Style::default().fg(Color::Cyan))),
                '\r' => spans.push(Span::styled("\\r", Style::default().fg(Color::Cyan))),
                c if c.is_control() => {
                    spans.push(Span::styled(
                        format!("\\u{{{:04x}}}", c as u32),
                        Style::default().fg(Color::Yellow)
                    ));
                }
                c => spans.push(Span::styled(c.to_string(), Style::default().fg(Color::White))),
            }
        }
        
        if spans.is_empty() {
            spans.push(Span::styled("<empty line>", Style::default().fg(Color::Gray)));
        }
        
        Line::from(spans)
    }
    
    /// Display data as hexadecimal dump with ASCII preview
    fn display_as_hex(data: &[u8], max_lines: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        const BYTES_PER_LINE: usize = 16;
        
        let max_bytes = max_lines.saturating_mul(BYTES_PER_LINE);
        let display_data = if data.len() > max_bytes {
            &data[..max_bytes]
        } else {
            data
        };
        
        for (line_idx, chunk) in display_data.chunks(BYTES_PER_LINE).enumerate() {
            let offset = line_idx * BYTES_PER_LINE;
            let mut spans = Vec::new();
            
            // Offset
            spans.push(Span::styled(
                format!("{:08x}: ", offset),
                Style::default().fg(Color::Gray)
            ));
            
            // Hex bytes
            for (i, &byte) in chunk.iter().enumerate() {
                if i == 8 {
                    spans.push(Span::styled(" ", Style::default()));
                }
                
                let hex_color = match byte {
                    0 => Color::DarkGray,
                    1..=31 => Color::Yellow,
                    32..=126 => Color::Green,
                    _ => Color::Cyan,
                };
                
                spans.push(Span::styled(
                    format!("{:02x} ", byte),
                    Style::default().fg(hex_color)
                ));
            }
            
            // Padding for incomplete lines
            let padding_bytes = BYTES_PER_LINE - chunk.len();
            for i in 0..padding_bytes {
                if chunk.len() + i == 8 {
                    spans.push(Span::styled(" ", Style::default()));
                }
                spans.push(Span::styled("   ", Style::default()));
            }
            
            // ASCII preview
            spans.push(Span::styled(" |", Style::default().fg(Color::Gray)));
            
            for &byte in chunk {
                let ch = match byte {
                    32..=126 => char::from(byte).to_string(),
                    _ => ".".to_string(),
                };
                
                let ascii_color = match byte {
                    0 => Color::DarkGray,
                    1..=31 => Color::Yellow,
                    32..=126 => Color::White,
                    _ => Color::Cyan,
                };
                
                spans.push(Span::styled(ch, Style::default().fg(ascii_color)));
            }
            
            spans.push(Span::styled("|", Style::default().fg(Color::Gray)));
            
            lines.push(Line::from(spans));
        }
        
        if data.len() > max_bytes {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("... ({} more bytes)", data.len() - max_bytes),
                    Style::default().fg(Color::Gray)
                ),
            ]));
        }
        
        if lines.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("<empty>", Style::default().fg(Color::Gray)),
            ]));
        }
        
        lines
    }
    
    /// Display data as Base64 encoded string
    fn display_as_base64(data: &[u8], max_lines: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        let base64_string = base64::encode(data);
        const CHARS_PER_LINE: usize = 80;
        
        let base64_lines: Vec<&str> = base64_string
            .as_bytes()
            .chunks(CHARS_PER_LINE)
            .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
            .collect();
        
        let display_lines = if base64_lines.len() > max_lines - 1 {
            &base64_lines[..max_lines - 1]
        } else {
            &base64_lines
        };
        
        for line in display_lines {
            lines.push(Line::from(vec![
                Span::styled(line.to_string(), Style::default().fg(Color::Green)),
            ]));
        }
        
        if base64_lines.len() > max_lines - 1 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("... ({} more lines)", base64_lines.len() - max_lines + 1),
                    Style::default().fg(Color::Gray)
                ),
            ]));
        }
        
        if lines.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("<empty>", Style::default().fg(Color::Gray)),
            ]));
        }
        
        lines
    }
    
    /// Format byte size in human-readable format
    fn format_size(size: usize) -> String {
        if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else if size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_analysis() {
        // Text data
        let text_data = b"Hello, World!";
        let info = BinaryViewer::analyze_data(text_data);
        assert!(info.is_likely_text);
        assert!(!info.has_binary_content);
        
        // Binary data with null bytes
        let binary_data = &[0x00, 0x01, 0x02, 0xFF, 0x00];
        let info = BinaryViewer::analyze_data(binary_data);
        assert!(!info.is_likely_text);
        assert!(info.has_binary_content);
        assert_eq!(info.null_bytes, 2);
    }
    
    #[test]
    fn test_mode_detection() {
        let text_data = b"Hello, World!";
        assert_eq!(BinaryViewer::auto_detect_mode(text_data), DisplayMode::Text);
        
        let binary_data = &[0x00, 0x01, 0x02, 0xFF];
        assert_eq!(BinaryViewer::auto_detect_mode(binary_data), DisplayMode::Hex);
    }
}