use crate::{ClipboardContent, HistoryItem, get_app_state, clipboard, clipboard::storage};
use serde::Serialize;

#[derive(Serialize)]
pub struct CommandResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[tauri::command]
pub fn get_clipboard_content() -> CommandResult<Option<ClipboardContent>> {
    let state = get_app_state();
    
    if state.preview_cleared.load(std::sync::atomic::Ordering::Relaxed) {
        return CommandResult {
            success: true,
            data: None,
            error: None,
        };
    }
    
    let content = clipboard::monitor::get_current_clipboard();
    
    CommandResult {
        success: true,
        data: Some(content),
        error: None,
    }
}

#[tauri::command]
pub fn get_history(count: Option<usize>) -> CommandResult<Vec<HistoryItem>> {
    let state = get_app_state();
    let history = state.history.lock().unwrap();
    
    let count = count.unwrap_or(20).min(history.len());
    let items: Vec<HistoryItem> = history.iter().take(count).cloned().collect();
    
    CommandResult {
        success: true,
        data: Some(items),
        error: None,
    }
}

#[tauri::command]
pub fn clear_preview() -> CommandResult<()> {
    let state = get_app_state();
    state.preview_cleared.store(true, std::sync::atomic::Ordering::Relaxed);
    
    CommandResult {
        success: true,
        data: Some(()),
        error: None,
    }
}

#[tauri::command]
pub fn toggle_monitoring() -> CommandResult<bool> {
    let state = get_app_state();
    let current = state.is_monitoring.load(std::sync::atomic::Ordering::Relaxed);
    let new_state = !current;
    state.is_monitoring.store(new_state, std::sync::atomic::Ordering::Relaxed);
    
    CommandResult {
        success: true,
        data: Some(new_state),
        error: None,
    }
}

#[tauri::command]
pub fn get_monitoring_status() -> CommandResult<bool> {
    let state = get_app_state();
    let is_monitoring = state.is_monitoring.load(std::sync::atomic::Ordering::Relaxed);
    
    CommandResult {
        success: true,
        data: Some(is_monitoring),
        error: None,
    }
}

#[tauri::command]
pub fn open_data_directory() -> CommandResult<()> {
    match storage::open_data_directory() {
        Ok(_) => CommandResult {
            success: true,
            data: Some(()),
            error: None,
        },
        Err(e) => CommandResult {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}
