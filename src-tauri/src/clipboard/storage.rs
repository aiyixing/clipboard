use crate::ClipboardContent;
use chrono::Local;
use regex::Regex;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

lazy_static::lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(
        "https?://[^\\s<>\"{}|\\\\^\\[\\]]+"
    ).unwrap();
}

fn get_app_data_dir() -> io::Result<PathBuf> {
    // 尝试多个路径，直到找到一个可用的
    let possible_paths = vec![
        // 1. 尝试文档目录
        dirs::document_dir().map(|p| p.join("ClipboardMonitor")),
        // 2. 尝试用户主目录
        dirs::home_dir().map(|p| p.join("ClipboardMonitor")),
        // 3. 尝试当前工作目录
        std::env::current_dir().map(|p| p.join("data").join("ClipboardMonitor")).ok(),
    ];
    
    for path in possible_paths.into_iter().flatten() {
        log::info!("Trying app data directory: {:?}", path);
        
        if !path.exists() {
            log::info!("Creating directory: {:?}", path);
            match fs::create_dir_all(&path) {
                Ok(_) => {
                    log::info!("Directory created successfully: {:?}", path);
                    return Ok(path);
                }
                Err(e) => {
                    log::error!("Failed to create directory {:?}: {}", path, e);
                    continue;
                }
            }
        } else {
            // 检查是否有写入权限
            let test_file = path.join(".write_test");
            match fs::write(&test_file, "test") {
                Ok(_) => {
                    let _ = fs::remove_file(&test_file);
                    log::info!("Directory already exists and is writable: {:?}", path);
                    return Ok(path);
                }
                Err(e) => {
                    log::error!("Directory exists but is not writable {:?}: {}", path, e);
                    continue;
                }
            }
        }
    }
    
    // 如果所有路径都失败了，返回一个错误
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Failed to find a writable directory for app data"
    ))
}

pub fn get_images_dir() -> io::Result<PathBuf> {
    let path = get_app_data_dir()?.join("images");
    
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    
    Ok(path)
}

fn get_hooks_dir() -> io::Result<PathBuf> {
    let path = get_app_data_dir()?.join("hooks");
    
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    
    Ok(path)
}

fn get_history_file() -> io::Result<PathBuf> {
    Ok(get_app_data_dir()?.join("clipboard_history.md"))
}

fn get_links_file() -> io::Result<PathBuf> {
    Ok(get_app_data_dir()?.join("links.txt"))
}

pub fn save_content(content: &ClipboardContent) -> io::Result<()> {
    log::info!("save_content called with: {:?}", content);
    
    let history_file = match get_history_file() {
        Ok(f) => f,
        Err(e) => {
            log::error!("Failed to get history file: {}", e);
            return Err(e);
        }
    };
    
    log::info!("Writing to: {:?}", history_file);
    
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_file)?;
    
    match content {
        ClipboardContent::Text(text) => {
            writeln!(file, "")?;
            writeln!(file, "## {}", timestamp)?;
            writeln!(file, "{}", text)?;
            writeln!(file, "---")?;
        }
        ClipboardContent::Image(filename) => {
            writeln!(file, "")?;
            writeln!(file, "## {}", timestamp)?;
            writeln!(file, "![Image](images/{})", filename)?;
            writeln!(file, "---")?;
        }
    }
    
    log::info!("Content saved successfully");
    Ok(())
}

pub fn save_image(image: &arboard::ImageData, filename: &str) -> io::Result<()> {
    log::info!("save_image called");
    
    let images_dir = get_images_dir()?;
    let image_path = images_dir.join(filename);
    
    log::info!("Saving image to: {:?}", image_path);
    
    let rgba_image = image::RgbaImage::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.to_vec(),
    ).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to create image from clipboard data"))?;
    
    let dynamic_image = image::DynamicImage::ImageRgba8(rgba_image);
    
    dynamic_image.save(&image_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to save image: {}", e)))?;
    
    log::info!("Image saved successfully");
    Ok(())
}

pub fn extract_and_save_links(text: &str) -> io::Result<()> {
    let links: Vec<&str> = URL_REGEX.find_iter(text)
        .map(|mat| mat.as_str())
        .collect();
    
    if links.is_empty() {
        return Ok(());
    }
    
    log::info!("Found {} links", links.len());
    
    let links_file = get_links_file()?;
    
    let existing_links: std::collections::HashSet<String> = if links_file.exists() {
        let content = fs::read_to_string(&links_file)?;
        content.lines().map(|s| s.to_string()).collect()
    } else {
        std::collections::HashSet::new()
    };
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&links_file)?;
    
    for link in links {
        if !existing_links.contains(link) {
            writeln!(file, "{}", link)?;
        }
    }
    
    Ok(())
}

pub fn open_data_directory() -> io::Result<()> {
    let path = get_app_data_dir()?;
    log::info!("Opening directory: {:?}", path);
    
    open::that(&path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open directory: {}", e)))
}

pub fn get_app_data_path() -> io::Result<PathBuf> {
    get_app_data_dir()
}

pub fn get_images_path() -> io::Result<PathBuf> {
    get_images_dir()
}

pub fn get_hooks_path() -> io::Result<PathBuf> {
    get_hooks_dir()
}
