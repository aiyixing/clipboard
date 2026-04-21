use crate::{ClipboardContent, HistoryItem, get_app_state, hooks};
use crate::clipboard::storage;
use chrono::Local;
use std::time::Duration;
use tauri::Emitter;

const MONITOR_INTERVAL: u64 = 500;
const MAX_HISTORY_ITEMS: usize = 100;

pub async fn start_monitor(app_handle: tauri::AppHandle) {
    let mut last_content: Option<ClipboardContent> = None;
    
    // 创建一个持久的剪贴板上下文
    let mut clipboard_ctx = match arboard::Clipboard::new() {
        Ok(ctx) => {
            log::info!("Clipboard context created successfully");
            Some(ctx)
        }
        Err(e) => {
            log::error!("Failed to create clipboard context: {}", e);
            None
        }
    };
    
    loop {
        let state = get_app_state();
        
        if !state.is_monitoring.load(std::sync::atomic::Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(MONITOR_INTERVAL)).await;
            continue;
        }
        
        let current_content = read_clipboard(&mut clipboard_ctx);
        
        if let Some(ref content) = current_content {
            if !is_duplicate(content, &last_content) {
                log::info!("New clipboard content detected");
                
                // 存储内容
                if let Err(e) = storage::save_content(content) {
                    log::error!("Failed to save content: {}", e);
                }
                
                // 提取链接
                if let ClipboardContent::Text(ref text) = content {
                    if let Err(e) = storage::extract_and_save_links(text) {
                        log::error!("Failed to extract links: {}", e);
                    }
                }
                
                // 触发钩子
                if let Err(e) = hooks::execute_hooks(content) {
                    log::error!("Failed to execute hooks: {}", e);
                }
                
                // 更新历史记录
                update_history(content.clone());
                
                // 通知前端
                log::info!("Emitting clipboard-changed event");
                match app_handle.emit("clipboard-changed", content.clone()) {
                    Ok(_) => log::info!("Event emitted successfully"),
                    Err(e) => log::error!("Failed to emit event: {}", e),
                }
                
                last_content = Some(content.clone());
            }
        }
        
        tokio::time::sleep(Duration::from_millis(MONITOR_INTERVAL)).await;
    }
}

fn read_clipboard(ctx: &mut Option<arboard::Clipboard>) -> Option<ClipboardContent> {
    // 如果上下文不存在，尝试创建一个新的
    if ctx.is_none() {
        match arboard::Clipboard::new() {
            Ok(new_ctx) => {
                log::info!("Recreated clipboard context");
                *ctx = Some(new_ctx);
            }
            Err(e) => {
                log::error!("Failed to recreate clipboard context: {}", e);
                return None;
            }
        }
    }
    
    let ctx = ctx.as_mut().unwrap();
    
    // 先尝试读取文本
    match ctx.get_text() {
        Ok(text) => {
            if !text.is_empty() {
                log::debug!("Read text from clipboard: {} chars", text.len());
                return Some(ClipboardContent::Text(text));
            }
        }
        Err(e) => {
            log::debug!("Failed to read text: {:?}", e);
        }
    }
    
    // 尝试读取图片
    match ctx.get_image() {
        Ok(image) => {
            let timestamp = Local::now().timestamp_millis();
            let filename = format!("{}.png", timestamp);
            
            if let Err(e) = storage::save_image(&image, &filename) {
                log::error!("Failed to save image: {}", e);
                return None;
            }
            
            log::debug!("Read image from clipboard: {}x{}", image.width, image.height);
            return Some(ClipboardContent::Image(filename));
        }
        Err(e) => {
            log::debug!("Failed to read image: {:?}", e);
        }
    }
    
    log::debug!("No clipboard content available");
    None
}

fn is_duplicate(new: &ClipboardContent, last: &Option<ClipboardContent>) -> bool {
    let Some(ref last_content) = last else {
        return false;
    };
    
    match (new, last_content) {
        (ClipboardContent::Text(a), ClipboardContent::Text(b)) => a == b,
        (ClipboardContent::Image(_), ClipboardContent::Image(_)) => true,
        _ => false,
    }
}

fn update_history(content: ClipboardContent) {
    let state = get_app_state();
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let id = format!("{}", Local::now().timestamp_millis());
    
    let item = HistoryItem {
        timestamp,
        content: content.clone(),
        id,
    };
    
    let mut history = state.history.lock().unwrap();
    history.insert(0, item);
    
    if history.len() > MAX_HISTORY_ITEMS {
        history.truncate(MAX_HISTORY_ITEMS);
    }
    
    // 同时更新当前内容
    let mut current = state.current_content.lock().unwrap();
    *current = Some(content);
}

pub fn get_current_clipboard() -> Option<ClipboardContent> {
    let mut ctx = arboard::Clipboard::new().ok();
    read_clipboard(&mut ctx)
}
