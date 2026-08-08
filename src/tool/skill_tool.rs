// 技能列表工具
use serde::Serialize;
use std::path::PathBuf;

use super::ToolResult;

// Skill 数据结构
#[derive(Debug, Clone, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub path: String,
    pub content: String,
}

// 存储所有的 skill 信息
static SKILLS: std::sync::RwLock<Vec<SkillInfo>> = std::sync::RwLock::new(Vec::new());

// Skill 读取目录
fn skills_dir_list() -> Vec<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    vec![
        PathBuf::from(&home).join(".openagents").join("skills"),
        PathBuf::from(&home).join(".agents").join("skills"),
    ]
}

// 初始化所有 skill 信息
pub async fn init_skills() {
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
                if line.starts_with("name:") {
                    name = line[5..].trim().to_string();
                } else if line.starts_with("description:") {
                    description = line[12..].trim().to_string();
                }
            }

            if name == skill_name {
                // 提取内容(第二个 --- 之后的部分)
                let content_start = text.match_indices("---").nth(1);
                let content = match content_start {
                    Some((idx, _)) => text[idx + 3..].trim().to_string(),
                    None => String::new(),
                };

                let path = skill_file.canonicalize().unwrap_or(skill_file).to_string_lossy().to_string();
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

    // 更新全局存储
    {
        let mut guard = SKILLS.write().unwrap();
        *guard = skills;
    }

    let count = SKILLS.read().unwrap().len();
    tracing::info!("Skills initialized, having {} skills", count);
}

// 获取技能列表
pub fn list_skills() -> Vec<SkillInfo> {
    SKILLS.read().unwrap().clone()
}

// 执行
pub fn execute(cmd_and_args: &[String]) -> ToolResult {
    // 1. skill list
    if cmd_and_args.len() == 2 && cmd_and_args[0] == "skill" && cmd_and_args[1] == "list" {
        let skills = SKILLS.read().unwrap();
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