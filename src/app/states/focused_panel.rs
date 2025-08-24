/// Which panel currently has focus
#[derive(Debug, Default, Clone, PartialEq)]
pub enum FocusedPanel {
    #[default]
    ConnectionList,
    DatabaseBrowser,
    KeyViewer,
    CommandInput,
} 