// 连接池, 建表, 版本迁移
use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::config;

// 列定义: column_type 为基本类型, suffix 为主键/外键等附加子句(仅建表使用)
struct ColumnSchema {
    name: &'static str,
    column_type: &'static str,
    not_null: bool,
    suffix: &'static str,
}

// 自增主键列
const ID_COL: ColumnSchema = ColumnSchema { name: "id", column_type: "INTEGER", not_null: false, suffix: "PRIMARY KEY AUTOINCREMENT" };

// 表定义: indexes 为依附于该表的索引语句, 重建表后需重新创建
struct TableSchema {
    name: &'static str,
    columns: &'static [ColumnSchema],
    indexes: &'static [&'static str],
}

// 全部表结构定义, 建表与迁移共用同一份数据源
const TABLES: &[TableSchema] = &[
    TableSchema { name: "t_model_provider", columns: &[ID_COL, ColumnSchema { name: "name", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "protocol_type", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "base_url", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "api_key", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "create_time", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "update_time", column_type: "TEXT", not_null: true, suffix: "" }], indexes: &[] },
    TableSchema { name: "t_agent", columns: &[ID_COL, ColumnSchema { name: "name", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "description", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "prompt", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "model_provider_id", column_type: "INTEGER", not_null: true, suffix: "REFERENCES t_model_provider(id) ON DELETE RESTRICT" }, ColumnSchema { name: "model", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "thinking", column_type: "INTEGER", not_null: true, suffix: "" }, ColumnSchema { name: "create_time", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "update_time", column_type: "TEXT", not_null: true, suffix: "" }], indexes: &[] },
    TableSchema { name: "t_task", columns: &[ID_COL, ColumnSchema { name: "title", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "content", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "agent_ids", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "work_dir", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "status", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "create_time", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "update_time", column_type: "TEXT", not_null: true, suffix: "" }], indexes: &[] },
    TableSchema { name: "t_schedule", columns: &[ID_COL, ColumnSchema { name: "name", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "content", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "work_dir", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "cron_expr", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "agent_id", column_type: "INTEGER", not_null: false, suffix: "REFERENCES t_agent(id) ON DELETE RESTRICT" }, ColumnSchema { name: "enabled", column_type: "INTEGER", not_null: true, suffix: "" }, ColumnSchema { name: "create_time", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "update_time", column_type: "TEXT", not_null: true, suffix: "" }], indexes: &[] },
    TableSchema { name: "t_conversation", columns: &[ID_COL, ColumnSchema { name: "task_id", column_type: "INTEGER", not_null: false, suffix: "REFERENCES t_task(id) ON DELETE CASCADE" }, ColumnSchema { name: "schedule_id", column_type: "INTEGER", not_null: false, suffix: "REFERENCES t_schedule(id) ON DELETE RESTRICT" }, ColumnSchema { name: "agent_id", column_type: "INTEGER", not_null: false, suffix: "REFERENCES t_agent(id) ON DELETE RESTRICT" }, ColumnSchema { name: "title", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "work_dir", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "system_prompt", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "create_time", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "update_time", column_type: "TEXT", not_null: true, suffix: "" }], indexes: &["CREATE INDEX IF NOT EXISTS idx_conversation_task ON t_conversation(task_id)"] },
    TableSchema { name: "t_message", columns: &[ID_COL, ColumnSchema { name: "conversation_id", column_type: "INTEGER", not_null: true, suffix: "REFERENCES t_conversation(id) ON DELETE CASCADE" }, ColumnSchema { name: "content", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "create_time", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "update_time", column_type: "TEXT", not_null: true, suffix: "" }], indexes: &["CREATE INDEX IF NOT EXISTS idx_message_conversation ON t_message(conversation_id)"] },
    TableSchema { name: "t_mcp_server", columns: &[ID_COL, ColumnSchema { name: "name", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "description", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "protocol_type", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "url", column_type: "TEXT", not_null: false, suffix: "" }, ColumnSchema { name: "headers", column_type: "TEXT", not_null: false, suffix: "" }, ColumnSchema { name: "command", column_type: "TEXT", not_null: false, suffix: "" }, ColumnSchema { name: "args", column_type: "TEXT", not_null: false, suffix: "" }, ColumnSchema { name: "create_time", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "update_time", column_type: "TEXT", not_null: true, suffix: "" }], indexes: &[] },
    TableSchema { name: "t_web_storage", columns: &[ColumnSchema { name: "key", column_type: "TEXT", not_null: false, suffix: "PRIMARY KEY" }, ColumnSchema { name: "value", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "create_time", column_type: "TEXT", not_null: true, suffix: "" }, ColumnSchema { name: "update_time", column_type: "TEXT", not_null: true, suffix: "" }], indexes: &[] },
];

// 创建数据库连接池并初始化
pub async fn init_db() -> anyhow::Result<SqlitePool> {
    // 确保数据目录存在
    let db_file = config::database_file();
    if let Some(parent) = db_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite:{}", db_file.display());
    let options = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true).pragma("journal_mode", "WAL").pragma("foreign_keys", "ON");

    // WAL 模式下读写可并发, 连接数放宽到 5, 写事务仍串行, 由 sqlx 默认 5s busy_timeout 兜底
    let pool = SqlitePoolOptions::new().max_connections(5).connect_with(options).await?;

    // 建表并迁移
    migrate_tables(&pool).await?;

    tracing::info!("Database initialized: path={}", db_file.display());
    Ok(pool)
}

// 创建并迁移所有表: 缺失列补充(非空列先带默认值, 再重建表去除默认值), 多余列删除
async fn migrate_tables(pool: &SqlitePool) -> anyhow::Result<()> {
    // 单连接执行, 迁移期间关闭外键约束以允许删表重建
    let mut conn = pool.acquire().await?;
    sqlx::query("PRAGMA foreign_keys=OFF").execute(&mut *conn).await?;

    for table in TABLES {
        // 建表, 新装库直接得到完整结构
        let fragments: Vec<String> = table
            .columns
            .iter()
            .map(|col| {
                let mut fragment = format!("{} {}", col.name, col.column_type);
                if col.not_null {
                    fragment.push_str(" NOT NULL");
                }
                if !col.suffix.is_empty() {
                    fragment.push(' ');
                    fragment.push_str(col.suffix);
                }
                fragment
            })
            .collect();
        let create_sql = format!("CREATE TABLE IF NOT EXISTS {} ({})", table.name, fragments.join(", "));
        sqlx::query(&create_sql).execute(&mut *conn).await?;

        // 读取现有列
        let column_rows = sqlx::query(&format!("PRAGMA table_info({})", table.name)).fetch_all(&mut *conn).await?;
        let existing_columns: Vec<String> = column_rows.iter().map(|row| row.get::<String, _>("name")).collect();

        // 补充缺失列, 非空列先带默认值创建(数值类型默认 0, 其余默认空串), 并标记该表需要重建以去除默认值
        let mut need_rebuild = false;
        for col in table.columns {
            if existing_columns.iter().any(|name| name == col.name) {
                continue;
            }
            let add_sql = if col.not_null {
                need_rebuild = true;
                let default_sql = match col.column_type {
                    "INTEGER" | "REAL" => "0",
                    _ => "''",
                };
                format!("ALTER TABLE {} ADD COLUMN {} {} NOT NULL DEFAULT {}", table.name, col.name, col.column_type, default_sql)
            } else {
                format!("ALTER TABLE {} ADD COLUMN {} {}", table.name, col.name, col.column_type)
            };
            sqlx::query(&add_sql).execute(&mut *conn).await?;
            tracing::info!("Column added: table={}, column={}", table.name, col.name);
        }

        // 删除多余列
        for name in &existing_columns {
            if table.columns.iter().any(|col| col.name == name) {
                continue;
            }
            let drop_sql = format!("ALTER TABLE {} DROP COLUMN {}", table.name, name);
            sqlx::query(&drop_sql).execute(&mut *conn).await?;
            tracing::info!("Column dropped: table={}, column={}", table.name, name);
        }

        // 重建表以去除补列时引入的默认值: 建新表 -> 按交集列拷数据 -> 删旧表 -> 改名
        if need_rebuild {
            let latest_rows = sqlx::query(&format!("PRAGMA table_info({})", table.name)).fetch_all(&mut *conn).await?;
            let latest_columns: Vec<String> = latest_rows.iter().map(|row| row.get::<String, _>("name")).collect();
            let copy_columns: Vec<&str> = table.columns.iter().filter(|col| latest_columns.iter().any(|name| name == col.name)).map(|col| col.name).collect();
            let temp_table = format!("{}__migrate", table.name);
            let rebuild_sqls = [format!("CREATE TABLE {} ({})", temp_table, fragments.join(", ")), format!("INSERT INTO {} ({}) SELECT {} FROM {}", temp_table, copy_columns.join(", "), copy_columns.join(", "), table.name), format!("DROP TABLE {}", table.name), format!("ALTER TABLE {} RENAME TO {}", temp_table, table.name)];
            sqlx::query("BEGIN").execute(&mut *conn).await?;
            let mut rebuild_error: Option<anyhow::Error> = None;
            for sql in &rebuild_sqls {
                if let Err(err) = sqlx::query(sql).execute(&mut *conn).await {
                    rebuild_error = Some(err.into());
                    break;
                }
            }
            match rebuild_error {
                Some(err) => {
                    sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                    return Err(err);
                }
                None => {
                    sqlx::query("COMMIT").execute(&mut *conn).await?;
                    tracing::info!("Table rebuilt: table={}", table.name);
                }
            }
        }

        // 重建依附索引, 表重建后旧索引会随旧表一并删除
        for index_sql in table.indexes {
            sqlx::query(index_sql).execute(&mut *conn).await?;
        }
    }

    sqlx::query("PRAGMA foreign_keys=ON").execute(&mut *conn).await?;
    Ok(())
}
