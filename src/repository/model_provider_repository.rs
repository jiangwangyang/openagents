// 模型提供商 CRUD
use sqlx::SqlitePool;

use super::entity::ModelProviderEntity;

// 查询全部模型提供商，按 id 升序
pub async fn list_model_providers(pool: &SqlitePool) -> Result<Vec<ModelProviderEntity>, sqlx::Error> {
    sqlx::query_as::<_, ModelProviderEntity>("SELECT id, name, protocol_type, base_url, api_key, create_time, update_time FROM t_model_provider ORDER BY id")
        .fetch_all(pool)
        .await
}

// 按 id 查询模型提供商
pub async fn get_model_provider(pool: &SqlitePool, provider_id: i64) -> Result<Option<ModelProviderEntity>, sqlx::Error> {
    sqlx::query_as::<_, ModelProviderEntity>("SELECT id, name, protocol_type, base_url, api_key, create_time, update_time FROM t_model_provider WHERE id = ?")
        .bind(provider_id)
        .fetch_optional(pool)
        .await
}

// 新增模型提供商，成功返回自增 id
pub async fn add_model_provider(pool: &SqlitePool, name: &str, protocol_type: &str, base_url: &str, api_key: &str) -> Result<i64, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO t_model_provider (name, protocol_type, base_url, api_key, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(name)
    .bind(protocol_type)
    .bind(base_url)
    .bind(api_key)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

// 按 id 更新模型提供商，id 不存在返回 false
pub async fn update_model_provider(pool: &SqlitePool, provider_id: i64, name: &str, protocol_type: &str, base_url: &str, api_key: &str) -> Result<bool, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE t_model_provider SET name = ?, protocol_type = ?, base_url = ?, api_key = ?, update_time = ? WHERE id = ?",
    )
    .bind(name)
    .bind(protocol_type)
    .bind(base_url)
    .bind(api_key)
    .bind(&now)
    .bind(provider_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// 按 id 删除模型提供商，不存在或被 Agent 引用返回 false
pub async fn delete_model_provider(pool: &SqlitePool, provider_id: i64) -> Result<bool, sqlx::Error> {
    let referenced: Option<(i64,)> = sqlx::query_as("SELECT id FROM t_agent WHERE model_provider_id = ? LIMIT 1")
        .bind(provider_id)
        .fetch_optional(pool)
        .await?;
    if referenced.is_some() {
        return Ok(false);
    }
    let result = sqlx::query("DELETE FROM t_model_provider WHERE id = ?")
        .bind(provider_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
