// 技能列表 API
use axum::Json;

use crate::tool::skill_tool;
use crate::tool::skill_tool::SkillInfo;

// 查询全部已加载的 Skill
pub async fn list_skills() -> Json<Vec<SkillInfo>> {
    Json(skill_tool::list_skills())
}
