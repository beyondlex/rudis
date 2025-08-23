use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::app::config::{AppConfig, ConnectionConfig};
use crate::error::AppResult;
use crate::redis::RedisConnection;
use crate::events::AppEvent;

/// Current view mode of the application
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    /// Connection list view
    ConnectionList,
    /// Database browser view
    DatabaseBrowser,
    /// Key viewer and editor
    KeyViewer,
    /// Command interface for Redis CLI
    CommandInterface,
    /// Application settings
    Settings,
    /// Help screen
    Help,
}

/// Application state container
#[derive(Debug)]
pub struct AppState {
    /// Is the application running?
    pub running: bool,
    
    /// Current view mode
    pub current_view: ViewMode,
    
    /// Active connection ID
    pub active_connection: Option<String>,
    
    /// All Redis connections
    pub connections: HashMap<String, RedisConnection>,
    
    /// Selected database number
    pub selected_database: Option<u8>,
    
    /// Currently selected key
    pub selected_key: Option<String>,
    
    /// Application configuration
    pub config: AppConfig,
    
    /// Event receiver for async operations
    pub event_rx: Option<mpsc::UnboundedReceiver<AppEvent>>,
    
    /// Event sender for async operations
    pub event_tx: mpsc::UnboundedSender<AppEvent>,
    
    /// Current status message
    pub status_message: Option<String>,
    
    /// UI state for different panels
    pub ui_state: UiState,
}

/// UI-specific state information
#[derive(Debug, Default)]
pub struct UiState {
    /// Currently focused panel
    pub focused_panel: FocusedPanel,
    
    /// Connection list state
    pub connection_list: ConnectionListState,
    
    /// Database browser state
    pub database_browser: DatabaseBrowserState,
    
    /// Key viewer state
    pub key_viewer: KeyViewerState,
    
    /// Command input state
    pub command_input: CommandInputState,
    
    /// Connection dialog state
    pub connection_dialog: ConnectionDialogState,
    
    /// Confirmation dialog state
    pub confirmation_dialog: crate::ui::confirmation_dialog::ConfirmationDialog,
    
    /// Export/Import dialog state
    pub export_import_dialog: crate::ui::export_import_dialog::ExportImportDialog,
    
    /// Bulk operations dialog state
    pub bulk_operations_dialog: crate::ui::bulk_operations_dialog::BulkOperationsDialog,
    
    /// Progress bar manager
    pub progress_bar_manager: crate::ui::progress_bar::ProgressBarManager,
}

/// Which panel currently has focus
#[derive(Debug, Default, Clone, PartialEq)]
pub enum FocusedPanel {
    #[default]
    ConnectionList,
    DatabaseBrowser,
    KeyViewer,
    CommandInput,
}

/// State for connection list panel
#[derive(Debug, Default)]
pub struct ConnectionListState {
    /// Currently selected connection index
    pub selected_index: usize,
    /// Scroll offset for the list
    pub scroll_offset: usize,
}

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
    pub key_tree: crate::ui::tree_view::KeyTree,
    /// Whether to use tree view (true) or flat list view (false)
    pub use_tree_view: bool,
    /// Key separator for tree hierarchy (default: ":")
    pub tree_separator: String,
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
            key_tree: crate::ui::tree_view::KeyTree::new(":".to_string()),
            use_tree_view: true, // Enable tree view by default
            tree_separator: ":".to_string(),
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

/// State for key viewer panel
#[derive(Debug)]
pub struct KeyViewerState {
    /// Current key name being viewed
    pub current_key: Option<String>,
    /// Current key value with type-specific data
    pub value: Option<crate::redis::value_types::RedisValue>,
    /// Key metadata (type, ttl, size)
    pub metadata: Option<KeyMetadata>,
    /// Current page for paginated data types
    pub current_page: usize,
    /// Items per page for paginated data types
    pub page_size: usize,
    /// Scroll position in content
    pub scroll_position: usize,
    /// Whether we're in edit mode
    pub edit_mode: bool,
    /// Edit buffer for string values
    pub edit_buffer: String,
    /// Cursor position in edit buffer
    pub edit_cursor_position: usize,
    /// Original value before editing (for cancellation)
    pub original_value: Option<String>,
    /// Whether there are unsaved changes
    pub has_unsaved_changes: bool,
    /// Loading state
    pub loading: bool,
    /// Hash field editing state
    pub hash_field_index: usize,
    /// Selected hash field for editing
    pub selected_hash_field: Option<String>,
    /// Hash field edit mode (None, Field, Value)
    pub hash_edit_mode: HashEditMode,
    /// Hash field edit buffer
    pub hash_field_buffer: String,
    /// Hash value edit buffer
    pub hash_value_buffer: String,
    /// Original hash field name (for field renaming)
    pub original_field_name: Option<String>,
    /// List element editing state
    pub list_element_index: usize,
    /// Selected list element for editing
    pub selected_list_element: Option<String>,
    /// List edit mode (None, Element, Insert)
    pub list_edit_mode: ListEditMode,
    /// List element edit buffer
    pub list_element_buffer: String,
    /// Insert position for new elements
    pub list_insert_index: Option<usize>,
    /// Set member editing state
    pub set_member_index: usize,
    /// Selected set member for operations
    pub selected_set_member: Option<String>,
    /// Set edit mode (None, Add, Remove)
    pub set_edit_mode: SetEditMode,
    /// Set member edit buffer
    pub set_member_buffer: String,
    /// Sorted set editing state
    pub zset_member_index: usize,
    /// Selected sorted set member for operations
    pub selected_zset_member: Option<String>,
    /// Sorted set edit mode (None, Add, Remove, UpdateScore)
    pub zset_edit_mode: ZSetEditMode,
    /// Sorted set member edit buffer
    pub zset_member_buffer: String,
    /// Sorted set score edit buffer
    pub zset_score_buffer: String,
    /// Stream viewing state
    pub stream_entry_index: usize,
    /// Selected stream entry for viewing
    pub selected_stream_entry: Option<String>, // Entry ID
    /// Stream view mode (List, Detail)
    pub stream_view_mode: StreamViewMode,
    /// Stream field index for detail view
    pub stream_field_index: usize,
    /// Binary display mode for binary data
    pub binary_display_mode: crate::ui::binary_viewer::DisplayMode,
    /// Whether to enable JSON syntax highlighting
    pub json_highlighting_enabled: bool,
}

impl Default for KeyViewerState {
    fn default() -> Self {
        Self {
            current_key: None,
            value: None,
            metadata: None,
            current_page: 0,
            page_size: 50,
            scroll_position: 0,
            edit_mode: false,
            edit_buffer: String::new(),
            edit_cursor_position: 0,
            original_value: None,
            has_unsaved_changes: false,
            loading: false,
            hash_field_index: 0,
            selected_hash_field: None,
            hash_edit_mode: HashEditMode::None,
            hash_field_buffer: String::new(),
            hash_value_buffer: String::new(),
            original_field_name: None,
            list_element_index: 0,
            selected_list_element: None,
            list_edit_mode: ListEditMode::None,
            list_element_buffer: String::new(),
            list_insert_index: None,
            set_member_index: 0,
            selected_set_member: None,
            set_edit_mode: SetEditMode::None,
            set_member_buffer: String::new(),
            zset_member_index: 0,
            selected_zset_member: None,
            zset_edit_mode: ZSetEditMode::None,
            zset_member_buffer: String::new(),
            zset_score_buffer: String::new(),
            stream_entry_index: 0,
            selected_stream_entry: None,
            stream_view_mode: StreamViewMode::List,
            stream_field_index: 0,
            binary_display_mode: crate::ui::binary_viewer::DisplayMode::Auto,
            json_highlighting_enabled: true,
        }
    }
}

/// Key metadata information
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub key_type: String,
    pub ttl: Option<i64>,
    pub size: usize,
    pub encoding: Option<String>,
}

/// Hash field editing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum HashEditMode {
    #[default]
    None,
    Field,    // Editing field name
    Value,    // Editing field value
    NewField, // Adding new field
}

/// List element editing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ListEditMode {
    #[default]
    None,
    Element,  // Editing existing element
    Insert,   // Inserting new element
    Append,   // Appending new element
}

/// Set member editing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SetEditMode {
    #[default]
    None,
    Add,      // Adding new member
    Remove,   // Removing member (confirmation)
}

/// Sorted set editing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ZSetEditMode {
    #[default]
    None,
    Add,          // Adding new member with score
    Remove,       // Removing member (confirmation)
    UpdateScore,  // Updating score of existing member
}

/// Stream viewing mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum StreamViewMode {
    #[default]
    List,    // List view showing entry IDs and summary
    Detail,  // Detail view showing selected entry fields
}

impl KeyViewerState {
    /// Enter edit mode for string values
    pub fn enter_edit_mode(&mut self) {
        if let Some(crate::redis::value_types::RedisValue::String(s)) = &self.value {
            self.edit_mode = true;
            self.edit_buffer = s.clone();
            self.original_value = Some(s.clone());
            self.edit_cursor_position = s.len();
            self.has_unsaved_changes = false;
        }
    }
    
    /// Exit edit mode and discard changes
    pub fn exit_edit_mode(&mut self) {
        self.edit_mode = false;
        self.edit_buffer.clear();
        self.original_value = None;
        self.edit_cursor_position = 0;
        self.has_unsaved_changes = false;
    }
    
    /// Insert character at cursor position
    pub fn insert_char(&mut self, c: char) {
        if self.edit_mode {
            self.edit_buffer.insert(self.edit_cursor_position, c);
            self.edit_cursor_position += c.len_utf8();
            self.has_unsaved_changes = true;
        }
    }
    
    /// Delete character before cursor
    pub fn delete_char(&mut self) {
        if self.edit_mode && self.edit_cursor_position > 0 {
            let mut chars: Vec<char> = self.edit_buffer.chars().collect();
            if !chars.is_empty() {
                // Find the actual character position (not byte position)
                let char_pos = self.edit_buffer[..self.edit_cursor_position].chars().count();
                if char_pos > 0 {
                    chars.remove(char_pos - 1);
                    self.edit_buffer = chars.into_iter().collect();
                    // Update cursor position
                    self.edit_cursor_position = self.edit_buffer.chars().take(char_pos - 1).map(|c| c.len_utf8()).sum();
                    self.has_unsaved_changes = true;
                }
            }
        }
    }
    
    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.edit_mode && self.edit_cursor_position > 0 {
            let chars: Vec<char> = self.edit_buffer.chars().collect();
            let char_pos = self.edit_buffer[..self.edit_cursor_position].chars().count();
            if char_pos > 0 {
                self.edit_cursor_position = chars.iter().take(char_pos - 1).map(|c| c.len_utf8()).sum();
            }
        }
    }
    
    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.edit_mode {
            let chars: Vec<char> = self.edit_buffer.chars().collect();
            let char_pos = self.edit_buffer[..self.edit_cursor_position].chars().count();
            if char_pos < chars.len() {
                self.edit_cursor_position = chars.iter().take(char_pos + 1).map(|c| c.len_utf8()).sum();
            }
        }
    }
    
    /// Move cursor to beginning of line
    pub fn move_cursor_home(&mut self) {
        if self.edit_mode {
            self.edit_cursor_position = 0;
        }
    }
    
    /// Move cursor to end of line
    pub fn move_cursor_end(&mut self) {
        if self.edit_mode {
            self.edit_cursor_position = self.edit_buffer.len();
        }
    }
    
    /// Get the current edit buffer value
    pub fn get_edit_value(&self) -> &str {
        &self.edit_buffer
    }
    
    /// Apply changes from edit buffer to the value
    pub fn apply_edit_changes(&mut self) {
        if self.edit_mode && self.has_unsaved_changes {
            self.value = Some(crate::redis::value_types::RedisValue::String(self.edit_buffer.clone()));
        }
    }
    
    /// Validate the current edit buffer
    pub fn validate_edit_buffer(&self) -> crate::ui::validation::ValidationResult {
        if self.edit_mode {
            let validator = crate::ui::validation::StringValidator::default();
            validator.validate(&self.edit_buffer)
        } else {
            crate::ui::validation::ValidationResult::Valid
        }
    }
    
    /// Validate JSON if the current value looks like JSON
    pub fn validate_json(&self) -> Option<crate::ui::validation::ValidationResult> {
        let text = if self.edit_mode {
            &self.edit_buffer
        } else if let Some(crate::redis::value_types::RedisValue::String(s)) = &self.value {
            s
        } else {
            return None;
        };
        
        if crate::ui::validation::JsonValidator::is_json_like(text) {
            Some(crate::ui::validation::JsonValidator::validate_json(text))
        } else {
            None
        }
    }
    
    // Hash editing methods
    
    /// Select a hash field by index
    pub fn select_hash_field(&mut self, index: usize) {
        if let Some(crate::redis::value_types::RedisValue::Hash(fields)) = &self.value {
            if index < fields.len() {
                self.hash_field_index = index;
                self.selected_hash_field = Some(fields[index].0.clone());
            }
        }
    }
    
    /// Start editing a hash field name
    pub fn start_edit_hash_field(&mut self) {
        if let Some(field_name) = &self.selected_hash_field {
            self.hash_edit_mode = HashEditMode::Field;
            self.hash_field_buffer = field_name.clone();
            self.original_field_name = Some(field_name.clone());
            self.has_unsaved_changes = false;
        }
    }
    
    /// Start editing a hash field value
    pub fn start_edit_hash_value(&mut self) {
        if let (Some(crate::redis::value_types::RedisValue::Hash(fields)), Some(field_name)) = (&self.value, &self.selected_hash_field) {
            if let Some((_, value)) = fields.iter().find(|(f, _)| f == field_name) {
                self.hash_edit_mode = HashEditMode::Value;
                self.hash_value_buffer = value.clone();
                self.has_unsaved_changes = false;
            }
        }
    }
    
    /// Start adding a new hash field
    pub fn start_add_hash_field(&mut self) {
        self.hash_edit_mode = HashEditMode::NewField;
        self.hash_field_buffer.clear();
        self.hash_value_buffer.clear();
        self.has_unsaved_changes = false;
    }
    
    /// Cancel hash editing
    pub fn cancel_hash_edit(&mut self) {
        self.hash_edit_mode = HashEditMode::None;
        self.hash_field_buffer.clear();
        self.hash_value_buffer.clear();
        self.original_field_name = None;
        self.has_unsaved_changes = false;
    }
    
    /// Insert character into hash edit buffer
    pub fn insert_hash_char(&mut self, c: char) {
        match self.hash_edit_mode {
            HashEditMode::Field | HashEditMode::NewField => {
                self.hash_field_buffer.push(c);
                self.has_unsaved_changes = true;
            }
            HashEditMode::Value => {
                self.hash_value_buffer.push(c);
                self.has_unsaved_changes = true;
            }
            HashEditMode::None => {}
        }
    }
    
    /// Delete character from hash edit buffer
    pub fn delete_hash_char(&mut self) {
        match self.hash_edit_mode {
            HashEditMode::Field | HashEditMode::NewField => {
                if !self.hash_field_buffer.is_empty() {
                    self.hash_field_buffer.pop();
                    self.has_unsaved_changes = true;
                }
            }
            HashEditMode::Value => {
                if !self.hash_value_buffer.is_empty() {
                    self.hash_value_buffer.pop();
                    self.has_unsaved_changes = true;
                }
            }
            HashEditMode::None => {}
        }
    }
    
    /// Move to next hash field
    pub fn next_hash_field(&mut self) {
        if let Some(crate::redis::value_types::RedisValue::Hash(fields)) = &self.value {
            if self.hash_field_index + 1 < fields.len() {
                self.hash_field_index += 1;
                self.selected_hash_field = Some(fields[self.hash_field_index].0.clone());
            }
        }
    }
    
    /// Move to previous hash field
    pub fn prev_hash_field(&mut self) {
        if self.hash_field_index > 0 {
            self.hash_field_index -= 1;
            if let Some(crate::redis::value_types::RedisValue::Hash(fields)) = &self.value {
                self.selected_hash_field = Some(fields[self.hash_field_index].0.clone());
            }
        }
    }
    
    /// Apply hash field changes
    pub fn apply_hash_changes(&mut self) {
        if !self.has_unsaved_changes {
            return;
        }
        
        if let Some(value) = &mut self.value {
            if let crate::redis::value_types::RedisValue::Hash(fields) = value {
            match self.hash_edit_mode {
                HashEditMode::Field => {
                    // Rename field
                    if let Some(original_name) = &self.original_field_name {
                        if let Some(pos) = fields.iter().position(|(f, _)| f == original_name) {
                            fields[pos].0 = self.hash_field_buffer.clone();
                            self.selected_hash_field = Some(self.hash_field_buffer.clone());
                        }
                    }
                }
                HashEditMode::Value => {
                    // Update field value
                    if let Some(field_name) = &self.selected_hash_field {
                        if let Some(pos) = fields.iter().position(|(f, _)| f == field_name) {
                            fields[pos].1 = self.hash_value_buffer.clone();
                        }
                    }
                }
                HashEditMode::NewField => {
                    // Add new field
                    if !self.hash_field_buffer.is_empty() {
                        fields.push((self.hash_field_buffer.clone(), self.hash_value_buffer.clone()));
                        self.selected_hash_field = Some(self.hash_field_buffer.clone());
                        self.hash_field_index = fields.len() - 1;
                    }
                }
                HashEditMode::None => {}
            }
        }
    }
    }
    
    /// Delete selected hash field
    pub fn delete_hash_field(&mut self) {
        if let Some(value) = &mut self.value {
            if let (crate::redis::value_types::RedisValue::Hash(fields), Some(field_name)) = (value, &self.selected_hash_field) {
            if let Some(pos) = fields.iter().position(|(f, _)| f == field_name) {
                fields.remove(pos);
                
                // Update selection
                if fields.is_empty() {
                    self.selected_hash_field = None;
                    self.hash_field_index = 0;
                } else {
                    self.hash_field_index = self.hash_field_index.min(fields.len() - 1);
                    self.selected_hash_field = Some(fields[self.hash_field_index].0.clone());
                }
                
                self.has_unsaved_changes = true;
            }
        }
    }
    }
    
    // List editing methods
    
    /// Select a list element by index
    pub fn select_list_element(&mut self, index: usize) {
        if let Some(crate::redis::value_types::RedisValue::List(elements)) = &self.value {
            if index < elements.len() {
                self.list_element_index = index;
                self.selected_list_element = Some(elements[index].clone());
            }
        }
    }
    
    /// Start editing a list element
    pub fn start_edit_list_element(&mut self) {
        if let Some(element) = &self.selected_list_element {
            self.list_edit_mode = ListEditMode::Element;
            self.list_element_buffer = element.clone();
            self.has_unsaved_changes = false;
        }
    }
    
    /// Start inserting a new list element at specific index
    pub fn start_insert_list_element(&mut self, index: usize) {
        self.list_edit_mode = ListEditMode::Insert;
        self.list_element_buffer.clear();
        self.list_insert_index = Some(index);
        self.has_unsaved_changes = false;
    }
    
    /// Start appending a new list element
    pub fn start_append_list_element(&mut self) {
        self.list_edit_mode = ListEditMode::Append;
        self.list_element_buffer.clear();
        self.list_insert_index = None;
        self.has_unsaved_changes = false;
    }
    
    /// Cancel list editing
    pub fn cancel_list_edit(&mut self) {
        self.list_edit_mode = ListEditMode::None;
        self.list_element_buffer.clear();
        self.list_insert_index = None;
        self.has_unsaved_changes = false;
    }
    
    /// Insert character into list edit buffer
    pub fn insert_list_char(&mut self, c: char) {
        if self.list_edit_mode != ListEditMode::None {
            self.list_element_buffer.push(c);
            self.has_unsaved_changes = true;
        }
    }
    
    /// Delete character from list edit buffer
    pub fn delete_list_char(&mut self) {
        if self.list_edit_mode != ListEditMode::None && !self.list_element_buffer.is_empty() {
            self.list_element_buffer.pop();
            self.has_unsaved_changes = true;
        }
    }
    
    /// Move to next list element
    pub fn next_list_element(&mut self) {
        if let Some(crate::redis::value_types::RedisValue::List(elements)) = &self.value {
            if self.list_element_index + 1 < elements.len() {
                self.list_element_index += 1;
                self.selected_list_element = Some(elements[self.list_element_index].clone());
            }
        }
    }
    
    /// Move to previous list element
    pub fn prev_list_element(&mut self) {
        if self.list_element_index > 0 {
            self.list_element_index -= 1;
            if let Some(crate::redis::value_types::RedisValue::List(elements)) = &self.value {
                self.selected_list_element = Some(elements[self.list_element_index].clone());
            }
        }
    }
    
    /// Apply list element changes
    pub fn apply_list_changes(&mut self) {
        if !self.has_unsaved_changes {
            return;
        }
        
        if let Some(value) = &mut self.value {
            if let crate::redis::value_types::RedisValue::List(elements) = value {
                match self.list_edit_mode {
                    ListEditMode::Element => {
                        // Update existing element
                        if self.list_element_index < elements.len() {
                            elements[self.list_element_index] = self.list_element_buffer.clone();
                            self.selected_list_element = Some(self.list_element_buffer.clone());
                        }
                    }
                    ListEditMode::Insert => {
                        // Insert element at specific index
                        if let Some(insert_index) = self.list_insert_index {
                            let insert_pos = insert_index.min(elements.len());
                            elements.insert(insert_pos, self.list_element_buffer.clone());
                            self.list_element_index = insert_pos;
                            self.selected_list_element = Some(self.list_element_buffer.clone());
                        }
                    }
                    ListEditMode::Append => {
                        // Append element to end
                        elements.push(self.list_element_buffer.clone());
                        self.list_element_index = elements.len() - 1;
                        self.selected_list_element = Some(self.list_element_buffer.clone());
                    }
                    ListEditMode::None => {}
                }
            }
        }
    }
    
    /// Delete selected list element
    pub fn delete_list_element(&mut self) {
        if let Some(value) = &mut self.value {
            if let crate::redis::value_types::RedisValue::List(elements) = value {
                if self.list_element_index < elements.len() {
                    elements.remove(self.list_element_index);
                    
                    // Update selection
                    if elements.is_empty() {
                        self.selected_list_element = None;
                        self.list_element_index = 0;
                    } else {
                        self.list_element_index = self.list_element_index.min(elements.len() - 1);
                        self.selected_list_element = Some(elements[self.list_element_index].clone());
                    }
                    
                    self.has_unsaved_changes = true;
                }
            }
        }
    }
    
    /// Move list element up (towards index 0)
    pub fn move_list_element_up(&mut self) {
        if self.list_element_index > 0 {
            if let Some(value) = &mut self.value {
                if let crate::redis::value_types::RedisValue::List(elements) = value {
                    elements.swap(self.list_element_index, self.list_element_index - 1);
                    self.list_element_index -= 1;
                    self.has_unsaved_changes = true;
                }
            }
        }
    }
    
    /// Move list element down (towards end)
    pub fn move_list_element_down(&mut self) {
        if let Some(value) = &mut self.value {
            if let crate::redis::value_types::RedisValue::List(elements) = value {
                if self.list_element_index + 1 < elements.len() {
                    elements.swap(self.list_element_index, self.list_element_index + 1);
                    self.list_element_index += 1;
                    self.has_unsaved_changes = true;
                }
            }
        }
    }
    
    // Set editing methods
    
    /// Select a set member by index
    pub fn select_set_member(&mut self, index: usize) {
        if let Some(crate::redis::value_types::RedisValue::Set(members)) = &self.value {
            if index < members.len() {
                self.set_member_index = index;
                self.selected_set_member = Some(members[index].clone());
            }
        }
    }
    
    /// Start adding a new set member
    pub fn start_add_set_member(&mut self) {
        self.set_edit_mode = SetEditMode::Add;
        self.set_member_buffer.clear();
        self.has_unsaved_changes = false;
    }
    
    /// Start removing a set member (confirmation mode)
    pub fn start_remove_set_member(&mut self) {
        if self.selected_set_member.is_some() {
            self.set_edit_mode = SetEditMode::Remove;
            self.has_unsaved_changes = false;
        }
    }
    
    /// Cancel set editing
    pub fn cancel_set_edit(&mut self) {
        self.set_edit_mode = SetEditMode::None;
        self.set_member_buffer.clear();
        self.has_unsaved_changes = false;
    }
    
    /// Insert character into set edit buffer
    pub fn insert_set_char(&mut self, c: char) {
        if self.set_edit_mode == SetEditMode::Add {
            self.set_member_buffer.push(c);
            self.has_unsaved_changes = true;
        }
    }
    
    /// Delete character from set edit buffer
    pub fn delete_set_char(&mut self) {
        if self.set_edit_mode == SetEditMode::Add && !self.set_member_buffer.is_empty() {
            self.set_member_buffer.pop();
            self.has_unsaved_changes = true;
        }
    }
    
    /// Move to next set member
    pub fn next_set_member(&mut self) {
        if let Some(crate::redis::value_types::RedisValue::Set(members)) = &self.value {
            if self.set_member_index + 1 < members.len() {
                self.set_member_index += 1;
                self.selected_set_member = Some(members[self.set_member_index].clone());
            }
        }
    }
    
    /// Move to previous set member
    pub fn prev_set_member(&mut self) {
        if self.set_member_index > 0 {
            self.set_member_index -= 1;
            if let Some(crate::redis::value_types::RedisValue::Set(members)) = &self.value {
                self.selected_set_member = Some(members[self.set_member_index].clone());
            }
        }
    }
    
    /// Apply set member changes
    pub fn apply_set_changes(&mut self) {
        if let Some(value) = &mut self.value {
            if let crate::redis::value_types::RedisValue::Set(members) = value {
                match self.set_edit_mode {
                    SetEditMode::Add => {
                        // Add new member if not already present
                        if !self.set_member_buffer.is_empty() && !members.contains(&self.set_member_buffer) {
                            members.push(self.set_member_buffer.clone());
                            // Sort members for consistent display
                            members.sort();
                            // Update selection to new member
                            if let Some(pos) = members.iter().position(|m| m == &self.set_member_buffer) {
                                self.set_member_index = pos;
                                self.selected_set_member = Some(self.set_member_buffer.clone());
                            }
                            self.has_unsaved_changes = true;
                        }
                    }
                    SetEditMode::Remove => {
                        // Remove selected member
                        if let Some(member) = &self.selected_set_member {
                            if let Some(pos) = members.iter().position(|m| m == member) {
                                members.remove(pos);
                                
                                // Update selection
                                if members.is_empty() {
                                    self.selected_set_member = None;
                                    self.set_member_index = 0;
                                } else {
                                    self.set_member_index = self.set_member_index.min(members.len() - 1);
                                    self.selected_set_member = Some(members[self.set_member_index].clone());
                                }
                                
                                self.has_unsaved_changes = true;
                            }
                        }
                    }
                    SetEditMode::None => {}
                }
            }
        }
    }
    
    /// Check if a member already exists in the set
    pub fn set_member_exists(&self, member: &str) -> bool {
        if let Some(crate::redis::value_types::RedisValue::Set(members)) = &self.value {
            members.contains(&member.to_string())
        } else {
            false
        }
    }
    
    // Sorted set editing methods
    
    /// Select a sorted set member by index
    pub fn select_zset_member(&mut self, index: usize) {
        if let Some(crate::redis::value_types::RedisValue::ZSet(members)) = &self.value {
            if index < members.len() {
                self.zset_member_index = index;
                self.selected_zset_member = Some(members[index].0.clone());
            }
        }
    }
    
    /// Start adding a new sorted set member
    pub fn start_add_zset_member(&mut self) {
        self.zset_edit_mode = ZSetEditMode::Add;
        self.zset_member_buffer.clear();
        self.zset_score_buffer.clear();
        self.has_unsaved_changes = false;
    }
    
    /// Start updating score of existing member
    pub fn start_update_zset_score(&mut self) {
        if let (Some(crate::redis::value_types::RedisValue::ZSet(members)), Some(member)) = (&self.value, &self.selected_zset_member) {
            if let Some((_, score)) = members.iter().find(|(m, _)| m == member) {
                self.zset_edit_mode = ZSetEditMode::UpdateScore;
                self.zset_score_buffer = score.to_string();
                self.has_unsaved_changes = false;
            }
        }
    }
    
    /// Start removing a sorted set member (confirmation mode)
    pub fn start_remove_zset_member(&mut self) {
        if self.selected_zset_member.is_some() {
            self.zset_edit_mode = ZSetEditMode::Remove;
            self.has_unsaved_changes = false;
        }
    }
    
    /// Cancel sorted set editing
    pub fn cancel_zset_edit(&mut self) {
        self.zset_edit_mode = ZSetEditMode::None;
        self.zset_member_buffer.clear();
        self.zset_score_buffer.clear();
        self.has_unsaved_changes = false;
    }
    
    /// Insert character into sorted set edit buffer
    pub fn insert_zset_char(&mut self, c: char, is_score_field: bool) {
        match self.zset_edit_mode {
            ZSetEditMode::Add => {
                if is_score_field {
                    self.zset_score_buffer.push(c);
                } else {
                    self.zset_member_buffer.push(c);
                }
                self.has_unsaved_changes = true;
            }
            ZSetEditMode::UpdateScore => {
                self.zset_score_buffer.push(c);
                self.has_unsaved_changes = true;
            }
            _ => {}
        }
    }
    
    /// Delete character from sorted set edit buffer
    pub fn delete_zset_char(&mut self, is_score_field: bool) {
        match self.zset_edit_mode {
            ZSetEditMode::Add => {
                if is_score_field && !self.zset_score_buffer.is_empty() {
                    self.zset_score_buffer.pop();
                    self.has_unsaved_changes = true;
                } else if !is_score_field && !self.zset_member_buffer.is_empty() {
                    self.zset_member_buffer.pop();
                    self.has_unsaved_changes = true;
                }
            }
            ZSetEditMode::UpdateScore => {
                if !self.zset_score_buffer.is_empty() {
                    self.zset_score_buffer.pop();
                    self.has_unsaved_changes = true;
                }
            }
            _ => {}
        }
    }
    
    /// Move to next sorted set member
    pub fn next_zset_member(&mut self) {
        if let Some(crate::redis::value_types::RedisValue::ZSet(members)) = &self.value {
            if self.zset_member_index + 1 < members.len() {
                self.zset_member_index += 1;
                self.selected_zset_member = Some(members[self.zset_member_index].0.clone());
            }
        }
    }
    
    /// Move to previous sorted set member
    pub fn prev_zset_member(&mut self) {
        if self.zset_member_index > 0 {
            self.zset_member_index -= 1;
            if let Some(crate::redis::value_types::RedisValue::ZSet(members)) = &self.value {
                self.selected_zset_member = Some(members[self.zset_member_index].0.clone());
            }
        }
    }
    
    /// Apply sorted set member changes
    pub fn apply_zset_changes(&mut self) {
        if let Some(value) = &mut self.value {
            if let crate::redis::value_types::RedisValue::ZSet(members) = value {
                match self.zset_edit_mode {
                    ZSetEditMode::Add => {
                        // Parse score and add new member
                        if !self.zset_member_buffer.is_empty() {
                            let score = self.zset_score_buffer.parse::<f64>().unwrap_or(0.0);
                            
                            // Remove existing member if it exists (update case)
                            members.retain(|(m, _)| m != &self.zset_member_buffer);
                            
                            // Add new member
                            members.push((self.zset_member_buffer.clone(), score));
                            
                            // Sort by score
                            members.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                            
                            // Update selection to new member
                            if let Some(pos) = members.iter().position(|(m, _)| m == &self.zset_member_buffer) {
                                self.zset_member_index = pos;
                                self.selected_zset_member = Some(self.zset_member_buffer.clone());
                            }
                            
                            self.has_unsaved_changes = true;
                        }
                    }
                    ZSetEditMode::UpdateScore => {
                        // Update score of existing member
                        if let Some(member) = &self.selected_zset_member {
                            let new_score = self.zset_score_buffer.parse::<f64>().unwrap_or(0.0);
                            
                            if let Some(pos) = members.iter().position(|(m, _)| m == member) {
                                members[pos].1 = new_score;
                                
                                // Re-sort by score
                                members.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                                
                                // Update selection to maintain member selection
                                if let Some(new_pos) = members.iter().position(|(m, _)| m == member) {
                                    self.zset_member_index = new_pos;
                                }
                                
                                self.has_unsaved_changes = true;
                            }
                        }
                    }
                    ZSetEditMode::Remove => {
                        // Remove selected member
                        if let Some(member) = &self.selected_zset_member {
                            if let Some(pos) = members.iter().position(|(m, _)| m == member) {
                                members.remove(pos);
                                
                                // Update selection
                                if members.is_empty() {
                                    self.selected_zset_member = None;
                                    self.zset_member_index = 0;
                                } else {
                                    self.zset_member_index = self.zset_member_index.min(members.len() - 1);
                                    self.selected_zset_member = Some(members[self.zset_member_index].0.clone());
                                }
                                
                                self.has_unsaved_changes = true;
                            }
                        }
                    }
                    ZSetEditMode::None => {}
                }
            }
        }
    }
    
    /// Validate score input
    pub fn is_valid_score(&self) -> bool {
        self.zset_score_buffer.parse::<f64>().is_ok()
    }
    
    // Stream viewing methods
    
    /// Select a stream entry by index
    pub fn select_stream_entry(&mut self, index: usize) {
        if let Some(crate::redis::value_types::RedisValue::Stream(entries)) = &self.value {
            if index < entries.len() {
                self.stream_entry_index = index;
                self.selected_stream_entry = Some(entries[index].id.clone());
                self.stream_field_index = 0; // Reset field selection
            }
        }
    }
    
    /// Toggle stream view mode between List and Detail
    pub fn toggle_stream_view_mode(&mut self) {
        self.stream_view_mode = match self.stream_view_mode {
            StreamViewMode::List => StreamViewMode::Detail,
            StreamViewMode::Detail => StreamViewMode::List,
        };
    }
    
    /// Move to next stream entry
    pub fn next_stream_entry(&mut self) {
        if let Some(crate::redis::value_types::RedisValue::Stream(entries)) = &self.value {
            if self.stream_entry_index + 1 < entries.len() {
                self.stream_entry_index += 1;
                self.selected_stream_entry = Some(entries[self.stream_entry_index].id.clone());
                self.stream_field_index = 0; // Reset field selection
            }
        }
    }
    
    /// Move to previous stream entry
    pub fn prev_stream_entry(&mut self) {
        if self.stream_entry_index > 0 {
            self.stream_entry_index -= 1;
            if let Some(crate::redis::value_types::RedisValue::Stream(entries)) = &self.value {
                self.selected_stream_entry = Some(entries[self.stream_entry_index].id.clone());
                self.stream_field_index = 0; // Reset field selection
            }
        }
    }
    
    /// Move to next field in detail view
    pub fn next_stream_field(&mut self) {
        if self.stream_view_mode == StreamViewMode::Detail {
            if let Some(crate::redis::value_types::RedisValue::Stream(entries)) = &self.value {
                if self.stream_entry_index < entries.len() {
                    let entry = &entries[self.stream_entry_index];
                    if self.stream_field_index + 1 < entry.fields.len() {
                        self.stream_field_index += 1;
                    }
                }
            }
        }
    }
    
    /// Move to previous field in detail view
    pub fn prev_stream_field(&mut self) {
        if self.stream_view_mode == StreamViewMode::Detail && self.stream_field_index > 0 {
            self.stream_field_index -= 1;
        }
    }
    
    /// Get current stream entry details
    pub fn get_current_stream_entry(&self) -> Option<&crate::redis::value_types::StreamEntry> {
        if let Some(crate::redis::value_types::RedisValue::Stream(entries)) = &self.value {
            entries.get(self.stream_entry_index)
        } else {
            None
        }
    }
    
    /// Get stream entry count
    pub fn get_stream_entry_count(&self) -> usize {
        if let Some(crate::redis::value_types::RedisValue::Stream(entries)) = &self.value {
            entries.len()
        } else {
            0
        }
    }
    
    /// Cycle through binary display modes
    pub fn cycle_binary_display_mode(&mut self) {
        self.binary_display_mode = match self.binary_display_mode {
            crate::ui::binary_viewer::DisplayMode::Auto => crate::ui::binary_viewer::DisplayMode::Text,
            crate::ui::binary_viewer::DisplayMode::Text => crate::ui::binary_viewer::DisplayMode::Hex,
            crate::ui::binary_viewer::DisplayMode::Hex => crate::ui::binary_viewer::DisplayMode::Base64,
            crate::ui::binary_viewer::DisplayMode::Base64 => crate::ui::binary_viewer::DisplayMode::Auto,
        };
    }
    
    /// Toggle JSON syntax highlighting
    pub fn toggle_json_highlighting(&mut self) {
        self.json_highlighting_enabled = !self.json_highlighting_enabled;
    }
    
    /// Check if current value contains binary data
    pub fn has_binary_data(&self) -> bool {
        if let Some(crate::redis::value_types::RedisValue::String(s)) = &self.value {
            let info = crate::ui::binary_viewer::BinaryViewer::analyze_data(s.as_bytes());
            info.has_binary_content
        } else {
            false
        }
    }
    
    /// Get current value as bytes for binary display
    pub fn get_value_bytes(&self) -> Option<&[u8]> {
        if let Some(crate::redis::value_types::RedisValue::String(s)) = &self.value {
            Some(s.as_bytes())
        } else {
            None
        }
    }
}

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

/// State for connection creation dialog
#[derive(Debug, Default)]
pub struct ConnectionDialogState {
    /// Whether the dialog is open
    pub is_open: bool,
    /// Currently focused field
    pub focused_field: ConnectionDialogField,
    /// Connection form data
    pub form: ConnectionFormData,
}

/// Fields in the connection dialog
#[derive(Debug, Default, Clone, PartialEq)]
pub enum ConnectionDialogField {
    #[default]
    Name,
    Host,
    Port,
    Password,
    Database,
    Buttons, // Save/Cancel buttons
}

/// Form data for connection creation
#[derive(Debug, Default, Clone)]
pub struct ConnectionFormData {
    pub name: String,
    pub host: String,
    pub port: String,
    pub password: String,
    pub database: String,
    pub ssl: bool,
}

/// Result of a Redis command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub command: String,
    pub result: Result<String, String>,
    pub timestamp: std::time::SystemTime,
}

impl AppState {
    /// Create a new application state
    pub fn new(config: AppConfig) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Self {
            running: true,
            current_view: ViewMode::ConnectionList,
            active_connection: None,
            connections: HashMap::new(),
            selected_database: None,
            selected_key: None,
            config,
            event_rx: Some(event_rx),
            event_tx,
            status_message: None,
            ui_state: UiState::default(),
        }
    }

    /// Set the current view mode
    pub fn set_view(&mut self, view: ViewMode) {
        self.current_view = view;
    }

    /// Set status message
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Get the currently active connection
    pub fn get_active_connection(&self) -> Option<&RedisConnection> {
        self.active_connection.as_ref()
            .and_then(|id| self.connections.get(id))
    }

    /// Get mutable reference to active connection
    pub fn get_active_connection_mut(&mut self) -> Option<&mut RedisConnection> {
        self.active_connection.clone()
            .and_then(|id| self.connections.get_mut(&id))
    }

    /// Add a new Redis connection
    pub fn add_connection(&mut self, id: String, connection: RedisConnection) {
        self.connections.insert(id.clone(), connection);
        if self.active_connection.is_none() {
            self.active_connection = Some(id);
        }
    }

    /// Remove a Redis connection
    pub fn remove_connection(&mut self, id: &str) -> Option<RedisConnection> {
        let connection = self.connections.remove(id);
        if self.active_connection.as_ref() == Some(&id.to_string()) {
            self.active_connection = self.connections.keys().next().cloned();
        }
        connection
    }

    /// Set the active connection
    pub fn set_active_connection(&mut self, id: String) -> AppResult<()> {
        if self.connections.contains_key(&id) {
            self.active_connection = Some(id);
            Ok(())
        } else {
            Err(crate::error::AppError::Generic(format!("Connection {} not found", id)))
        }
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Move focus to next panel
    pub fn next_panel(&mut self) {
        self.ui_state.focused_panel = match self.ui_state.focused_panel {
            FocusedPanel::ConnectionList => FocusedPanel::DatabaseBrowser,
            FocusedPanel::DatabaseBrowser => FocusedPanel::KeyViewer,
            FocusedPanel::KeyViewer => FocusedPanel::CommandInput,
            FocusedPanel::CommandInput => FocusedPanel::ConnectionList,
        };
    }

    /// Move focus to previous panel
    pub fn previous_panel(&mut self) {
        self.ui_state.focused_panel = match self.ui_state.focused_panel {
            FocusedPanel::ConnectionList => FocusedPanel::CommandInput,
            FocusedPanel::DatabaseBrowser => FocusedPanel::ConnectionList,
            FocusedPanel::KeyViewer => FocusedPanel::DatabaseBrowser,
            FocusedPanel::CommandInput => FocusedPanel::KeyViewer,
        };
    }

    /// Open connection creation dialog
    pub fn open_connection_dialog(&mut self) {
        self.ui_state.connection_dialog.is_open = true;
        self.ui_state.connection_dialog.focused_field = ConnectionDialogField::Name;
        // Pre-fill with defaults
        self.ui_state.connection_dialog.form = ConnectionFormData {
            name: "localhost".to_string(),
            host: "127.0.0.1".to_string(),
            port: "6379".to_string(),
            password: String::new(),
            database: "0".to_string(),
            ssl: false,
        };
    }

    /// Close connection creation dialog
    pub fn close_connection_dialog(&mut self) {
        self.ui_state.connection_dialog.is_open = false;
    }

    /// Move to next field in connection dialog
    pub fn next_dialog_field(&mut self) {
        self.ui_state.connection_dialog.focused_field = match self.ui_state.connection_dialog.focused_field {
            ConnectionDialogField::Name => ConnectionDialogField::Host,
            ConnectionDialogField::Host => ConnectionDialogField::Port,
            ConnectionDialogField::Port => ConnectionDialogField::Password,
            ConnectionDialogField::Password => ConnectionDialogField::Database,
            ConnectionDialogField::Database => ConnectionDialogField::Buttons,
            ConnectionDialogField::Buttons => ConnectionDialogField::Name,
        };
    }

    /// Move to previous field in connection dialog
    pub fn previous_dialog_field(&mut self) {
        self.ui_state.connection_dialog.focused_field = match self.ui_state.connection_dialog.focused_field {
            ConnectionDialogField::Name => ConnectionDialogField::Buttons,
            ConnectionDialogField::Host => ConnectionDialogField::Name,
            ConnectionDialogField::Port => ConnectionDialogField::Host,
            ConnectionDialogField::Password => ConnectionDialogField::Port,
            ConnectionDialogField::Database => ConnectionDialogField::Password,
            ConnectionDialogField::Buttons => ConnectionDialogField::Database,
        };
    }

    /// Update current field value in connection dialog
    pub fn update_dialog_field(&mut self, ch: char) {
        let form = &mut self.ui_state.connection_dialog.form;
        match self.ui_state.connection_dialog.focused_field {
            ConnectionDialogField::Name => form.name.push(ch),
            ConnectionDialogField::Host => form.host.push(ch),
            ConnectionDialogField::Port => {
                if ch.is_ascii_digit() {
                    form.port.push(ch);
                }
            }
            ConnectionDialogField::Password => form.password.push(ch),
            ConnectionDialogField::Database => {
                if ch.is_ascii_digit() {
                    form.database.push(ch);
                }
            }
            ConnectionDialogField::Buttons => {} // No text input for buttons
        }
    }

    /// Backspace in current field
    pub fn backspace_dialog_field(&mut self) {
        let form = &mut self.ui_state.connection_dialog.form;
        match self.ui_state.connection_dialog.focused_field {
            ConnectionDialogField::Name => { form.name.pop(); }
            ConnectionDialogField::Host => { form.host.pop(); }
            ConnectionDialogField::Port => { form.port.pop(); }
            ConnectionDialogField::Password => { form.password.pop(); }
            ConnectionDialogField::Database => { form.database.pop(); }
            ConnectionDialogField::Buttons => {}
        }
    }

    /// Create connection from dialog form
    pub async fn create_connection_from_dialog(&mut self) -> AppResult<()> {
        let form = self.ui_state.connection_dialog.form.clone();
        
        // Validate form data
        if form.name.trim().is_empty() {
            return Err(crate::error::AppError::Config("Connection name cannot be empty".to_string()));
        }
        if form.host.trim().is_empty() {
            return Err(crate::error::AppError::Config("Host cannot be empty".to_string()));
        }
        
        let port: u16 = form.port.parse()
            .map_err(|_| crate::error::AppError::Config("Invalid port number".to_string()))?;
        
        let database: u8 = form.database.parse()
            .map_err(|_| crate::error::AppError::Config("Invalid database number".to_string()))?;
        
        // Create connection config
        let connection_config = ConnectionConfig {
            name: form.name.clone(),
            host: form.host.clone(),
            port,
            password: if form.password.is_empty() { None } else { Some(form.password.clone()) },
            username: None,
            database,
            ssl: form.ssl,
            timeout: 5,
        };
        
        // Create Redis connection
        let mut redis_connection = crate::redis::RedisConnection::new(connection_config.clone())?;
        
        // Try to connect
        redis_connection.connect().await?;
        
        // Generate unique ID for connection
        let connection_id = uuid::Uuid::new_v4().to_string();
        let connection_id_for_config = connection_id.clone();
        let connection_id_for_event = connection_id.clone();
        
        // Add to connections
        self.add_connection(connection_id, redis_connection);
        
        // Add to config
        self.config.add_connection(connection_id_for_config, connection_config);
        
        // Close dialog
        self.close_connection_dialog();
        
        // Set status message
        self.set_status(format!("Connected to {}", form.name));
        
        // Trigger database browser initialization
        let _ = self.event_tx.send(crate::events::AppEvent::ConnectionStatusChanged {
            connection_id: connection_id_for_event,
            status: crate::redis::ConnectionStatus::Connected,
        });
        
        Ok(())
    }
    
    /// Load available databases for active connection
    pub async fn load_databases(&mut self) -> AppResult<()> {
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.get_databases().await {
                Ok(databases) => {
                    self.ui_state.database_browser.databases = databases;
                    self.set_status(format!("Found {} databases", self.ui_state.database_browser.databases.len()));
                }
                Err(err) => {
                    self.set_status(format!("Failed to load databases: {}", err));
                }
            }
        }
        Ok(())
    }
    
    /// Select a database
    pub async fn select_database(&mut self, db_num: u8) -> AppResult<()> {
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.select_database(db_num).await {
                Ok(()) => {
                    self.ui_state.database_browser.selected_database = db_num;
                    self.selected_database = Some(db_num);
                    // Clear current keys and reset scanning
                    self.ui_state.database_browser.keys.clear();
                    self.ui_state.database_browser.scan_cursor = 0;
                    self.ui_state.database_browser.scan_complete = false;
                    self.ui_state.database_browser.selected_key_index = 0;
                    // Load keys for the new database silently
                    self.load_keys_silent().await?;
                    self.set_status(format!("Selected database {}", db_num));
                }
                Err(err) => {
                    self.set_status(format!("Failed to select database {}: {}", db_num, err));
                }
            }
        }
        Ok(())
    }
    
    /// Load keys from current database with progress dialog
    pub async fn load_keys(&mut self) -> AppResult<()> {
        self.load_keys_internal(true).await
    }
    
    /// Load keys silently without progress dialog (for initial connection)
    pub async fn load_keys_silent(&mut self) -> AppResult<()> {
        self.load_keys_internal(false).await
    }
    
    /// Internal method to load keys with optional progress dialog
    async fn load_keys_internal(&mut self, show_progress: bool) -> AppResult<()> {
        if self.ui_state.database_browser.loading {
            return Ok(()); // Already loading
        }
        
        self.ui_state.database_browser.loading = true;
        
        // Conditionally start progress for key scanning
        let progress_index = if show_progress {
            Some(self.start_progress(
                crate::ui::progress_bar::ProgressType::DataLoading,
                "Loading Keys".to_string(),
                0, // Unknown total initially
                false // Cannot cancel Redis SCAN
            ))
        } else {
            None
        };
        
        // Extract values to avoid borrowing conflicts
        let pattern = if self.ui_state.database_browser.filter_pattern.is_empty() {
            "*".to_string()
        } else {
            format!("*{}*", self.ui_state.database_browser.filter_pattern)
        };
        
        let scan_cursor = self.ui_state.database_browser.scan_cursor;
        let keys_per_page = self.config.preferences.keys_per_page;
        
        // Get connection ID for later reference
        let connection_id = self.active_connection.clone();
        
        if let Some(progress_index) = progress_index {
            self.update_progress(progress_index, 0, "Starting key scan...".to_string());
        }
        
        // Perform scan operation
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.scan_keys(scan_cursor, &pattern, keys_per_page).await {
                Ok((new_cursor, key_names)) => {
                    // Update scan state
                    self.ui_state.database_browser.scan_cursor = new_cursor;
                    if new_cursor == 0 {
                        self.ui_state.database_browser.scan_complete = true;
                    }
                    
                    if let Some(progress_index) = progress_index {
                        self.update_progress(
                            progress_index, 
                            key_names.len() as u64, 
                            format!("Processing {} keys...", key_names.len())
                        );
                    }
                    
                    if !key_names.is_empty() {
                        // For now, create KeyInfo without type information
                        // We'll add type detection as a separate operation
                        let mut key_infos = Vec::new();
                        for key_name in key_names {
                            let key_info = KeyInfo {
                                name: key_name,
                                key_type: None, // Will be loaded separately
                                ttl: None,      // Will be loaded separately
                                size: None,
                                matches_filter: true,
                            };
                            key_infos.push(key_info);
                        }
                        
                        // Append new keys to existing ones
                        self.ui_state.database_browser.keys.extend(key_infos);
                        
                        // Rebuild tree view if enabled
                        if self.ui_state.database_browser.use_tree_view {
                            self.rebuild_tree_view();
                        }
                        
                        let final_status = format!(
                            "Loaded {} keys", 
                            self.ui_state.database_browser.keys.len()
                        );
                        
                        if let Some(progress_index) = progress_index {
                            self.complete_progress(progress_index, final_status.clone());
                        }
                        self.set_status(final_status);
                        
                        // Load types and TTLs for the first few keys asynchronously
                        self.load_key_details().await?;
                    } else {
                        if let Some(progress_index) = progress_index {
                            self.complete_progress(progress_index, "No keys found".to_string());
                        }
                        self.set_status("No keys found".to_string());
                    }
                }
                Err(err) => {
                    let error_msg = format!("Failed to scan keys: {}", err);
                    if let Some(progress_index) = progress_index {
                        self.complete_progress(progress_index, error_msg.clone());
                    }
                    self.set_status(error_msg);
                }
            }
        } else {
            if let Some(progress_index) = progress_index {
                self.complete_progress(progress_index, "No active connection".to_string());
            }
        }
        
        self.ui_state.database_browser.loading = false;
        
        // Schedule progress bar removal if we showed one
        if let Some(progress_index) = progress_index {
            self.schedule_progress_removal(progress_index, 1500);
        }
        
        Ok(())
    }
    
    /// Load type and TTL information for keys that don't have it yet
    pub async fn load_key_details(&mut self) -> AppResult<()> {
        // Load details for up to 10 keys at a time to avoid blocking UI
        let mut keys_to_process = Vec::new();
        let mut indices_to_update = Vec::new();
        
        for (idx, key_info) in self.ui_state.database_browser.keys.iter().enumerate() {
            if key_info.key_type.is_none() && keys_to_process.len() < 10 {
                keys_to_process.push(key_info.name.clone());
                indices_to_update.push(idx);
            }
        }
        
        if keys_to_process.is_empty() {
            return Ok(());
        }
        
        // Load key information
        if let Some(connection) = self.get_active_connection_mut() {
            match connection.get_keys_info(&keys_to_process).await {
                Ok(key_infos_data) => {
                    let mut types_loaded = 0;
                    let mut ttls_loaded = 0;
                    
                    // Update the key information
                    for ((_, key_type, ttl), &idx) in key_infos_data.iter().zip(indices_to_update.iter()) {
                        if let Some(key_info) = self.ui_state.database_browser.keys.get_mut(idx) {
                            key_info.key_type = key_type.clone();
                            key_info.ttl = *ttl;
                            
                            if key_type.is_some() {
                                types_loaded += 1;
                            }
                            if ttl.is_some() {
                                ttls_loaded += 1;
                            }
                        }
                    }
                    
                    if types_loaded > 0 || ttls_loaded > 0 {
                        self.set_status(format!(
                            "Loaded details: {} types, {} TTLs", 
                            types_loaded, ttls_loaded
                        ));
                    }
                }
                Err(err) => {
                    self.set_status(format!("Failed to load key details: {}", err));
                }
            }
        }
        
        Ok(())
    }
    
    /// Load more keys (pagination)
    pub async fn load_more_keys(&mut self) -> AppResult<()> {
        if !self.ui_state.database_browser.scan_complete {
            self.load_keys().await?
        }
        Ok(())
    }
    
    /// Schedule key loading without blocking UI - for responsive navigation
    pub fn schedule_key_loading(&mut self) -> AppResult<()> {
        if !self.ui_state.database_browser.loading && !self.ui_state.database_browser.scan_complete {
            // Send an async event to load more keys in the background
            let _ = self.event_tx.send(crate::events::AppEvent::RefreshData);
        }
        Ok(())
    }
    
    /// Select next key in the browser - optimized for performance
    pub fn select_next_key(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        
        if browser.use_tree_view {
            // Tree view navigation
            let visible_count = browser.key_tree.visible_count();
            if visible_count > 0 {
                let old_index = browser.selected_key_index;
                browser.selected_key_index = (browser.selected_key_index + 1).min(visible_count - 1);
                
                if old_index != browser.selected_key_index {
                    // Adjust scroll offset if needed
                    let display_count = 10;
                    if browser.selected_key_index >= browser.scroll_offset + display_count {
                        browser.scroll_offset = browser.selected_key_index - display_count + 1;
                    }
                    
                    // Update selected key from tree
                    if let Some(display_info) = browser.key_tree.get_visible_node_info(browser.selected_key_index) {
                        if display_info.is_key {
                            if let Some(key_info) = &display_info.key_info {
                                self.selected_key = Some(key_info.name.clone());
                            }
                        } else {
                            // For non-key nodes, clear selected key
                            self.selected_key = None;
                        }
                    }
                }
            }
        } else {
            // Flat list navigation
            if !browser.keys.is_empty() {
                let old_index = browser.selected_key_index;
                browser.selected_key_index = (browser.selected_key_index + 1).min(browser.keys.len() - 1);
                
                // Only update if index actually changed
                if old_index != browser.selected_key_index {
                    // Adjust scroll offset if needed
                    let visible_count = 10; // Number of keys visible at once
                    if browser.selected_key_index >= browser.scroll_offset + visible_count {
                        browser.scroll_offset = browser.selected_key_index - visible_count + 1;
                    }
                    
                    // Update selected key - use reference to avoid cloning when possible
                    if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                        self.selected_key = Some(key_info.name.clone());
                    }
                }
            }
        }
    }
    
    /// Select previous key in the browser - optimized for performance
    pub fn select_previous_key(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        
        if browser.use_tree_view {
            // Tree view navigation
            if browser.selected_key_index > 0 {
                let old_index = browser.selected_key_index;
                browser.selected_key_index -= 1;
                
                if old_index != browser.selected_key_index {
                    // Adjust scroll offset if needed
                    if browser.selected_key_index < browser.scroll_offset {
                        browser.scroll_offset = browser.selected_key_index;
                    }
                    
                    // Update selected key from tree
                    if let Some(display_info) = browser.key_tree.get_visible_node_info(browser.selected_key_index) {
                        if display_info.is_key {
                            if let Some(key_info) = &display_info.key_info {
                                self.selected_key = Some(key_info.name.clone());
                            }
                        } else {
                            // For non-key nodes, clear selected key
                            self.selected_key = None;
                        }
                    }
                }
            }
        } else {
            // Flat list navigation
            if browser.selected_key_index > 0 {
                let old_index = browser.selected_key_index;
                browser.selected_key_index -= 1;
                
                // Only update if index actually changed
                if old_index != browser.selected_key_index {
                    // Adjust scroll offset if needed
                    if browser.selected_key_index < browser.scroll_offset {
                        browser.scroll_offset = browser.selected_key_index;
                    }
                    
                    // Update selected key - use reference to avoid cloning when possible
                    if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                        self.selected_key = Some(key_info.name.clone());
                    }
                }
            }
        }
    }
    
    /// Select key by offset for efficient page navigation
    pub fn select_key_by_offset(&mut self, offset: i32) {
        let browser = &mut self.ui_state.database_browser;
        if browser.keys.is_empty() {
            return;
        }
        
        let old_index = browser.selected_key_index;
        let new_index = if offset < 0 {
            browser.selected_key_index.saturating_sub((-offset) as usize)
        } else {
            (browser.selected_key_index + offset as usize).min(browser.keys.len() - 1)
        };
        
        if old_index != new_index {
            browser.selected_key_index = new_index;
            
            // Adjust scroll offset for the new position
            let visible_count = 10;
            if browser.selected_key_index >= browser.scroll_offset + visible_count {
                browser.scroll_offset = browser.selected_key_index - visible_count + 1;
            } else if browser.selected_key_index < browser.scroll_offset {
                browser.scroll_offset = browser.selected_key_index;
            }
            
            // Update selected key
            if let Some(key_info) = browser.keys.get(browser.selected_key_index) {
                self.selected_key = Some(key_info.name.clone());
            }
        }
    }
    
    /// Set filter pattern for key search
    pub async fn set_key_filter(&mut self, pattern: String) -> AppResult<()> {
        self.ui_state.database_browser.filter_pattern = pattern;
        // Reset scanning and reload keys with new filter
        self.ui_state.database_browser.keys.clear();
        self.ui_state.database_browser.scan_cursor = 0;
        self.ui_state.database_browser.scan_complete = false;
        self.ui_state.database_browser.selected_key_index = 0;
        self.load_keys().await
    }
    
    /// Get currently selected key info (works for both tree and flat view)
    pub fn get_selected_key_info(&self) -> Option<&KeyInfo> {
        let browser = &self.ui_state.database_browser;
        
        if browser.use_tree_view {
            // In tree view, get key info from tree structure
            browser.key_tree.get_key_info_at_index(browser.selected_key_index)
        } else {
            // In flat view, get from keys vector
            browser.keys.get(browser.selected_key_index)
        }
    }
    
    /// Enter search mode for key filtering
    pub fn enter_search_mode(&mut self) {
        self.ui_state.database_browser.search_mode = true;
        self.ui_state.database_browser.filter_pattern.clear();
    }
    
    /// Exit search mode
    pub fn exit_search_mode(&mut self) {
        self.ui_state.database_browser.search_mode = false;
        if !self.ui_state.database_browser.filter_pattern.is_empty() {
            // Clear filter and reload all keys
            self.ui_state.database_browser.filter_pattern.clear();
            // Reset scanning state
            self.ui_state.database_browser.keys.clear();
            self.ui_state.database_browser.scan_cursor = 0;
            self.ui_state.database_browser.scan_complete = false;
            self.ui_state.database_browser.selected_key_index = 0;
        }
    }
    
    /// Rebuild tree view from current keys
    pub fn rebuild_tree_view(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        browser.key_tree.build_from_keys(&browser.keys);
        
        // Update selected index to match current key in tree
        if let Some(selected_key) = &self.selected_key {
            if let Some(tree_index) = browser.key_tree.find_key_index(selected_key) {
                browser.selected_key_index = tree_index;
            }
        }
    }
    
    /// Toggle tree view mode
    pub fn toggle_tree_view(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        browser.use_tree_view = !browser.use_tree_view;
        
        if browser.use_tree_view {
            // Build tree from current keys
            browser.key_tree.build_from_keys(&browser.keys);
            
            // Update selected index to match current key in tree
            if let Some(selected_key) = &self.selected_key {
                if let Some(tree_index) = browser.key_tree.find_key_index(selected_key) {
                    browser.selected_key_index = tree_index;
                }
            }
        }
    }
    
    /// Toggle node expansion in tree view
    pub fn toggle_tree_node(&mut self) {
        let browser = &mut self.ui_state.database_browser;
        if browser.use_tree_view {
            if browser.key_tree.toggle_node_at_index(browser.selected_key_index) {
                // Tree was rebuilt, may need to adjust selected index
                let visible_count = browser.key_tree.visible_count();
                if browser.selected_key_index >= visible_count && visible_count > 0 {
                    browser.selected_key_index = visible_count - 1;
                }
                
                // Update scroll offset if needed
                let display_count = 10;
                if browser.selected_key_index >= browser.scroll_offset + display_count {
                    browser.scroll_offset = browser.selected_key_index - display_count + 1;
                } else if browser.selected_key_index < browser.scroll_offset {
                    browser.scroll_offset = browser.selected_key_index;
                }
            }
        }
    }
    
    /// Add character to search pattern
    pub fn add_search_char(&mut self, ch: char) {
        if self.ui_state.database_browser.search_mode {
            self.ui_state.database_browser.filter_pattern.push(ch);
        }
    }
    
    /// Remove last character from search pattern
    pub fn backspace_search(&mut self) {
        if self.ui_state.database_browser.search_mode {
            self.ui_state.database_browser.filter_pattern.pop();
        }
    }
    
    /// Apply current search filter
    pub async fn apply_search_filter(&mut self) -> AppResult<()> {
        if self.ui_state.database_browser.search_mode {
            // Reset scanning state and search with new pattern
            self.ui_state.database_browser.keys.clear();
            self.ui_state.database_browser.scan_cursor = 0;
            self.ui_state.database_browser.scan_complete = false;
            self.ui_state.database_browser.selected_key_index = 0;
            // Load keys with filter
            self.load_keys().await?;
            // Exit search mode after applying
            self.ui_state.database_browser.search_mode = false;
        }
        Ok(())
    }
    
    // Confirmation dialog methods
    
    /// Open confirmation dialog for saving changes
    pub fn confirm_save_changes(&mut self, key_name: String, old_value: String, new_value: String) {
        let old_summary = Self::create_value_summary(&old_value);
        let new_summary = Self::create_value_summary(&new_value);
        
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::SaveChanges {
            key_name,
            old_value_summary: old_summary,
            new_value_summary: new_summary,
        });
    }
    
    /// Open confirmation dialog for deleting a key
    pub fn confirm_delete_key(&mut self, key_name: String, key_type: String) {
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::DeleteKey {
            key_name,
            key_type,
        });
    }
    
    /// Open confirmation dialog for discarding changes
    pub fn confirm_discard_changes(&mut self, key_name: String) {
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::DiscardChanges {
            key_name,
        });
    }
    
    /// Open confirmation dialog for large value edit
    pub fn confirm_large_value_edit(&mut self, key_name: String, size: usize) {
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::LargeValueEdit {
            key_name,
            size,
        });
    }
    
    /// Open confirmation dialog for binary data edit
    pub fn confirm_binary_data_edit(&mut self, key_name: String, binary_info: String) {
        self.ui_state.confirmation_dialog.open(crate::ui::ConfirmationType::BinaryDataEdit {
            key_name,
            binary_info,
        });
    }
    
    /// Check if edit requires confirmation and show dialog if needed
    pub fn validate_edit_and_confirm(&mut self, key_name: &str) -> bool {
        let viewer_state = &self.ui_state.key_viewer;
        
        // Check for large value
        if viewer_state.edit_buffer.len() > 1024 * 1024 {  // 1MB
            self.confirm_large_value_edit(key_name.to_string(), viewer_state.edit_buffer.len());
            return false; // Need confirmation
        }
        
        // Check for binary data
        if viewer_state.has_binary_data() {
            let binary_info = crate::ui::BinaryViewer::analyze_data(viewer_state.edit_buffer.as_bytes());
            let info_text = format!("{} null bytes, {} control chars", 
                                  binary_info.null_bytes, binary_info.control_chars);
            self.confirm_binary_data_edit(key_name.to_string(), info_text);
            return false; // Need confirmation
        }
        
        // Check validation errors
        let validation = viewer_state.validate_edit_buffer();
        if let crate::ui::ValidationResult::Error(msg) = validation {
            self.set_status(format!("Validation error: {}", msg));
            return false; // Cannot save invalid data
        }
        
        true // No confirmation needed
    }
    
    /// Handle confirmation dialog response
    pub fn handle_confirmation_response(&mut self) -> Option<crate::ui::ConfirmationResponse> {
        if self.ui_state.confirmation_dialog.is_open {
            let response = self.ui_state.confirmation_dialog.get_response();
            if response != crate::ui::ConfirmationResponse::Pending {
                self.ui_state.confirmation_dialog.close();
                Some(response)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Close confirmation dialog
    pub fn close_confirmation_dialog(&mut self) {
        self.ui_state.confirmation_dialog.close();
    }
    
    /// Create a summary of a value for display in confirmations
    fn create_value_summary(value: &str) -> String {
        if value.len() <= 50 {
            value.to_string()
        } else {
            format!("{}... ({} chars)", &value[..47], value.len())
        }
    }
    
    // Export/Import functionality
    
    /// Open export dialog for current key
    pub fn open_export_dialog(&mut self) {
        if let Some(key_name) = &self.ui_state.key_viewer.current_key {
            let default_path = format!("{}.json", key_name.replace(':', "_"));
            self.ui_state.export_import_dialog.open_export(default_path);
        }
    }
    
    /// Open import dialog
    pub fn open_import_dialog(&mut self) {
        let default_path = "import_data.json".to_string();
        self.ui_state.export_import_dialog.open_import(default_path);
    }
    
    /// Export current key value
    pub async fn export_current_key(&mut self) -> AppResult<()> {
        // Extract data first to avoid borrowing conflicts
        let (key_name, format) = {
            let dialog = &self.ui_state.export_import_dialog;
            if !dialog.is_open {
                return Ok(());
            }
            
            let key_name = match &self.ui_state.key_viewer.current_key {
                Some(key) => key.clone(),
                None => {
                    self.set_status("No key selected for export".to_string());
                    return Ok(());
                }
            };
            
            if self.ui_state.key_viewer.value.is_none() {
                self.set_status("No value loaded for export".to_string());
                return Ok(());
            }
            
            (key_name, dialog.selected_format.clone())
        };
        
        // Start progress for export operation
        let progress_index = self.start_progress(
            crate::ui::progress_bar::ProgressType::Transfer,
            format!("Exporting Key: {}", key_name),
            1, // Single key export
            false
        );
        
        self.update_progress(progress_index, 0, "Preparing export data...".to_string());
        
        let value = self.ui_state.key_viewer.value.as_ref().unwrap();
        let ttl = self.ui_state.key_viewer.metadata.as_ref()
            .and_then(|m| m.ttl)
            .filter(|&t| t > 0);
        
        // Simulate export processing with progress
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        match crate::utils::DataExporter::export_value(
            &key_name,
            value,
            ttl,
            &format,
        ) {
            Ok(exported_data) => {
                self.update_progress(progress_index, 1, "Writing export file...".to_string());
                
                // Simulate file writing delay
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                
                let final_status = format!(
                    "Exported '{}' to {} format",
                    key_name, format
                );
                
                self.complete_progress(progress_index, final_status.clone());
                let file_path = self.ui_state.export_import_dialog.file_path.clone();
                self.set_status(format!(
                    "Exported key '{}' to {} format (would save to '{}')",
                    key_name, format, file_path
                ));
                
                self.ui_state.export_import_dialog.close();
                
                // Remove progress bar after delay
                self.schedule_progress_removal(progress_index, 2000);
            }
            Err(err) => {
                let error_msg = format!("Export failed: {}", err);
                self.complete_progress(progress_index, error_msg.clone());
                self.set_status(error_msg);
                
                // Remove progress bar after error delay
                self.schedule_progress_removal(progress_index, 3000);
            }
        }
        
        Ok(())
    }
    
    /// Import data from file
    pub async fn import_data(&mut self) -> AppResult<()> {
        let dialog = &self.ui_state.export_import_dialog;
        if !dialog.is_open {
            return Ok(());
        }
        
        // In a real implementation, you would read from file here
        // For now, we'll just show a placeholder message
        self.set_status(format!(
            "Would import from '{}' using {} format",
            dialog.file_path, dialog.selected_format
        ));
        self.ui_state.export_import_dialog.close();
        
        Ok(())
    }
    
    // Bulk operations functionality
    
    /// Open bulk operations dialog with selected keys
    pub fn open_bulk_operations_dialog(&mut self, selected_keys: Vec<String>) {
        self.ui_state.bulk_operations_dialog.open(selected_keys);
    }
    
    /// Execute bulk operation
    pub async fn execute_bulk_operation(&mut self) -> AppResult<()> {
        // Extract data before borrowing
        let (operation, keys) = {
            let dialog = &self.ui_state.bulk_operations_dialog;
            if !dialog.is_open || dialog.selected_keys.is_empty() {
                return Ok(());
            }
            
            let operation = match dialog.get_current_operation() {
                Some(op) => op,
                None => {
                    self.set_status("No operation selected".to_string());
                    return Ok(());
                }
            };
            
            // Validate operation
            if let Err(err) = crate::utils::BulkOperationsManager::validate_operation(
                &operation, 
                &dialog.selected_keys
            ) {
                self.set_status(format!("Validation failed: {}", err));
                return Ok(());
            }
            
            (operation, dialog.selected_keys.clone())
        };
        
        // Start progress bar
        let operation_name = crate::utils::BulkOperationsManager::get_operation_description(&operation);
        let progress_index = self.start_progress(
            crate::ui::progress_bar::ProgressType::BulkOperation,
            format!("Bulk Operation: {}", operation_name),
            keys.len() as u64,
            true
        );
        
        // Start execution
        self.ui_state.bulk_operations_dialog.start_execution();
        
        // Get connection
        let connection = match self.get_active_connection_mut() {
            Some(conn) => conn,
            None => {
                self.set_status("No active connection".to_string());
                self.remove_progress(progress_index);
                return Ok(());
            }
        };
        
        // Create progress callback
        let mut progress_data = (0u64, 0u64, 0u64); // (completed, successful, failed)
        
        // Execute bulk operation
        let result: AppResult<crate::utils::bulk_operations::BulkOperationResult> = 
            crate::utils::BulkOperationsManager::execute_bulk_operation(
                connection,
                keys,
                operation.clone(),
                None, // No progress callback for now due to borrowing complexity
            ).await;
        
        match result {
            Ok(bulk_result) => {
                let final_status = format!(
                    "Completed: {} successful, {} failed in {:.2}s",
                    bulk_result.successful,
                    bulk_result.failed,
                    bulk_result.duration.as_secs_f64()
                );
                
                self.complete_progress(progress_index, final_status.clone());
                self.set_status(format!("Bulk operation completed: {}", final_status));
                
                // Close dialog on success
                self.ui_state.bulk_operations_dialog.close();
                
                // Refresh keys list if needed
                if matches!(operation, crate::utils::BulkOperation::Delete | 
                          crate::utils::BulkOperation::Rename { .. }) {
                    self.load_keys().await?;
                }
                
                // Remove progress bar after a short delay
                self.schedule_progress_removal(progress_index, 2000);
            }
            Err(err) => {
                self.complete_progress(progress_index, format!("Failed: {}", err));
                self.set_status(format!("Bulk operation failed: {}", err));
                self.ui_state.bulk_operations_dialog.close();
                
                // Remove progress bar after error display
                self.schedule_progress_removal(progress_index, 3000);
            }
        }
        
        Ok(())
    }
    
    /// Get selected keys for bulk operations
    pub fn get_selected_keys_for_bulk(&self) -> Vec<String> {
        // For now, just return current key if any
        // In a full implementation, you'd track multi-selection
        if let Some(key) = &self.selected_key {
            vec![key.clone()]
        } else {
            Vec::new()
        }
    }
    
    // Progress bar functionality
    
    /// Start a progress operation
    pub fn start_progress(
        &mut self, 
        progress_type: crate::ui::progress_bar::ProgressType, 
        title: String, 
        total: u64,
        can_cancel: bool
    ) -> usize {
        let progress_bar = crate::ui::progress_bar::ProgressBar::new(
            progress_type,
            title,
            total,
            can_cancel
        );
        
        self.ui_state.progress_bar_manager.add_progress_bar(progress_bar)
    }
    
    /// Update progress
    pub fn update_progress(&mut self, index: usize, completed: u64, status: String) {
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.update(completed, status);
        }
    }
    
    /// Update bulk operation progress
    pub fn update_bulk_progress(
        &mut self, 
        index: usize, 
        completed: u64, 
        successful: u64, 
        failed: u64, 
        current_op: String
    ) {
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.update_bulk(completed, successful, failed, current_op);
        }
    }
    
    /// Complete progress operation
    pub fn complete_progress(&mut self, index: usize, final_status: String) {
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.complete(final_status);
        }
    }
    
    /// Hide progress bar
    pub fn hide_progress(&mut self, index: usize) {
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.hide();
        }
    }
    
    /// Remove progress bar
    pub fn remove_progress(&mut self, index: usize) {
        self.ui_state.progress_bar_manager.remove_progress_bar(index);
    }
    
    /// Check if any progress bars are visible
    pub fn has_active_progress(&self) -> bool {
        self.ui_state.progress_bar_manager.get_active()
            .map(|pb| pb.is_visible)
            .unwrap_or(false)
    }
    
    /// Schedule progress bar removal after delay
    pub fn schedule_progress_removal(&mut self, index: usize, delay_ms: u64) {
        // Mark the progress as completed and allow manual dismissal
        if let Some(progress_bar) = self.ui_state.progress_bar_manager.progress_bars.get_mut(index) {
            progress_bar.can_cancel = true; // Allow manual dismissal
            
            // For completed operations, just hide immediately if they're done
            if progress_bar.progress >= 1.0 {
                progress_bar.hide();
            }
        }
    }
    
    /// Dismiss active progress bars (for Esc key)
    pub fn dismiss_progress_bars(&mut self) {
        self.ui_state.progress_bar_manager.hide_all();
    }
}