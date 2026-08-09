#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod anthropic;
mod api;
mod config;
mod error;
mod repository;
mod service;
mod state;
mod tool;

use dashmap::DashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::repository::database;
use crate::state::AppState;
use crate::tool::skill_tool;

fn main() -> anyhow::Result<()> {
    // 解析启动模式: 默认桌面模式，--web 为纯 HTTP 服务模式
    let arg: Option<String> = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--web") => {
            // Web 模式: 仅启动 HTTP 服务，浏览器访问，Ctrl-C 停止，固定 8000 端口
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(run_server(None, "127.0.0.1:8000".to_string()))
        }
        Some("--desktop") | None => run_desktop(),
        _ => {
            eprintln!("Usage: openagents [--desktop | --web]");
            std::process::exit(2);
        }
    }
}

// 桌面模式: 后台线程运行 HTTP 服务并绑定随机端口，主线程由 Tauri 创建窗口，关窗即退出进程
fn run_desktop() -> anyhow::Result<()> {
    // 通道用于后台服务线程回传实际监听端口
    let (port_tx, port_rx) = std::sync::mpsc::channel::<u16>();

    // 后台线程运行 HTTP 服务，桌面模式使用随机端口避免 8000 冲突
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        if let Err(err) = runtime.block_on(run_server(Some(port_tx), "127.0.0.1:0".to_string())) {
            eprintln!("HTTP server exited with error: {err:#}");
        }
    });

    // 等待服务就绪并获取实际端口，超时说明后台服务在发送端口前挂起(如初始化阻塞)，避免主线程永久等待
    let port: u16 = port_rx.recv_timeout(std::time::Duration::from_secs(30))?;
    let url: String = format!("http://127.0.0.1:{port}");

    // 主线程创建 Tauri 窗口 (GUI 事件循环必须在主线程，macOS 强制要求)
    run_tauri(&url)
}

// 创建 Tauri 窗口并加载本地 HTTP 页面，窗口关闭时 Tauri 事件循环退出，后台服务随进程一并停止
fn run_tauri(url: &str) -> anyhow::Result<()> {
    let url = tauri::Url::parse(url)?;
    tauri::Builder::default()
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::External(url.clone()))
                .title("OpenAgents")
                .inner_size(1440.0, 900.0)
                .maximized(true)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())?;
    Ok(())
}

// 启动 HTTP 服务，Web 模式固定 8000 端口，桌面模式由调用方传入随机端口地址
async fn run_server(port_tx: Option<Sender<u16>>, bind_addr: String) -> anyhow::Result<()> {
    // 初始化日志: 文件 + 控制台双输出
    let log_file = config::log_file();
    if let Some(parent) = log_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file_appender = tracing_appender::rolling::daily(log_file.parent().unwrap(), log_file.file_name().unwrap());
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

    // 初始化定时任务调度器
    let conversations = Arc::new(DashMap::new());
    service::schedule_service::init_scheduler(&db, &conversations).await?;

    // 组装应用状态
    let state = AppState {
        db,
        conversations,
    };

    // 组装路由
    let app = api::create_router(state);

    // 绑定地址: Web 模式固定 8000，桌面模式传入 127.0.0.1:0 使用随机可用端口
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    let port: u16 = listener.local_addr()?.port();
    tracing::info!("Application started, listening on 127.0.0.1:{port}");

    // 桌面模式回传端口给主线程用于加载页面
    if let Some(tx) = port_tx {
        let _ = tx.send(port);
    }

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
