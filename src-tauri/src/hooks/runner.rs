use crate::{ClipboardContent, clipboard::storage};
use std::fs;
use std::io;
use std::path::PathBuf;

pub fn execute_hooks(content: &ClipboardContent) -> io::Result<()> {
    let hooks_dir = match storage::get_hooks_path() {
        Ok(dir) => dir,
        Err(e) => {
            log::error!("Failed to get hooks directory: {}", e);
            return Ok(());
        }
    };
    
    if !hooks_dir.exists() {
        return Ok(());
    }
    
    // 读取 hooks 目录下的所有 .ps1 脚本
    let entries = match fs::read_dir(&hooks_dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::error!("Failed to read hooks directory: {}", e);
            return Ok(());
        }
    };
    
    for entry in entries.flatten() {
        let path = entry.path();
        
        if path.extension().map(|ext| ext == "ps1").unwrap_or(false) {
            if let Err(e) = execute_single_hook(&path, content) {
                log::error!("Failed to execute hook {}: {}", path.display(), e);
            }
        }
    }
    
    Ok(())
}

fn execute_single_hook(script_path: &PathBuf, content: &ClipboardContent) -> io::Result<()> {
    log::info!("Executing hook: {}", script_path.display());
    
    // 准备环境变量
    let (content_str, content_type, image_path) = match content {
        ClipboardContent::Text(text) => (text.clone(), "text".to_string(), String::new()),
        ClipboardContent::Image(filename) => {
            let full_path = match storage::get_images_path() {
                Ok(images_dir) => images_dir.join(filename),
                Err(_) => PathBuf::from(filename),
            };
            (
                String::new(),
                "image".to_string(),
                full_path.to_string_lossy().to_string(),
            )
        }
    };
    
    // 使用 PowerShell 执行脚本
    let mut command = std::process::Command::new("powershell");
    command
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(script_path)
        .env("CLIPBOARD_CONTENT", content_str)
        .env("CLIPBOARD_TYPE", content_type)
        .env("CLIPBOARD_PATH", image_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    
    let output = command.spawn()?.wait_with_output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("Hook execution failed: {}", stderr);
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            log::info!("Hook output: {}", stdout);
        }
    }
    
    Ok(())
}
