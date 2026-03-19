//! Agent Adapters — unified abstraction layer for heterogeneous AI agents.
//!
//! Each agent type (OpenClaw, Claude Code, Codex, etc.) implements the
//! [`AgentAdapter`] trait, exposing a common interface for health checks,
//! status queries, task dispatch, and log streaming.

pub mod claude_code;
pub mod codex;
pub mod mock;
pub mod openclaw;

use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

// ── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("operation not supported by this agent")]
    NotSupported,

    #[error("agent unreachable: {0}")]
    Unreachable(String),

    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AdapterError>;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    OpenClaw,
    ClaudeCode,
    Codex,
    Custom(String),
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenClaw => write!(f, "openclaw"),
            Self::ClaudeCode => write!(f, "claude_code"),
            Self::Codex => write!(f, "codex"),
            Self::Custom(s) => write!(f, "custom:{s}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentCapability {
    /// Can accept and respond to chat messages.
    Chat,
    /// Can execute coding or analysis tasks.
    TaskExecution,
    /// Has built-in cron / scheduling support.
    CronScheduling,
    /// Supports multiple messaging channels (Telegram, Discord, etc.).
    MultiChannel,
    /// Persists session history across restarts.
    SessionPersist,
    /// Exposes a native health-check endpoint.
    HealthEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy {
        latency_ms: u64,
    },
    Degraded {
        reason: String,
    },
    Unreachable {
        last_seen: Option<DateTime<Utc>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentStatus {
    pub agent_type: AgentType,
    pub health: HealthStatus,
    pub active_sessions: u32,
    pub uptime_secs: Option<u64>,
    pub version: Option<String>,
    /// Agent-specific metadata (channels, skills, etc.)
    #[ts(type = "Record<string, unknown>")]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentSession {
    pub id: String,
    pub name: Option<String>,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Idle,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub prompt: String,
    pub working_dir: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskHandle {
    pub task_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct StatusUpdate {
    pub agent_id: String,
    pub status: AgentStatus,
    pub timestamp: DateTime<Utc>,
}

// ── Connection Config ────────────────────────────────────────────────────────

/// Serialized into `agent_registry.connection_config` (JSON column).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectionConfig {
    /// WebSocket-based agent (OpenClaw gateway).
    WebSocket {
        url: String,
        token: Option<String>,
    },
    /// CLI-based agent (Claude Code, Codex).
    Cli {
        command: String,
        args: Vec<String>,
        env: std::collections::HashMap<String, String>,
    },
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Unified interface for all agent types.
///
/// Implementors provide agent-specific logic for health checks, status queries,
/// task dispatch, and log streaming. Operations that a particular agent does
/// not support should return [`AdapterError::NotSupported`].
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    // ── Identity ──

    fn agent_type(&self) -> AgentType;
    fn display_name(&self) -> &str;
    fn capabilities(&self) -> &[AgentCapability];

    // ── Lifecycle ──

    async fn health_check(&self) -> Result<HealthStatus>;
    async fn connect(&mut self) -> Result<()> {
        Ok(()) // default no-op for stateless adapters
    }
    async fn disconnect(&mut self) -> Result<()> {
        Ok(()) // default no-op
    }

    // ── Status ──

    async fn get_status(&self) -> Result<AgentStatus>;
    async fn get_sessions(&self) -> Result<Vec<AgentSession>>;

    // ── Control ──

    async fn send_task(&self, _request: TaskRequest) -> Result<TaskHandle> {
        Err(AdapterError::NotSupported)
    }

    async fn stop_session(&self, _session_id: &str) -> Result<()> {
        Err(AdapterError::NotSupported)
    }

    async fn send_message(&self, _session_id: &str, _message: &str) -> Result<()> {
        Err(AdapterError::NotSupported)
    }

    // ── Streaming ──

    async fn subscribe_logs(
        &self,
        _session_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogEntry> + Send>>> {
        Err(AdapterError::NotSupported)
    }

    async fn subscribe_status(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = StatusUpdate> + Send>>> {
        Err(AdapterError::NotSupported)
    }
}
