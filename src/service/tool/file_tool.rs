// 文件操作工具: read/write/edit
use std::path::PathBuf;

use tokio::fs;

use super::ToolResult;

// 执行文件操作
pub async fn execute(cmd_and_args: &[String], work_dir: &str) -> ToolResult {
    let args: Vec<&str> = cmd_and_args.iter().map(String::as_str).collect();
    match args.as_slice() {
        // file read <path>
        ["file", "read", file_path] => {
            let path = resolve_path(work_dir, file_path);
            if !path.exists() {
                return (format!("File not found: {}", file_path), true);
            }
            if !path.is_file() {
                return (format!("Path is not file: {}", file_path), true);
            }
            match fs::read_to_string(&path).await {
                Ok(content) => (content, false),
                Err(e) => (format!("Failed to read file: {}", e), true),
            }
        }
        // file write <path> <content>
        ["file", "write", file_path, content] => {
            let path = resolve_path(work_dir, file_path);
            // 确保父目录存在
            if let Some(parent) = path.parent() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    return (format!("Failed to create directory: {}", e), true);
                }
            }
            match fs::write(&path, content).await {
                Ok(_) => (String::new(), false),
                Err(e) => (format!("Failed to write file: {}", e), true),
            }
        }
        // file edit <path> <old_str> <new_str>
        ["file", "edit", file_path, old_str, new_str] => {
            let path = resolve_path(work_dir, file_path);
            if !path.exists() {
                return (format!("File not found: {}", file_path), true);
            }
            if !path.is_file() {
                return (format!("Path is not file: {}", file_path), true);
            }
            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(e) => return (format!("Failed to read file: {}", e), true),
            };
            if !content.contains(*old_str) {
                return (format!("Target string not found in file:\n{}", old_str), true);
            }
            let new_content = content.replace(old_str, new_str);
            match fs::write(&path, new_content).await {
                Ok(_) => (String::new(), false),
                Err(e) => (format!("Failed to write file: {}", e), true),
            }
        }
        _ => (format!("Unknown file command: {}", args.join(" ")), true),
    }
}

// 解析路径: 将相对路径基于 work_dir 解析
fn resolve_path(work_dir: &str, file_path: &str) -> PathBuf {
    let path = PathBuf::from(work_dir).join(file_path);
    path.canonicalize().unwrap_or(path)
}
