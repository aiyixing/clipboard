pub mod clipboard;
pub mod hooks;
pub mod tray;
pub mod commands;

#[derive(Debug, Clone, serde::Serialize)]
pub enum ClipboardContent {
    Text(String),
    Image(String),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HistoryItem {
    pub timestamp: String,
    pub content: ClipboardContent,
    pub id: String,
}

#[derive(Debug)]
pub struct AppState {
    pub is_monitoring: std::sync::atomic::AtomicBool,
    pub preview_cleared: std::sync::atomic::AtomicBool,
    pub history: std::sync::Mutex<Vec<HistoryItem>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            is_monitoring: std::sync::atomic::AtomicBool::new(true),
            preview_cleared: std::sync::atomic::AtomicBool::new(false),
            history: std::sync::Mutex::new(Vec::new()),
        }
    }
}

static APP_STATE: once_cell::sync::Lazy<AppState> = once_cell::sync::Lazy::new(AppState::default);

pub fn get_app_state() -> &'static AppState {
    &APP_STATE
}
