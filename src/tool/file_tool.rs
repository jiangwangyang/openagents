// 文件操作工具: read/write/edit
use std::path::PathBuf;
use tokio::fs;

use super::ToolResult;

// 执行文件操作
pub async fn execute(cmd_and_args: &[String], work_dir: &str) -> ToolResult {
    // 1. file read <file_path>
    if cmd_and_args.len() == 3 && cmd_and_args[0] == "file" && cmd_and_args[1] == "read" {
        let file_path = &cmd_and_args[2];
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
    // 2. file write <file_path> <content>
    else if cmd_and_args.len() == 4 && cmd_and_args[0] == "file" && cmd_and_args[1] == "write" {
        let file_path = &cmd_and_args[2];
        let content = &cmd_and_args[3];
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
    // 3. file edit <file_path> <old_str> <new_str>
    else if cmd_and_args.len() == 5 && cmd_and_args[0] == "file" && cmd_and_args[1] == "edit" {
        let file_path = &cmd_and_args[2];
        let old_str = &cmd_and_args[3];
        let new_str = &cmd_and_args[4];
        let path = resolve_path(work_dir, file_path);
        if !path.exists() {
            return (format!("File not found: {}", file_path), true);
        }
        if !path.is_file() {
            return (format!("Path is not file: {}", file_path), true);
        }
        // 读文件
        let content = match fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => return (format!("Failed to read file: {}", e), true),
        };
        // 应用替换逻辑
        if !content.contains(old_str.as_str()) {
            return (format!("Target string not found in file:\n{}", old_str), true);
        }
        let new_content = content.replace(old_str.as_str(), new_str);
        // 写文件
        match fs::write(&path, new_content).await {
            Ok(_) => (String::new(), false),
            Err(e) => (format!("Failed to write file: {}", e), true),
        }
    } else {
        ("Unknown command".to_string(), true)
    }
}

// 解析路径: 将相对路径基于 work_dir 解析
fn resolve_path(work_dir: &str, file_path: &str) -> PathBuf {
    let path = PathBuf::from(work_dir).join(file_path);
    // 尝试规范化路径
    path.canonicalize().unwrap_or(path)
}