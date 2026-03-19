use axum::{
    extract::{Path, State},
    response::Json as ResponseJson,
    Json,
};
use serde::Deserialize;
use ts_rs::TS;
use uuid::Uuid;

use agent_adapters::{AgentSession, TaskHandle, TaskRequest};

use crate::{DeploymentImpl, error::ApiError};
use utils::response::ApiResponse;

use super::status::adapter_for_agent;

// ── Request types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SendTaskRequest {
    pub prompt: String,
    pub working_dir: Option<String>,
    pub session_id: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// POST /agents/{id}/task — dispatch a task to the agent.
pub async fn send_task(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(payload): Json<SendTaskRequest>,
) -> Result<ResponseJson<ApiResponse<TaskHandle>>, ApiError> {
    let adapter = adapter_for_agent(&deployment, id).await?;

    let request = TaskRequest {
        prompt: payload.prompt,
        working_dir: payload.working_dir,
        session_id: payload.session_id,
    };

    let handle = adapter
        .send_task(request)
        .await
        .map_err(|e| ApiError::BadRequest(format!("send_task failed: {e}")))?;

    Ok(ResponseJson(ApiResponse::success(handle)))
}

/// GET /agents/{id}/sessions — list active sessions for the agent.
pub async fn list_sessions(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<AgentSession>>>, ApiError> {
    let adapter = adapter_for_agent(&deployment, id).await?;

    let sessions = adapter
        .get_sessions()
        .await
        .map_err(|e| ApiError::BadRequest(format!("get_sessions failed: {e}")))?;

    Ok(ResponseJson(ApiResponse::success(sessions)))
}
