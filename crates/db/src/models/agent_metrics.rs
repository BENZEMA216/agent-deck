use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentMetricsRecord {
    pub id: i64,
    pub agent_id: String,
    pub metric_type: String,
    pub value: f64,
    pub recorded_at: String,
}

impl AgentMetricsRecord {
    pub async fn insert(
        pool: &SqlitePool,
        agent_id: &str,
        metric_type: &str,
        value: f64,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, AgentMetricsRecord>(
            r#"INSERT INTO agent_metrics (agent_id, metric_type, value)
               VALUES ($1, $2, $3)
               RETURNING id, agent_id, metric_type, value, recorded_at"#,
        )
        .bind(agent_id)
        .bind(metric_type)
        .bind(value)
        .fetch_one(pool)
        .await
    }

    pub async fn latest_for_agent(
        pool: &SqlitePool,
        agent_id: &str,
        metric_type: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentMetricsRecord>(
            r#"SELECT id, agent_id, metric_type, value, recorded_at
               FROM agent_metrics
               WHERE agent_id = $1 AND metric_type = $2
               ORDER BY recorded_at DESC
               LIMIT 1"#,
        )
        .bind(agent_id)
        .bind(metric_type)
        .fetch_optional(pool)
        .await
    }

    pub async fn series_for_agent(
        pool: &SqlitePool,
        agent_id: &str,
        metric_type: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentMetricsRecord>(
            r#"SELECT id, agent_id, metric_type, value, recorded_at
               FROM agent_metrics
               WHERE agent_id = $1 AND metric_type = $2 AND recorded_at >= $3 AND recorded_at <= $4
               ORDER BY recorded_at ASC"#,
        )
        .bind(agent_id)
        .bind(metric_type)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
    }
}
