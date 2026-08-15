// HTTP 层: 路由组装
use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::state::AppState;

pub mod agent_api;
pub mod app_api;
pub mod conversation_api;
pub mod mcp_server_api;
pub mod model_provider_api;
pub mod schedule_api;
pub mod skill_api;
pub mod task_api;
pub mod web_storage_api;

// 组装全部路由
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // 应用入口
        .route("/", get(app_api::index))
        .route("/static/{*path}", get(app_api::static_file))
        .route("/dir/list", get(app_api::list_directory))
        // Agent CRUD
        .route("/agent/list", get(agent_api::list_agents))
        .route("/agent/{agent_id}", get(agent_api::get_agent))
        .route("/agent", post(agent_api::add_agent))
        .route("/agent/{agent_id}", put(agent_api::update_agent))
        .route("/agent/{agent_id}", delete(agent_api::delete_agent))
        // 对话
        .route("/conversation/list", get(conversation_api::list_conversations))
        .route("/conversation/{conversation_id}", get(conversation_api::get_conversation))
        .route("/conversation/{conversation_id}", delete(conversation_api::delete_conversation))
        .route("/conversation/{conversation_id}/message", post(conversation_api::add_conversation_message))
        .route("/conversation/start", post(conversation_api::create_conversation_work))
        .route("/conversation/{conversation_id}/start", post(conversation_api::start_conversation_work))
        .route("/conversation/{conversation_id}/stream", get(conversation_api::stream_conversation_work))
        // 模型提供商 CRUD
        .route("/model-provider/list", get(model_provider_api::list_model_providers))
        .route("/model-provider/{provider_id}", get(model_provider_api::get_model_provider))
        .route("/model-provider/{provider_id}/model/list", get(model_provider_api::list_provider_models))
        .route("/model-provider", post(model_provider_api::add_model_provider))
        .route("/model-provider/{provider_id}", put(model_provider_api::update_model_provider))
        .route("/model-provider/{provider_id}", delete(model_provider_api::delete_model_provider))
        // MCP 服务 CRUD
        .route("/mcp-server/list", get(mcp_server_api::list_mcp_servers))
        .route("/mcp-server/{server_id}", get(mcp_server_api::get_mcp_server))
        .route("/mcp-server/streamable-http", post(mcp_server_api::add_mcp_streamable_http_server))
        .route("/mcp-server/stdio", post(mcp_server_api::add_mcp_stdio_server))
        .route("/mcp-server/{server_id}/streamable-http", put(mcp_server_api::update_mcp_streamable_http_server))
        .route("/mcp-server/{server_id}/stdio", put(mcp_server_api::update_mcp_stdio_server))
        .route("/mcp-server/{server_id}", delete(mcp_server_api::delete_mcp_server))
        .route("/mcp-server/{server_id}/tool/list", post(mcp_server_api::list_mcp_server_tools))
        // 技能列表
        .route("/skill/list", get(skill_api::list_skills))
        // 定时任务 CRUD
        .route("/schedule/list", get(schedule_api::list_schedules))
        .route("/schedule/{schedule_id}", get(schedule_api::get_schedule))
        .route("/schedule", post(schedule_api::add_schedule))
        .route("/schedule/{schedule_id}", put(schedule_api::update_schedule))
        .route("/schedule/{schedule_id}", delete(schedule_api::delete_schedule))
        // 任务 CRUD
        .route("/task/list", get(task_api::list_tasks))
        .route("/task/{task_id}", get(task_api::get_task))
        .route("/task", post(task_api::add_task))
        .route("/task/{task_id}", delete(task_api::delete_task))
        .route("/task/{task_id}/start", post(task_api::start_task))
        // Web 存储
        .route("/web-storage/{key}", get(web_storage_api::get_web_storage))
        .route("/web-storage/{key}", put(web_storage_api::put_web_storage))
        .with_state(state)
}
