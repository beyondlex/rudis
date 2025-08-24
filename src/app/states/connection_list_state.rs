/// State for connection list panel
#[derive(Debug, Default)]
pub struct ConnectionListState {
    /// Currently selected connection index
    pub selected_index: usize,
    /// Scroll offset for the list
    pub scroll_offset: usize,
} 