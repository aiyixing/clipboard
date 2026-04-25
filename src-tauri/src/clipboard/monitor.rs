use crate::{ClipboardContent, HistoryItem, get_app_state, hooks};
use crate::clipboard::storage;
use chrono::Local;
use clipboard_rs::{Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

const MAX_HISTORY_ITEMS: usize = 100;

enum RawClipboardContent {
    Text(String),
    Image {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        hash: u64,
    },
}

impl RawClipboardContent {
    fn from_text(text: String) -> Self {
        RawClipboardContent::Text(text)
    }
    
    fn from_image(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        let hash = calculate_hash(&rgba);
        RawClipboardContent::Image {
            width,
            height,
            rgba,
            hash,
        }
    }
    
    fn get_hash(&self) -> u64 {
        match self {
            RawClipboardContent::Text(text) => calculate_hash(text),
            RawClipboardContent::Image { hash, .. } => *hash,
        }
    }
}

fn calculate_hash<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

struct LastContentInfo {
    hash: u64,
    content: Option<ClipboardContent>,
}

impl LastContentInfo {
    fn new() -> Self {
        LastContentInfo {
            hash: 0,
            content: None,
        }
    }
    
    fn from_raw(raw: &RawClipboardContent, content: ClipboardContent) -> Self {
        LastContentInfo {
            hash: raw.get_hash(),
            content: Some(content),
        }
    }
    
    fn is_duplicate(&self, raw: &RawClipboardContent) -> bool {
        if self.content.is_none() {
            return false;
        }
        self.hash == raw.get_hash()
    }
}

struct ClipboardMonitorHandler {
    app_handle: tauri::AppHandle,
    last_content: Arc<Mutex<LastContentInfo>>,
}

impl ClipboardHandler for ClipboardMonitorHandler {
    fn on_clipboard_change(&mut self) {
        let state = get_app_state();
        
        if !state.is_monitoring.load(std::sync::atomic::Ordering::Relaxed) {
            log::debug!("Monitoring is paused, skipping clipboard change");
            return;
        }
        
        // 读取剪贴板内容（只读取，不保存）
        match read_raw_clipboard() {
            Some(raw_content) => {
                log::info!("========== CLIPBOARD CHANGE DETECTED (clipboard-rs) ==========");
                
                // 检查是否重复（比较哈希值）
                let mut last = self.last_content.lock().unwrap();
                
                if last.is_duplicate(&raw_content) {
                    log::debug!("Content is duplicate (hash: {}), skipping", raw_content.get_hash());
                    log::info!("==================================================");
                    return;
                }
                
                log::info!("New content detected (hash: {})", raw_content.get_hash());
                
                // 转换为 ClipboardContent 并保存（如果是图片）
                let content = match convert_and_save(&raw_content) {
                    Some(c) => c,
                    None => {
                        log::error!("Failed to convert and save content");
                        log::info!("==================================================");
                        return;
                    }
                };
                
                log::info!("Content: {:?}", content);
                
                // 存储内容到 MD 文件
                if let Err(e) = storage::save_content(&content) {
                    log::error!("Failed to save content: {}", e);
                }
                
                // 提取链接
                if let ClipboardContent::Text(ref text) = content {
                    if let Err(e) = storage::extract_and_save_links(text) {
                        log::error!("Failed to extract links: {}", e);
                    }
                }
                
                // 触发钩子
                if let Err(e) = hooks::execute_hooks(&content) {
                    log::error!("Failed to execute hooks: {}", e);
                }
                
                // 更新历史记录
                update_history(content.clone());
                
                // 通知前端
                log::info!("Emitting clipboard-changed event to frontend...");
                match self.app_handle.emit("clipboard-changed", content.clone()) {
                    Ok(_) => log::info!("Event emitted successfully!"),
                    Err(e) => log::error!("Failed to emit event: {}", e),
                }
                
                // 更新 last_content
                *last = LastContentInfo::from_raw(&raw_content, content);
                log::info!("last_content updated");
                log::info!("==================================================");
            }
            None => {
                log::debug!("Clipboard change detected but no content available");
            }
        }
    }
}

fn convert_and_save(raw: &RawClipboardContent) -> Option<ClipboardContent> {
    match raw {
        RawClipboardContent::Text(text) => {
            Some(ClipboardContent::Text(text.clone()))
        }
        RawClipboardContent::Image { width, height, rgba, .. } => {
            // 保存图片
            let timestamp = Local::now().timestamp_millis();
            let filename = format!("{}.png", timestamp);
            
            // 获取图片目录
            let images_dir = match storage::get_images_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    log::error!("Failed to get images directory: {}", e);
                    return None;
                }
            };
            
            let image_path = images_dir.join(&filename);
            
            // 保存图片
            let rgba_image = match image::RgbaImage::from_raw(
                *width,
                *height,
                rgba.clone(),
            ) {
                Some(img) => img,
                None => {
                    log::error!("Failed to create image from raw data");
                    return None;
                }
            };
            
            let dynamic_image = image::DynamicImage::ImageRgba8(rgba_image);
            
            match dynamic_image.save(&image_path) {
                Ok(_) => {
                    log::info!("Image saved to: {:?}", image_path);
                    Some(ClipboardContent::Image(filename))
                }
                Err(e) => {
                    log::error!("Failed to save image: {}", e);
                    None
                }
            }
        }
    }
}

pub async fn start_monitor(app_handle: tauri::AppHandle) {
    log::info!("Clipboard monitor starting with clipboard-rs...");
    
    // 初始化时读取一次剪贴板内容（只读取，不保存）
    if let Some(initial_raw) = read_raw_clipboard() {
        // 转换并保存内容
        if let Some(initial_content) = convert_and_save(&initial_raw) {
            log::info!("Initial clipboard content: {:?}", initial_content);
            
            let state = get_app_state();
            if state.is_monitoring.load(std::sync::atomic::Ordering::Relaxed) {
                // 存储内容到 MD 文件
                if let Err(e) = storage::save_content(&initial_content) {
                    log::error!("Failed to save initial content: {}", e);
                }
                
                // 更新历史记录
                update_history(initial_content.clone());
                
                // 通知前端
                log::info!("Emitting initial clipboard-changed event...");
                match app_handle.emit("clipboard-changed", initial_content.clone()) {
                    Ok(_) => log::info!("Initial event emitted successfully!"),
                    Err(e) => log::error!("Failed to emit initial event: {}", e),
                }
            }
        }
    }
    
    // 创建 handler
    let handler = ClipboardMonitorHandler {
        app_handle: app_handle.clone(),
        last_content: Arc::new(Mutex::new(LastContentInfo::new())),
    };
    
    // 创建 watcher
    let mut watcher = match ClipboardWatcherContext::new() {
        Ok(w) => w,
        Err(e) => {
            log::error!("Failed to create clipboard watcher: {}", e);
            log::info!("Falling back to polling mode...");
            start_monitor_polling(app_handle).await;
            return;
        }
    };
    
    // 添加 handler 并启动监听
    let shutdown = watcher.add_handler(handler).get_shutdown_channel();
    
    log::info!("Clipboard watcher started successfully!");
    
    // 在另一个线程中运行 watcher
    std::thread::spawn(move || {
        watcher.start_watch();
    });
    
    // 启动自动测试线程（定期复制不同内容到剪贴板）
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        let mut test_count: u64 = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            
            test_count += 1;
            let test_text = format!("自动测试文本 {} - {}", test_count, Local::now().format("%H:%M:%S"));
            
            log::info!("[自动测试] 复制到剪贴板: {}", test_text);
            
            // 使用 clipboard-win 复制文本到剪贴板
            match clipboard_win::set_clipboard(clipboard_win::formats::Unicode, test_text.as_str()) {
                Ok(_) => {
                    log::info!("[自动测试] 成功复制文本到剪贴板");
                }
                Err(e) => {
                    log::error!("[自动测试] 复制失败: {}", e);
                }
            }
            
            // 最多测试 5 次，避免无限循环
            if test_count >= 5 {
                log::info!("[自动测试] 自动测试完成，共测试 {} 次", test_count);
                break;
            }
        }
    });
    
    // 保持运行
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn start_monitor_polling(app_handle: tauri::AppHandle) {
    let mut last_info = LastContentInfo::new();
    let mut loop_count: u64 = 0;
    const MONITOR_INTERVAL: u64 = 500;
    
    log::info!("Starting polling monitor as fallback...");
    
    loop {
        loop_count += 1;
        
        let state = get_app_state();
        
        if !state.is_monitoring.load(std::sync::atomic::Ordering::Relaxed) {
            if loop_count % 10 == 0 {
                log::debug!("Monitoring is paused, sleeping...");
            }
            tokio::time::sleep(Duration::from_millis(MONITOR_INTERVAL)).await;
            continue;
        }
        
        // 读取原始内容（只读取，不保存）
        let current_raw = read_raw_clipboard();
        
        if loop_count % 10 == 0 {
            log::debug!("Poll Loop {}: current_raw is_some={:?}", 
                loop_count, current_raw.is_some());
        }
        
        if let Some(ref raw) = current_raw {
            // 检查是否重复（比较哈希值）
            if last_info.is_duplicate(raw) {
                if loop_count % 10 == 0 {
                    log::debug!("Content is duplicate (hash: {}), skipping", raw.get_hash());
                }
                tokio::time::sleep(Duration::from_millis(MONITOR_INTERVAL)).await;
                continue;
            }
            
            log::info!("========== NEW CLIPBOARD CONTENT DETECTED (Polling) ==========");
            log::info!("New hash: {}", raw.get_hash());
            
            // 转换并保存（如果是图片）
            let content = match convert_and_save(raw) {
                Some(c) => c,
                None => {
                    log::error!("Failed to convert and save content");
                    tokio::time::sleep(Duration::from_millis(MONITOR_INTERVAL)).await;
                    continue;
                }
            };
            
            log::info!("Content: {:?}", content);
            
            // 存储内容到 MD 文件
            if let Err(e) = storage::save_content(&content) {
                log::error!("Failed to save content: {}", e);
            }
            
            // 提取链接
            if let ClipboardContent::Text(ref text) = content {
                if let Err(e) = storage::extract_and_save_links(text) {
                    log::error!("Failed to extract links: {}", e);
                }
            }
            
            // 触发钩子
            if let Err(e) = hooks::execute_hooks(&content) {
                log::error!("Failed to execute hooks: {}", e);
            }
            
            // 更新历史记录
            update_history(content.clone());
            
            // 通知前端
            log::info!("Emitting clipboard-changed event to frontend...");
            match app_handle.emit("clipboard-changed", content.clone()) {
                Ok(_) => log::info!("Event emitted successfully!"),
                Err(e) => log::error!("Failed to emit event: {}", e),
            }
            
            // 更新 last_info
            last_info = LastContentInfo::from_raw(raw, content);
            log::info!("last_info updated");
            log::info!("==================================================");
        }
        
        tokio::time::sleep(Duration::from_millis(MONITOR_INTERVAL)).await;
    }
}

fn read_raw_clipboard() -> Option<RawClipboardContent> {
    // 首先尝试使用 clipboard-rs 读取文本（Windows 事件驱动）
    match ClipboardContext::new() {
        Ok(ctx) => {
            // 尝试读取文本
            match ctx.get_text() {
                Ok(text) => {
                    if !text.is_empty() {
                        let preview: String = text.chars().take(50).collect();
                        log::debug!("clipboard-rs read text: {} chars, preview: {:?}", text.len(), preview);
                        return Some(RawClipboardContent::from_text(text));
                    }
                }
                Err(e) => {
                    log::debug!("clipboard-rs failed to read text: {:?}", e);
                }
            }
            
            // 对于图片，使用 arboard（因为它提供直接的 bytes 访问）
            log::debug!("Trying arboard for image...");
        }
        Err(e) => {
            log::error!("Failed to create clipboard-rs context: {}", e);
        }
    }
    
    // 使用 arboard 读取图片（或文本作为备用）
    match arboard::Clipboard::new() {
        Ok(mut ctx) => {
            // 先尝试读取文本（备用）
            match ctx.get_text() {
                Ok(text) => {
                    if !text.is_empty() {
                        let preview: String = text.chars().take(50).collect();
                        log::debug!("arboard read text: {} chars, preview: {:?}", text.len(), preview);
                        return Some(RawClipboardContent::from_text(text));
                    }
                }
                Err(e) => {
                    log::debug!("arboard failed to read text: {:?}", e);
                }
            }
            
            // 尝试读取图片
            match ctx.get_image() {
                Ok(img) => {
                    let width = img.width as u32;
                    let height = img.height as u32;
                    let rgba = img.bytes.to_vec();
                    
                    log::debug!("arboard read image: {}x{}, {} bytes", width, height, rgba.len());
                    
                    return Some(RawClipboardContent::from_image(width, height, rgba));
                }
                Err(e) => {
                    log::debug!("arboard failed to read image: {:?}", e);
                }
            }
        }
        Err(e) => {
            log::error!("Failed to create arboard context: {}", e);
        }
    }
    
    log::debug!("No clipboard content available");
    None
}

fn read_clipboard_for_display() -> Option<ClipboardContent> {
    match read_raw_clipboard() {
        Some(raw) => {
            match raw {
                RawClipboardContent::Text(text) => Some(ClipboardContent::Text(text)),
                RawClipboardContent::Image { width, height, .. } => {
                    // 对于显示，只需要显示 "[图片]" 而不需要保存
                    let display = format!("[图片 {}x{}]", width, height);
                    Some(ClipboardContent::Image(display))
                }
            }
        }
        None => None,
    }
}

fn read_clipboard() -> Option<ClipboardContent> {
    // 首先尝试使用 clipboard-win 读取文本（Windows 专用，更可靠）
    match clipboard_win::get_clipboard::<String, _>(clipboard_win::formats::Unicode) {
        Ok(text) => {
            if !text.is_empty() {
                let preview: String = text.chars().take(50).collect();
                log::debug!("clipboard-win read text: {} chars, preview: {:?}", text.len(), preview);
                return Some(ClipboardContent::Text(text));
            }
        }
        Err(e) => {
            log::debug!("clipboard-win failed to read text: {:?}", e);
        }
    }
    
    // 然后尝试使用 arboard
    match arboard::Clipboard::new() {
        Ok(mut ctx) => {
            // 先尝试读取文本
            match ctx.get_text() {
                Ok(text) => {
                    if !text.is_empty() {
                        let preview: String = text.chars().take(50).collect();
                        log::debug!("arboard read text: {} chars, preview: {:?}", text.len(), preview);
                        return Some(ClipboardContent::Text(text));
                    }
                }
                Err(e) => {
                    log::debug!("arboard failed to read text: {:?}", e);
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
                    
                    log::debug!("arboard read image: {}x{}", image.width, image.height);
                    return Some(ClipboardContent::Image(filename));
                }
                Err(e) => {
                    log::debug!("arboard failed to read image: {:?}", e);
                }
            }
        }
        Err(e) => {
            log::error!("Failed to create arboard context: {}", e);
        }
    }
    
    log::debug!("No clipboard content available");
    None
}

fn update_history(content: ClipboardContent) {
    log::info!("update_history called with: {:?}", content);
    
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
    
    log::info!("History updated, now has {} items", history.len());
    
    // 同时更新当前内容
    let mut current = state.current_content.lock().unwrap();
    *current = Some(content);
    
    log::info!("current_content updated");
}

pub fn get_current_clipboard() -> Option<ClipboardContent> {
    read_clipboard_for_display()
}
