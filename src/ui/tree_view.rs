use std::collections::HashMap;
use crate::app::KeyInfo;

/// Represents a node in the key tree
#[derive(Debug, Clone, Default)]
pub struct TreeNode {
    /// The name/label of this node (just the segment, not full path)
    pub name: String,
    /// Full key path (for actual Redis keys)
    pub full_path: Option<String>,
    /// Key information if this is a leaf node (actual key)
    pub key_info: Option<KeyInfo>,
    /// Child nodes
    pub children: HashMap<String, TreeNode>,
    /// Whether this node is expanded in the UI
    pub is_expanded: bool,
    /// Tree depth level (0 = root)
    pub depth: usize,
}

impl TreeNode {
    /// Create a new tree node
    pub fn new(name: String, depth: usize) -> Self {
        Self {
            name,
            full_path: None,
            key_info: None,
            children: HashMap::new(),
            is_expanded: true, // Expanded by default
            depth,
        }
    }
    
    /// Create a leaf node (actual Redis key)
    pub fn new_leaf(name: String, key_info: KeyInfo, depth: usize) -> Self {
        Self {
            name: name.clone(),
            full_path: Some(key_info.name.clone()),
            key_info: Some(key_info),
            children: HashMap::new(),
            is_expanded: false, // Leaves can't be expanded
            depth,
        }
    }
    
    /// Check if this node represents an actual Redis key
    pub fn is_key(&self) -> bool {
        self.key_info.is_some()
    }
    
    /// Check if this node is a leaf (actual Redis key with no children)
    /// Note: A node can be both a key AND have children (hybrid node)
    pub fn is_leaf(&self) -> bool {
        self.key_info.is_some() && self.children.is_empty()
    }
    
    /// Check if this node has children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
    
    /// Toggle expansion state
    pub fn toggle_expansion(&mut self) {
        if self.has_children() {
            self.is_expanded = !self.is_expanded;
        }
    }
    
    /// Get sorted list of child names
    pub fn sorted_children(&self) -> Vec<String> {
        let mut children: Vec<String> = self.children.keys().cloned().collect();
        children.sort();
        children
    }
}

/// Tree structure for organizing Redis keys hierarchically
#[derive(Debug, Default)]
pub struct KeyTree {
    /// Root node
    pub root: TreeNode,
    /// Separator for splitting key paths
    pub separator: String,
    /// Flattened list of visible nodes for navigation
    pub visible_nodes: Vec<String>, // Full paths to nodes
    /// Map of full paths to TreeNode references
    pub node_map: HashMap<String, Vec<String>>, // path -> segments to reach node
}

impl KeyTree {
    /// Create a new key tree with the specified separator
    pub fn new(separator: String) -> Self {
        Self {
            root: TreeNode::new("root".to_string(), 0),
            separator,
            visible_nodes: Vec::new(),
            node_map: HashMap::new(),
        }
    }
    
    /// Build tree from a list of keys
    pub fn build_from_keys(&mut self, keys: &[KeyInfo]) {
        // Clear existing tree
        self.root = TreeNode::new("root".to_string(), 0);
        self.visible_nodes.clear();
        self.node_map.clear();
        
        // Insert each key into the tree
        for key_info in keys {
            self.insert_key(key_info.clone());
        }
        
        // Build the flattened view for navigation
        self.rebuild_visible_nodes();
    }
    
    /// Insert a key into the tree
    fn insert_key(&mut self, key_info: KeyInfo) {
        let segments: Vec<String> = key_info.name
            .split(&self.separator)
            .map(|s| s.to_string())
            .collect();
        
        if segments.is_empty() {
            return;
        }
        
        // Navigate to the correct position and create intermediate nodes
        let mut current = &mut self.root;
        
        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;
            
            if is_last {
                // This is the actual key
                if let Some(existing_node) = current.children.get_mut(segment) {
                    // Node already exists, just add key info to it (hybrid node)
                    existing_node.key_info = Some(key_info.clone());
                    existing_node.full_path = Some(key_info.name.clone());
                } else {
                    // Create new leaf node
                    let leaf = TreeNode::new_leaf(segment.clone(), key_info.clone(), i + 1);
                    current.children.insert(segment.clone(), leaf);
                }
            } else {
                // This is an intermediate node - create if doesn't exist
                if !current.children.contains_key(segment) {
                    let intermediate = TreeNode::new(segment.clone(), i + 1);
                    current.children.insert(segment.clone(), intermediate);
                }
                
                // Move to the child node - this now works even if it was previously a leaf
                current = current.children.get_mut(segment).unwrap();
            }
        }
    }
    
    /// Rebuild the flattened list of visible nodes
    pub fn rebuild_visible_nodes(&mut self) {
        self.visible_nodes.clear();
        self.node_map.clear();
        
        // Create a temporary copy to avoid borrowing issues
        let root_clone = self.root.clone();
        self.collect_visible_nodes_recursive(&root_clone, Vec::new());
    }
    
    /// Recursively collect visible nodes for navigation
    fn collect_visible_nodes_recursive(&mut self, node: &TreeNode, path_segments: Vec<String>) {
        // Skip the root node itself
        if !path_segments.is_empty() {
            let full_path = path_segments.join(&self.separator);
            self.visible_nodes.push(full_path.clone());
            self.node_map.insert(full_path, path_segments.clone());
        }
        
        // Add children if this node is expanded
        if node.is_expanded {
            for child_name in node.sorted_children() {
                if let Some(child) = node.children.get(&child_name) {
                    let mut child_path = path_segments.clone();
                    child_path.push(child_name);
                    self.collect_visible_nodes_recursive(child, child_path);
                }
            }
        }
    }
    
    /// Get node at the specified path
    pub fn get_node(&self, path_segments: &[String]) -> Option<&TreeNode> {
        let mut current = &self.root;
        
        for segment in path_segments {
            match current.children.get(segment) {
                Some(child) => current = child,
                None => return None,
            }
        }
        
        Some(current)
    }
    
    /// Get mutable node at the specified path
    pub fn get_node_mut(&mut self, path_segments: &[String]) -> Option<&mut TreeNode> {
        let mut current = &mut self.root;
        
        for segment in path_segments {
            match current.children.get_mut(segment) {
                Some(child) => current = child,
                None => return None,
            }
        }
        
        Some(current)
    }
    
    /// Toggle expansion of a node at the given index in visible_nodes
    pub fn toggle_node_at_index(&mut self, index: usize) -> bool {
        if index >= self.visible_nodes.len() {
            return false;
        }
        
        let path = self.visible_nodes[index].clone();
        let segments = self.node_map.get(&path).cloned();
        
        if let Some(segments) = segments {
            if let Some(node) = self.get_node_mut(&segments) {
                node.toggle_expansion();
                self.rebuild_visible_nodes();
                return true;
            }
        }
        
        false
    }
    
    /// Get the number of visible nodes
    pub fn visible_count(&self) -> usize {
        self.visible_nodes.len()
    }
    
    /// Get visible node at index for display
    pub fn get_visible_node_info(&self, index: usize) -> Option<TreeDisplayInfo> {
        if index >= self.visible_nodes.len() {
            return None;
        }
        
        let path = &self.visible_nodes[index];
        let segments = self.node_map.get(path)?;
        let node = self.get_node(segments)?;
        
        Some(TreeDisplayInfo {
            name: node.name.clone(),
            full_path: path.clone(),
            depth: node.depth,
            is_leaf: node.is_leaf(),
            is_key: node.is_key(), // Add this field
            is_expanded: node.is_expanded,
            has_children: node.has_children(),
            key_info: node.key_info.clone(),
        })
    }
    
    /// Find the index of a specific key in the visible nodes
    pub fn find_key_index(&self, key_name: &str) -> Option<usize> {
        self.visible_nodes.iter().position(|path| {
            if let Some(segments) = self.node_map.get(path) {
                if let Some(node) = self.get_node(segments) {
                    if let Some(ref key_info) = node.key_info {
                        return key_info.name == key_name;
                    }
                }
            }
            false
        })
    }
    
    /// Get actual Redis key info at visible index (if it's a key)
    pub fn get_key_info_at_index(&self, index: usize) -> Option<&KeyInfo> {
        if index >= self.visible_nodes.len() {
            return None;
        }
        
        let path = &self.visible_nodes[index];
        let segments = self.node_map.get(path)?;
        let node = self.get_node(segments)?;
        
        if node.is_key() {
            node.key_info.as_ref()
        } else {
            None
        }
    }
}

/// Information about a tree node for display purposes
#[derive(Debug, Clone)]
pub struct TreeDisplayInfo {
    /// Display name (just the segment)
    pub name: String,
    /// Full path
    pub full_path: String,
    /// Tree depth for indentation
    pub depth: usize,
    /// Whether this is an actual Redis key (can have children too)
    pub is_key: bool,
    /// Whether this is an actual Redis key with no children
    pub is_leaf: bool,
    /// Whether this node is expanded (only relevant for non-leaves)
    pub is_expanded: bool,
    /// Whether this node has children
    pub has_children: bool,
    /// Key information if this is a key
    pub key_info: Option<KeyInfo>,
}