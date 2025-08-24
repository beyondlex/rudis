/// State for command input panel
#[derive(Debug, Default)]
pub struct CommandInputState {
    /// Current command input
    pub input: String,
    /// Cursor position in input
    pub cursor_position: usize,
    /// Command history
    pub history: Vec<String>,
    /// Current history index
    pub history_index: usize,
    /// Command results
    pub results: Vec<CommandResult>,
}

/// Result of a Redis command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub command: String,
    pub result: Result<String, String>,
    pub timestamp: std::time::SystemTime,
} 