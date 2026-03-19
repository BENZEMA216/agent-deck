use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentRegistryRecord {
    pub id: String,
    pub agent_type: String,
    pub display_name: String,
    pub connection_config: String,
    pub capabilities: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentRegistryRecord {
    pub async fn create(
        pool: &SqlitePool,
        id: &str,
        agent_type: &str,
        display_name: &str,
        connection_config: &str,
        capabilities: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, AgentRegistryRecord>(
            r#"INSERT INTO agent_registry (id, agent_type, display_name, connection_config, capabilities)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, agent_type, display_name, connection_config, capabilities, enabled, created_at, updated_at"#,
        )
        .bind(id)
        .bind(agent_type)
        .bind(display_name)
        .bind(connection_config)
        .bind(capabilities)
        .fetch_one(pool)
        .await
    }

    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentRegistryRecord>(
            r#"SELECT id, agent_type, display_name, connection_config, capabilities, enabled, created_at, updated_at
               FROM agent_registry
               WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn list_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentRegistryRecord>(
            r#"SELECT id, agent_type, display_name, connection_config, capabilities, enabled, created_at, updated_at
               FROM agent_registry
               ORDER BY created_at DESC"#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_enabled(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, AgentRegistryRecord>(
            r#"SELECT id, agent_type, display_name, connection_config, capabilities, enabled, created_at, updated_at
               FROM agent_registry
               WHERE enabled = 1
               ORDER BY created_at DESC"#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        agent_type: Option<&str>,
        display_name: Option<&str>,
        connection_config: Option<&str>,
        capabilities: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::get_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let agent_type = agent_type.unwrap_or(&existing.agent_type);
        let display_name = display_name.unwrap_or(&existing.display_name);
        let connection_config = connection_config.unwrap_or(&existing.connection_config);
        let capabilities = capabilities.unwrap_or(&existing.capabilities);
        let enabled = enabled.unwrap_or(existing.enabled);

        sqlx::query_as::<_, AgentRegistryRecord>(
            r#"UPDATE agent_registry
               SET agent_type = $2, display_name = $3, connection_config = $4, capabilities = $5, enabled = $6, updated_at = datetime('now')
               WHERE id = $1
               RETURNING id, agent_type, display_name, connection_config, capabilities, enabled, created_at, updated_at"#,
        )
        .bind(id)
        .bind(agent_type)
        .bind(display_name)
        .bind(connection_config)
        .bind(capabilities)
        .bind(enabled)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM agent_registry WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}
