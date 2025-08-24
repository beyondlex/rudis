use super::{
    focused_panel::FocusedPanel,
    connection_list_state::ConnectionListState,
    database_browser_state::DatabaseBrowserState,
    command_input_state::CommandInputState,
    connection_dialog_state::ConnectionDialogState,
};

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
    pub key_viewer: crate::app::states::KeyViewerState,
    
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