use crate::{ClipboardContent, HistoryItem, get_app_state, clipboard::storage};
use serde::Serialize;

#[derive(Serialize)]
pub struct CommandResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[tauri::command]
pub fn get_clipboard_content() -> CommandResult<ClipboardContent> {
    log::info!("get_clipboard_content called - reading real-time from clipboard");
    
    // 实时读取剪贴板，而不是返回缓存的内容
    let content = crate::clipboard::monitor::get_current_clipboard();
    
    log::info!("Clipboard content: {:?}", content.is_some());
    
    CommandResult {
        success: true,
        data: content,
        error: None,
    }
}

#[tauri::command]
pub fn get_history(count: Option<usize>) -> CommandResult<Vec<HistoryItem>> {
    let state = get_app_state();
    let history = state.history.lock().unwrap();
    
    let count = count.unwrap_or(20).min(history.len());
    let items: Vec<HistoryItem> = history.iter().take(count).cloned().collect();
    
    log::info!("get_history called, returning {} items", items.len());
    
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
    
    // 同时清空当前内容
    let mut current = state.current_content.lock().unwrap();
    *current = None;
    
    log::info!("clear_preview called");
    
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
    
    log::info!("toggle_monitoring called, new state: {}", new_state);
    
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
    
    log::info!("get_monitoring_status called: {}", is_monitoring);
    
    CommandResult {
        success: true,
        data: Some(is_monitoring),
        error: None,
    }
}

#[tauri::command]
pub fn open_data_directory() -> CommandResult<()> {
    log::info!("open_data_directory called");
    
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

#[tauri::command]
pub fn copy_to_clipboard(text: String) -> CommandResult<()> {
    log::info!("copy_to_clipboard called with text: {}", text);
    
    match clipboard_win::set_clipboard(clipboard_win::formats::Unicode, text.as_str()) {
        Ok(_) => {
            log::info!("Successfully copied text to clipboard using clipboard-win");
            CommandResult {
                success: true,
                data: Some(()),
                error: None,
            }
        }
        Err(e) => {
            log::error!("Failed to set text using clipboard-win: {}", e);
            CommandResult {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }
        }
    }
}
