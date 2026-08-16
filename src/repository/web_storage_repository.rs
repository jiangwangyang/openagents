// Web 存储读写
use sqlx::SqlitePool;

use super::entity::WebStorageEntity;

// 按 key 查询 Web 存储
pub async fn get_web_storage(
    pool: &SqlitePool,
    key: &str,
) -> Result<Option<WebStorageEntity>, sqlx::Error> {
    sqlx::query_as::<_, WebStorageEntity>(
        "SELECT key, value, create_time, update_time FROM t_web_storage WHERE key = ?",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
}

// 按 key 写入 Web 存储, 不存在则新增, 存在则更新 value 与 update_time
pub async fn put_web_storage(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO t_web_storage (key, value, create_time, update_time) VALUES (?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, update_time = excluded.update_time",
    )
    .bind(key)
    .bind(value)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}
