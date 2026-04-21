#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use clipboard_monitor::{clipboard, hooks, tray, commands};
use tauri::Manager;

fn main() {
    env_logger::init();
    log::info!("Clipboard Monitor starting...");

    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            
            // 初始化系统托盘
            tray::init_tray(&app_handle)?;
            
            // 启动剪贴板监听
            let app_handle_clone = app_handle.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async {
                    clipboard::start_monitor(app_handle_clone).await;
                });
            });
            
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_clipboard_content,
            commands::get_history,
            commands::clear_preview,
            commands::toggle_monitoring,
            commands::get_monitoring_status,
            commands::open_data_directory,
            commands::copy_to_clipboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
