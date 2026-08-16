// 技能列表工具
use std::path::PathBuf;

use super::ToolResult;
use crate::config;
use crate::state::SkillInfo;

// Skill 读取目录
fn skills_dir_list() -> Vec<PathBuf> {
    let home = config::home_dir();
    vec![
        PathBuf::from(&home).join(".openagents").join("skills"),
        PathBuf::from(&home).join(".agents").join("skills"),
    ]
}

// 初始化所有 skill 信息
pub async fn init_skills(skills_store: &std::sync::RwLock<Vec<SkillInfo>>) {
    let mut loaded = std::collections::HashSet::new();
    let mut skills = Vec::new();

    for skills_dir in skills_dir_list() {
        if !skills_dir.exists() {
            continue;
        }

        let mut entries = match tokio::fs::read_dir(&skills_dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let skill_dir = entry.path();
            let skill_name = match skill_dir.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            if loaded.contains(&skill_name) {
                continue;
            }

            let skill_file = skill_dir.join("SKILL.md");
            if !skill_file.is_file() {
                continue;
            }

            // 读取 SKILL.md 提取 name 和 description
            let text = match tokio::fs::read_to_string(&skill_file).await {
                Ok(t) => t,
                Err(_) => continue,
            };

            let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();
            if lines.is_empty() || lines[0] != "---" {
                continue;
            }

            // 找到第二个 ---
            let second_index = match lines[1..].iter().position(|l| *l == "---") {
                Some(i) => i + 1,
                None => continue,
            };

            let mut name = String::new();
            let mut description = String::new();
            for line in &lines[1..second_index] {
                if let Some(value) = line.strip_prefix("name:") {
                    name = value.trim().to_string();
                } else if let Some(value) = line.strip_prefix("description:") {
                    description = value.trim().to_string();
                }
            }

            if name == skill_name {
                // 提取内容(第二个 --- 之后的部分)
                let content_start = text.match_indices("---").nth(1);
                let content = match content_start {
                    Some((idx, _)) => text[idx + 3..].trim().to_string(),
                    None => String::new(),
                };

                let path = skill_file
                    .canonicalize()
                    .unwrap_or(skill_file)
                    .to_string_lossy()
                    .to_string();
                skills.push(SkillInfo {
                    name,
                    description,
                    path,
                    content,
                });
                loaded.insert(skill_name);
            }
        }
    }

    // 更新存储
    {
        // 锁被毒化时恢复内部数据继续写入, 避免 panic
        let mut guard = skills_store.write().unwrap_or_else(|e| e.into_inner());
        *guard = skills;
    }

    let count = skills_store.read().unwrap_or_else(|e| e.into_inner()).len();
    tracing::info!("Skills initialized, having {} skills", count);
}

// 获取技能列表
pub fn list_skills(skills_store: &std::sync::RwLock<Vec<SkillInfo>>) -> Vec<SkillInfo> {
    skills_store
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

// 执行技能命令
pub fn execute(
    cmd_and_args: &[String],
    skills_store: &std::sync::RwLock<Vec<SkillInfo>>,
) -> ToolResult {
    // 1. skill list
    if cmd_and_args.len() == 2 && cmd_and_args[0] == "skill" && cmd_and_args[1] == "list" {
        let skills = skills_store.read().unwrap_or_else(|e| e.into_inner());
        let result: Vec<serde_json::Value> = skills
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "description": s.description,
                    "path": s.path
                })
            })
            .collect();
        (serde_json::to_string(&result).unwrap_or_default(), false)
    } else {
        ("Unknown command".to_string(), true)
    }
}
