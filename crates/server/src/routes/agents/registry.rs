use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::Json as ResponseJson,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use ts_rs::TS;
use uuid::Uuid;

use agent_adapters::HealthStatus;

use crate::{DeploymentImpl, error::ApiError};
use utils::response::ApiResponse;

// ── Request / Response Types ────────────────────────────────────────────────

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateAgentRequest {
    /// One of "openclaw", "claude_code", "codex"
    pub agent_type: String,
    pub display_name: String,
    /// Freeform JSON holding adapter-specific connection details.
    #[ts(type = "Record<string, unknown>")]
    pub connection_config: serde_json::Value,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateAgentRequest {
    pub display_name: Option<String>,
    #[ts(type = "Record<string, unknown> | undefined")]
    pub connection_config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentRecord {
    pub id: Uuid,
    pub agent_type: String,
    pub display_name: String,
    #[ts(type = "Record<string, unknown>")]
    pub connection_config: serde_json::Value,
    pub enabled: bool,
    pub health: Option<AgentHealthSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentHealthSummary {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub reason: Option<String>,
}

impl From<HealthStatus> for AgentHealthSummary {
    fn from(h: HealthStatus) -> Self {
        match h {
            HealthStatus::Healthy { latency_ms } => AgentHealthSummary {
                status: "healthy".into(),
                latency_ms: Some(latency_ms),
                reason: None,
            },
            HealthStatus::Degraded { reason } => AgentHealthSummary {
                status: "degraded".into(),
                latency_ms: None,
                reason: Some(reason),
            },
            HealthStatus::Unreachable { .. } => AgentHealthSummary {
                status: "unreachable".into(),
                latency_ms: None,
                reason: None,
            },
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Validate agent_type string — accepts known types and custom strings.
fn validate_agent_type(s: &str) -> Result<(), ApiError> {
    match s {
        "openclaw" | "claude_code" | "codex" => Ok(()),
        other if !other.is_empty() => Ok(()), // allow custom types
        _ => Err(ApiError::BadRequest("agent_type must not be empty".into())),
    }
}

/// Parse an [`AgentRecord`] from a [`sqlx::sqlite::SqliteRow`].
fn agent_record_from_row(row: &sqlx::sqlite::SqliteRow) -> AgentRecord {
    let id_str: String = row.get("id");
    let id = Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4());
    let agent_type: String = row.get("agent_type");
    let display_name: String = row.get("display_name");
    let config_str: String = row.get("connection_config");
    let enabled: bool = row.get("enabled");
    let connection_config: serde_json::Value =
        serde_json::from_str(&config_str).unwrap_or(serde_json::json!({}));

    AgentRecord {
        id,
        agent_type,
        display_name,
        connection_config,
        enabled,
        health: None,
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// GET /agents — list all registered agents with a quick health summary.
pub async fn list_agents(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<AgentRecord>>>, ApiError> {
    let pool = &deployment.db().pool;

    let rows = sqlx::query(
        "SELECT id, agent_type, display_name, connection_config, enabled FROM agent_registry ORDER BY rowid",
    )
    .fetch_all(pool)
    .await?;

    let mut agents = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut record = agent_record_from_row(row);

        // Best-effort health probe for enabled agents — do not block on errors.
        if record.enabled {
            record.health =
                super::status::try_health_check(&record.agent_type, &record.connection_config)
                    .await;
        }

        agents.push(record);
    }

    Ok(ResponseJson(ApiResponse::success(agents)))
}

/// POST /agents — register a new agent.
pub async fn create_agent(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateAgentRequest>,
) -> Result<(StatusCode, ResponseJson<ApiResponse<AgentRecord>>), ApiError> {
    validate_agent_type(&payload.agent_type)?;

    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();
    let id_str = id.to_string();
    let config_str = serde_json::to_string(&payload.connection_config)
        .map_err(|e| ApiError::BadRequest(format!("invalid connection_config: {e}")))?;

    sqlx::query(
        "INSERT INTO agent_registry (id, agent_type, display_name, connection_config, enabled) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id_str)
    .bind(&payload.agent_type)
    .bind(&payload.display_name)
    .bind(&config_str)
    .bind(true)
    .execute(pool)
    .await?;

    let record = AgentRecord {
        id,
        agent_type: payload.agent_type,
        display_name: payload.display_name,
        connection_config: payload.connection_config,
        enabled: true,
        health: None,
    };

    Ok((
        StatusCode::CREATED,
        ResponseJson(ApiResponse::success(record)),
    ))
}

/// GET /agents/{id} — fetch a single agent by ID with latest health.
pub async fn get_agent(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<AgentRecord>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_str = id.to_string();

    let row = sqlx::query(
        "SELECT id, agent_type, display_name, connection_config, enabled FROM agent_registry WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::BadRequest(format!("agent {id} not found")))?;

    let mut record = agent_record_from_row(&row);

    if record.enabled {
        record.health =
            super::status::try_health_check(&record.agent_type, &record.connection_config).await;
    }

    Ok(ResponseJson(ApiResponse::success(record)))
}

/// PUT /agents/{id} — update an agent's config.
pub async fn update_agent(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAgentRequest>,
) -> Result<ResponseJson<ApiResponse<AgentRecord>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_str = id.to_string();

    // Ensure the agent exists.
    let _existing = sqlx::query("SELECT 1 FROM agent_registry WHERE id = ?")
        .bind(&id_str)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::BadRequest(format!("agent {id} not found")))?;

    if let Some(ref name) = payload.display_name {
        sqlx::query("UPDATE agent_registry SET display_name = ? WHERE id = ?")
            .bind(name)
            .bind(&id_str)
            .execute(pool)
            .await?;
    }
    if let Some(ref config) = payload.connection_config {
        let config_str = serde_json::to_string(config)
            .map_err(|e| ApiError::BadRequest(format!("invalid connection_config: {e}")))?;
        sqlx::query("UPDATE agent_registry SET connection_config = ? WHERE id = ?")
            .bind(&config_str)
            .bind(&id_str)
            .execute(pool)
            .await?;
    }
    if let Some(enabled) = payload.enabled {
        sqlx::query("UPDATE agent_registry SET enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(&id_str)
            .execute(pool)
            .await?;
    }

    // Re-fetch updated record.
    let row = sqlx::query(
        "SELECT id, agent_type, display_name, connection_config, enabled FROM agent_registry WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    let record = agent_record_from_row(&row);

    Ok(ResponseJson(ApiResponse::success(record)))
}

/// DELETE /agents/{id} — deregister an agent.
pub async fn delete_agent(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, ResponseJson<ApiResponse<()>>), ApiError> {
    let pool = &deployment.db().pool;
    let id_str = id.to_string();

    let result = sqlx::query("DELETE FROM agent_registry WHERE id = ?")
        .bind(&id_str)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::BadRequest(format!("agent {id} not found")));
    }

    Ok((StatusCode::OK, ResponseJson(ApiResponse::success(()))))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_agent_type --------------------------------------------------

    #[test]
    fn validate_known_agent_types() {
        assert!(validate_agent_type("openclaw").is_ok());
        assert!(validate_agent_type("claude_code").is_ok());
        assert!(validate_agent_type("codex").is_ok());
    }

    #[test]
    fn validate_custom_agent_type() {
        assert!(validate_agent_type("my_custom_agent").is_ok());
        assert!(validate_agent_type("x").is_ok());
    }

    #[test]
    fn validate_empty_agent_type_fails() {
        assert!(validate_agent_type("").is_err());
    }

    // -- AgentHealthSummary from HealthStatus ---------------------------------

    #[test]
    fn health_summary_from_healthy() {
        let summary = AgentHealthSummary::from(HealthStatus::Healthy { latency_ms: 42 });
        assert_eq!(summary.status, "healthy");
        assert_eq!(summary.latency_ms, Some(42));
        assert!(summary.reason.is_none());
    }

    #[test]
    fn health_summary_from_degraded() {
        let summary = AgentHealthSummary::from(HealthStatus::Degraded {
            reason: "slow".into(),
        });
        assert_eq!(summary.status, "degraded");
        assert!(summary.latency_ms.is_none());
        assert_eq!(summary.reason.as_deref(), Some("slow"));
    }

    #[test]
    fn health_summary_from_unreachable() {
        let summary = AgentHealthSummary::from(HealthStatus::Unreachable { last_seen: None });
        assert_eq!(summary.status, "unreachable");
        assert!(summary.latency_ms.is_none());
        assert!(summary.reason.is_none());
    }

    // -- AgentRecord serialization round-trip ---------------------------------

    #[test]
    fn agent_record_json_round_trip() {
        let record = AgentRecord {
            id: Uuid::new_v4(),
            agent_type: "openclaw".into(),
            display_name: "Test Agent".into(),
            connection_config: serde_json::json!({"type": "web_socket", "url": "ws://localhost:18789"}),
            enabled: true,
            health: Some(AgentHealthSummary {
                status: "healthy".into(),
                latency_ms: Some(5),
                reason: None,
            }),
        };

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: AgentRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_type, "openclaw");
        assert_eq!(deserialized.display_name, "Test Agent");
        assert!(deserialized.enabled);
        assert!(deserialized.health.is_some());
    }

    // -- CreateAgentRequest deserialization ------------------------------------

    #[test]
    fn create_agent_request_deserializes() {
        let json = r#"{
            "agent_type": "openclaw",
            "display_name": "My Agent",
            "connection_config": {"type": "web_socket", "url": "ws://localhost:18789", "token": "abc"}
        }"#;
        let req: CreateAgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.agent_type, "openclaw");
        assert_eq!(req.display_name, "My Agent");
    }

    // -- UpdateAgentRequest deserialization ------------------------------------

    #[test]
    fn update_agent_request_all_fields() {
        let json = r#"{
            "display_name": "Renamed",
            "connection_config": {"type": "web_socket", "url": "ws://localhost:9999"},
            "enabled": false
        }"#;
        let req: UpdateAgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.display_name.as_deref(), Some("Renamed"));
        assert!(req.connection_config.is_some());
        assert_eq!(req.enabled, Some(false));
    }

    #[test]
    fn update_agent_request_partial() {
        let json = r#"{"enabled": true}"#;
        let req: UpdateAgentRequest = serde_json::from_str(json).unwrap();
        assert!(req.display_name.is_none());
        assert!(req.connection_config.is_none());
        assert_eq!(req.enabled, Some(true));
    }
}
