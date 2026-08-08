mod anthropic;
mod api;
mod config;
mod error;
mod repository;
mod service;
mod state;
mod tool;

use std::sync::Arc;
use dashmap::DashMap;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::repository::database;
use crate::state::AppState;
use crate::tool::skill_tool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志: 文件 + 控制台双输出
    let log_file = config::log_file();
    if let Some(parent) = log_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file_appender = tracing_appender::rolling::never(log_file.parent().unwrap(), log_file.file_name().unwrap());
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    // Windows 下配置 PowerShell UTF-8
    tool::shell_tool::setup_powershell_utf8().await;

    // 初始化技能目录
    skill_tool::init_skills().await;

    // 初始化数据库
    let db = database::init_db().await?;

    // 初始化 MCP 客户端
    tool::mcp_tool::init_mcp_clients(&db).await;

    // 组装应用状态
    let state = AppState {
        db,
        works: Arc::new(DashMap::new()),
    };

    // 组装路由
    let app = api::create_router(state);

    // 绑定地址
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    tracing::info!("Application started, listening on 127.0.0.1:8000");

    // 启动服务，支持 Ctrl-C graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// 等待 Ctrl-C 信号
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("Shutdown signal received");
}
