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