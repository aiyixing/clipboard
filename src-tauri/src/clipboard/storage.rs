use crate::ClipboardContent;
use chrono::Local;
use dirs::data_dir;
use regex::Regex;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

lazy_static::lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(
        "https?://[^\\s<>\"{}|\\\\^\\[\\]]+"
    ).unwrap();
}

fn get_app_data_dir() -> PathBuf {
    let mut path = data_dir().expect("Failed to get data directory");
    path.push("ClipboardMonitor");
    
    if !path.exists() {
        fs::create_dir_all(&path).expect("Failed to create app data directory");
    }
    
    path
}

fn get_images_dir() -> PathBuf {
    let mut path = get_app_data_dir();
    path.push("images");
    
    if !path.exists() {
        fs::create_dir_all(&path).expect("Failed to create images directory");
    }
    
    path
}

fn get_hooks_dir() -> PathBuf {
    let mut path = get_app_data_dir();
    path.push("hooks");
    
    if !path.exists() {
        fs::create_dir_all(&path).expect("Failed to create hooks directory");
    }
    
    path
}

fn get_history_file() -> PathBuf {
    let mut path = get_app_data_dir();
    path.push("clipboard_history.md");
    path
}

fn get_links_file() -> PathBuf {
    let mut path = get_app_data_dir();
    path.push("links.txt");
    path
}

pub fn save_content(content: &ClipboardContent) -> io::Result<()> {
    let history_file = get_history_file();
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
    
    Ok(())
}

pub fn save_image(image: &arboard::ImageData, filename: &str) -> io::Result<()> {
    let images_dir = get_images_dir();
    let image_path = images_dir.join(filename);
    
    let rgba_image = image::RgbaImage::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.to_vec(),
    ).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to create image from clipboard data"))?;
    
    let dynamic_image = image::DynamicImage::ImageRgba8(rgba_image);
    
    dynamic_image.save(&image_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to save image: {}", e)))?;
    
    Ok(())
}

pub fn extract_and_save_links(text: &str) -> io::Result<()> {
    let links: Vec<&str> = URL_REGEX.find_iter(text)
        .map(|mat| mat.as_str())
        .collect();
    
    if links.is_empty() {
        return Ok(());
    }
    
    let links_file = get_links_file();
    
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
    let path = get_app_data_dir();
    open::that(&path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open directory: {}", e)))
}

pub fn get_app_data_path() -> PathBuf {
    get_app_data_dir()
}

pub fn get_images_path() -> PathBuf {
    get_images_dir()
}

pub fn get_hooks_path() -> PathBuf {
    get_hooks_dir()
}
