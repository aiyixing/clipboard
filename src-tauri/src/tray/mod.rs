use crate::get_app_state;
use tauri::{Emitter, Manager};
use tauri::tray::TrayIconBuilder;
use tauri::menu::{MenuBuilder, MenuItemBuilder};

pub fn init_tray(app_handle: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItemBuilder::new("显示主窗口")
        .id("show")
        .build(app_handle)?;
    
    let toggle_item = MenuItemBuilder::new("暂停监听")
        .id("toggle")
        .build(app_handle)?;
    
    let quit_item = MenuItemBuilder::new("退出")
        .id("quit")
        .build(app_handle)?;
    
    let menu = MenuBuilder::new(app_handle)
        .item(&show_item)
        .item(&toggle_item)
        .separator()
        .item(&quit_item)
        .build()?;
    
    let _tray = TrayIconBuilder::with_id("main")
        .icon(app_handle.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "show" => show_main_window(app),
                "toggle" => toggle_monitoring(app),
                "quit" => quit_application(app),
                _ => {}
            }
        })
        .build(app_handle)?;
    
    Ok(())
}

fn show_main_window(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        window.show().ok();
        window.set_focus().ok();
    }
}

fn toggle_monitoring(app_handle: &tauri::AppHandle) {
    let state = get_app_state();
    let current = state.is_monitoring.load(std::sync::atomic::Ordering::Relaxed);
    state.is_monitoring.store(!current, std::sync::atomic::Ordering::Relaxed);
    
    let _ = app_handle.emit("monitoring-status-changed", !current);
}

fn quit_application(app_handle: &tauri::AppHandle) {
    app_handle.exit(0);
}
