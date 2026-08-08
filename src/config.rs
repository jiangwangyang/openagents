// 路径、环境变量、常量定义
use std::path::PathBuf;

// 数据目录: ~/.openagents
pub fn data_dir() -> PathBuf {
    PathBuf::from(dirs_home()).join(".openagents")
}

// 数据库文件路径
pub fn database_file() -> PathBuf {
    data_dir().join("database.db")
}

// 日志文件路径
pub fn log_file() -> PathBuf {
    data_dir().join("app.log")
}

// 获取用户主目录
fn dirs_home() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string())
}
