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