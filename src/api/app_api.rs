// 应用入口与目录浏览
use axum::extract::Query;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect};
use rust_embed::Embed;
use serde::Deserialize;
use serde_json::json;

use crate::config;
use crate::error::AppError;

// 嵌入静态资源
#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

// GET / 重定向到 static/index.html
pub async fn index() -> impl IntoResponse {
    Redirect::to("/static/index.html")
}

// GET /static/* 静态资源
pub async fn static_file(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches("/static/");
    match <StaticAssets as rust_embed::Embed>::get(path) {
        Some(content) => {
            let mime = guess_mime_type(path);
            ([(header::CONTENT_TYPE, mime)], content.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "File not found").into_response(),
    }
}

// 根据文件扩展名猜测 MIME 类型
fn guess_mime_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}

// 目录浏览查询参数
#[derive(Debug, Deserialize)]
pub struct DirListQuery {
    pub path: String,
}

// GET /dir/list 列出指定路径下的子目录
pub async fn list_directory(Query(query): Query<DirListQuery>) -> Result<impl IntoResponse, AppError> {
    let path = if query.path.is_empty() {
        std::path::PathBuf::from(config::home_dir())
    } else {
        std::path::PathBuf::from(&query.path)
    };

    if !path.exists() || !path.is_dir() {
        return Err(AppError::NotFound("Directory not found".to_string()));
    }

    let mut directories = Vec::new();
    let mut entries = tokio::fs::read_dir(&path).await.map_err(|e| AppError::Internal(e.into()))?;
    while let Some(entry) = entries.next_entry().await.map_err(|e| AppError::Internal(e.into()))? {
        let entry_path = entry.path();
        // 异步获取文件类型,避免阻塞运行时
        let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            let name = entry.file_name().to_string_lossy().to_string();
            let resolved = entry_path.canonicalize().unwrap_or(entry_path);
            directories.push(json!({
                "name": name,
                "path": resolved.to_string_lossy(),
            }));
        }
    }
    directories.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));

    let current_path = path.canonicalize().unwrap_or(path.clone());
    let parent_path = path.parent().map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()));

    Ok(axum::Json(json!({
        "current_path": current_path.to_string_lossy(),
        "parent_path": parent_path.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        "directories": directories,
    })))
}
