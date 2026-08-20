// 路径, 环境变量, 常量定义
use std::path::PathBuf;

// 获取用户主目录, 环境变量缺失时 panic, 避免数据目录错误落到当前工作目录
pub fn home_dir() -> String {
    std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).expect("cannot determine home directory: neither HOME nor USERPROFILE is set")
}

// 数据目录: ~/.openagents
pub fn data_dir() -> PathBuf {
    PathBuf::from(home_dir()).join(".openagents")
}

// 数据库文件路径
pub fn database_file() -> PathBuf {
    data_dir().join("database.db")
}

// 日志目录: ~/.openagents/log
pub fn log_dir() -> PathBuf {
    data_dir().join("log")
}
