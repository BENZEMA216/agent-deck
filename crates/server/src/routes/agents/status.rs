use axum::{
    extract::{Path, State},
    response::Json as ResponseJson,
};
use deployment::Deployment;
use sqlx::Row;
use uuid::Uuid;

use agent_adapters::{
    AgentAdapter, AgentStatus, ConnectionConfig,
    claude_code::{ClaudeCodeAdapter, ClaudeCodeConfig},
    codex::{CodexAdapter, CodexConfig},
    mock::MockAdapter,
    openclaw::{OpenClawAdapter, OpenClawConfig},
};

use crate::{DeploymentImpl, error::ApiError};
use utils::response::ApiResponse;

use super::registry::AgentHealthSummary;

// ── Adapter factory ─────────────────────────────────────────────────────────

/// Build an ephemeral adapter from the DB-stored type + config.
///
/// This is intentionally simple — a future `AgentManager` will cache adapters
/// and manage their lifecycle. For now we create a fresh adapter per request.
fn build_adapter(
    agent_type: &str,
    config: &serde_json::Value,
) -> Result<Box<dyn AgentAdapter>, ApiError> {
    match agent_type {
        "openclaw" => {
            let cc: ConnectionConfig = serde_json::from_value(config.clone())
                .map_err(|e| ApiError::BadRequest(format!("invalid connection_config: {e}")))?;
            match cc {
                ConnectionConfig::WebSocket { url, token } => {
                    let oc_config = OpenClawConfig { url, token };
                    Ok(Box::new(OpenClawAdapter::new(oc_config, "openclaw".into())))
                }
                _ => Err(ApiError::BadRequest(
                    "openclaw requires a websocket connection_config".into(),
                )),
            }
        }
        "claude_code" => {
            let cc: ConnectionConfig = serde_json::from_value(config.clone())
                .map_err(|e| ApiError::BadRequest(format!("invalid connection_config: {e}")))?;
            match cc {
                ConnectionConfig::Cli { command, .. } => {
                    let cc_config = ClaudeCodeConfig { cli_path: command };
                    Ok(Box::new(ClaudeCodeAdapter::new(cc_config, "claude_code".into())))
                }
                _ => Err(ApiError::BadRequest(
                    "claude_code requires a cli connection_config".into(),
                )),
            }
        }
        "codex" => {
            let cc: ConnectionConfig = serde_json::from_value(config.clone())
                .map_err(|e| ApiError::BadRequest(format!("invalid connection_config: {e}")))?;
            match cc {
                ConnectionConfig::Cli { command, .. } => {
                    let cx_config = CodexConfig { cli_path: command };
                    Ok(Box::new(CodexAdapter::new(cx_config, "codex".into())))
                }
                _ => Err(ApiError::BadRequest(
                    "codex requires a cli connection_config".into(),
                )),
            }
        }
        _ => {
            // Fall back to a mock for unknown types so the route still works.
            Ok(Box::new(MockAdapter::healthy(agent_type)))
        }
    }
}

// ── Internal helpers used by registry.rs ─────────────────────────────────────

/// Best-effort health check — returns `None` on any error so listing never fails.
pub(crate) async fn try_health_check(
    agent_type: &str,
    config: &serde_json::Value,
) -> Option<AgentHealthSummary> {
    let adapter = build_adapter(agent_type, config).ok()?;
    let health = adapter.health_check().await.ok()?;
    Some(AgentHealthSummary::from(health))
}

/// Build an adapter for a given agent ID (fetches from DB).
pub(crate) async fn adapter_for_agent(
    deployment: &DeploymentImpl,
    id: Uuid,
) -> Result<Box<dyn AgentAdapter>, ApiError> {
    let pool = &deployment.db().pool;
    let id_str = id.to_string();

    let row = sqlx::query(
        "SELECT agent_type, connection_config, enabled FROM agent_registry WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::BadRequest(format!("agent {id} not found")))?;

    let agent_type: String = row.get("agent_type");
    let config_str: String = row.get("connection_config");
    let enabled: bool = row.get("enabled");

    if !enabled {
        return Err(ApiError::BadRequest(format!("agent {id} is disabled")));
    }

    let config: serde_json::Value =
        serde_json::from_str(&config_str).unwrap_or(serde_json::json!({}));
    build_adapter(&agent_type, &config)
}

// ── Handler ─────────────────────────────────────────────────────────────────

/// GET /agents/{id}/status — live status check via the agent adapter.
pub async fn get_agent_status(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<AgentStatus>>, ApiError> {
    let adapter = adapter_for_agent(&deployment, id).await?;
    let status = adapter
        .get_status()
        .await
        .map_err(|e| ApiError::BadRequest(format!("agent status check failed: {e}")))?;
    Ok(ResponseJson(ApiResponse::success(status)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_adapter_openclaw_valid() {
        let config = serde_json::json!({
            "type": "web_socket",
            "url": "ws://127.0.0.1:18789",
            "token": "test-token"
        });
        let adapter = build_adapter("openclaw", &config).unwrap();
        assert_eq!(adapter.display_name(), "openclaw");
    }

    #[test]
    fn build_adapter_openclaw_rejects_cli_config() {
        let config = serde_json::json!({
            "type": "cli",
            "command": "/usr/bin/openclaw",
            "args": [],
            "env": {}
        });
        assert!(build_adapter("openclaw", &config).is_err());
    }

    #[test]
    fn build_adapter_claude_code_valid() {
        let config = serde_json::json!({
            "type": "cli",
            "command": "/usr/bin/claude",
            "args": [],
            "env": {}
        });
        let adapter = build_adapter("claude_code", &config).unwrap();
        assert_eq!(adapter.display_name(), "claude_code");
    }

    #[test]
    fn build_adapter_codex_valid() {
        let config = serde_json::json!({
            "type": "cli",
            "command": "/usr/bin/codex",
            "args": [],
            "env": {}
        });
        let adapter = build_adapter("codex", &config).unwrap();
        assert_eq!(adapter.display_name(), "codex");
    }

    #[test]
    fn build_adapter_unknown_type_returns_mock() {
        let config = serde_json::json!({});
        let adapter = build_adapter("some_unknown_agent", &config).unwrap();
        // Unknown types fall back to MockAdapter.
        assert_eq!(adapter.display_name(), "some_unknown_agent");
    }

    #[tokio::test]
    async fn try_health_check_unknown_type_returns_healthy_mock() {
        // Unknown types get a MockAdapter which is always healthy.
        let result = try_health_check("unknown_type", &serde_json::json!({})).await;
        assert!(result.is_some());
        let summary = result.unwrap();
        assert_eq!(summary.status, "healthy");
    }
}
