// 技能列表 API
use axum::extract::State;
use axum::Json;

use crate::service::tool::skill_tool;
use crate::state::{AppState, SkillInfo};

// 查询全部已加载的 Skill
pub async fn list_skills(State(state): State<AppState>) -> Json<Vec<SkillInfo>> {
    Json(skill_tool::list_skills(&state.skills))
}
