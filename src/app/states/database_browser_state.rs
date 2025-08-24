use ratatui::widgets::ScrollbarState;
use crate::ui::tree_view::KeyTree;

/// State for database browser panel
#[derive(Debug)]
pub struct DatabaseBrowserState {
    /// Available databases
    pub databases: Vec<u8>,
    /// Currently selected database
    pub selected_database: u8,
    /// Currently selected key index
    pub selected_key_index: usize,
    /// Scroll offset for the key list
    pub scroll_offset: usize,
    /// Current search/filter pattern
    pub filter_pattern: String,
    /// Whether we're in search mode
    pub search_mode: bool,
    /// Cached keys for current database
    pub keys: Vec<KeyInfo>,
    /// Key scan cursor for pagination
    pub scan_cursor: u64,
    /// Whether we're currently loading keys
    pub loading: bool,
    /// Whether we've loaded all keys (scan cursor = 0)
    pub scan_complete: bool,
    /// Total key count for current database
    pub total_keys: Option<usize>,
    /// Tree view for hierarchical key display
    pub key_tree: KeyTree,
    /// Whether to use tree view (true) or flat list view (false)
    pub use_tree_view: bool,
    /// Key separator for tree hierarchy (default: ":")
    pub tree_separator: String,
    /// Scrollbar state for visual scroll indicator
    pub scrollbar_state: ScrollbarState,
}

impl Default for DatabaseBrowserState {
    fn default() -> Self {
        Self {
            databases: Vec::new(),
            selected_database: 0,
            selected_key_index: 0,
            scroll_offset: 0,
            filter_pattern: String::new(),
            search_mode: false,
            keys: Vec::new(),
            scan_cursor: 0,
            loading: false,
            scan_complete: false,
            total_keys: None,
            key_tree: KeyTree::new(":".to_string()),
            use_tree_view: true, // Enable tree view by default
            tree_separator: ":".to_string(),
            scrollbar_state: ScrollbarState::default(),
        }
    }
}

/// Information about a Redis key
#[derive(Debug, Clone)]
pub struct KeyInfo {
    /// Key name
    pub name: String,
    /// Key type (string, hash, list, set, zset, stream)
    pub key_type: Option<String>,
    /// TTL in seconds (-1 for no expiry, -2 for key doesn't exist)
    pub ttl: Option<i64>,
    /// Key size/length
    pub size: Option<usize>,
    /// Whether this key matches current filter
    pub matches_filter: bool,
} 