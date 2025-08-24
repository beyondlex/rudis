use crate::app::state_core::{HashEditMode, ListEditMode, SetEditMode, StreamViewMode, ZSetEditMode, KeyMetadata};

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