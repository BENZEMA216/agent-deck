use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentHealthLogRecord {
    pub id: i64,
    pub agent_id: String,
    pub status: String,
    pub latency_ms: Option<i64>,
    pub detail: Option<String>,
    pub checked_at: String,
}

impl AgentHealthLogRecord {
    pub async fn insert(
        pool: &SqlitePool,
        agent_id: &str,
        status: &str,
        latency_ms: Option<i64>,
        detail: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, AgentHealthLogRecord>(
            r#"INSERT INTO agent_health_log (agent_id, status, latency_ms, detail)
               VALUES ($1, $2, $3, $4)
               RETURNING id, agent_id, status, latency_ms, detail, checked_at"#,
        )
        .bind(agent_id)
        .bind(status)
        .bind(latency_ms)
        .bind(detail)
        .fetch_one(pool)
        .await
    }

    pub async fn latest_for_agent(
        pool: &SqlitePool,
        agent_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentHealthLogRecord>(
            r#"SELECT id, agent_id, status, latency_ms, detail, checked_at
               FROM agent_health_log
               WHERE agent_id = $1
               ORDER BY checked_at DESC
               LIMIT 1"#,
        )
        .bind(agent_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn history_for_agent(
        pool: &SqlitePool,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentHealthLogRecord>(
            r#"SELECT id, agent_id, status, latency_ms, detail, checked_at
               FROM agent_health_log
               WHERE agent_id = $1
               ORDER BY checked_at DESC
               LIMIT $2"#,
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
