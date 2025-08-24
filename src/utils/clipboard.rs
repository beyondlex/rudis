use crate::error::AppResult;

/// Clipboard utilities for copying text to system clipboard
pub struct ClipboardUtils;

impl ClipboardUtils {
    /// Copy text to system clipboard
    pub fn copy_to_clipboard(text: &str) -> AppResult<()> {
        use arboard::Clipboard;
        
        let mut clipboard = Clipboard::new()
            .map_err(|e| crate::error::AppError::Generic(format!("Failed to access clipboard: {}", e)))?;
        
        clipboard.set_text(text)
            .map_err(|e| crate::error::AppError::Generic(format!("Failed to copy to clipboard: {}", e)))?;
        
        Ok(())
    }
    
    /// Get text from system clipboard
    pub fn get_from_clipboard() -> AppResult<String> {
        use arboard::Clipboard;
        
        let mut clipboard = Clipboard::new()
            .map_err(|e| crate::error::AppError::Generic(format!("Failed to access clipboard: {}", e)))?;
        
        clipboard.get_text()
            .map_err(|e| crate::error::AppError::Generic(format!("Failed to get from clipboard: {}", e)))
    }
}