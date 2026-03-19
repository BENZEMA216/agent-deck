//! Codex adapter — wraps the Codex CLI for health, status, and task execution.

use std::pin::Pin;
use std::time::Instant;

use async_trait::async_trait;
use futures::Stream;
use tokio::process::Command;
use tracing::{debug, warn};

use crate::{
    AdapterError, AgentAdapter, AgentCapability, AgentSession, AgentStatus, AgentType,
    HealthStatus, LogEntry, Result, StatusUpdate, TaskHandle, TaskRequest,
};

/// Configuration for the Codex CLI adapter.
#[derive(Debug, Clone)]
pub struct CodexConfig {
    /// Path to the `codex` CLI binary.
    pub cli_path: String,
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            cli_path: "codex".into(),
        }
    }
}

pub struct CodexAdapter {
    config: CodexConfig,
    display_name: String,
}

impl CodexAdapter {
    pub fn new(config: CodexConfig, display_name: String) -> Self {
        Self {
            config,
            display_name,
        }
    }
}

impl std::fmt::Debug for CodexAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexAdapter")
            .field("cli_path", &self.config.cli_path)
            .field("name", &self.display_name)
            .finish()
    }
}

static CAPABILITIES: &[AgentCapability] = &[
    AgentCapability::Chat,
    AgentCapability::TaskExecution,
];

#[async_trait]
impl AgentAdapter for CodexAdapter {
    fn agent_type(&self) -> AgentType {
        AgentType::Codex
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn capabilities(&self) -> &[AgentCapability] {
        CAPABILITIES
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        let start = Instant::now();

        let output = Command::new(&self.config.cli_path)
            .arg("--version")
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                let latency_ms = start.elapsed().as_millis() as u64;
                debug!(
                    name = %self.display_name,
                    version = %String::from_utf8_lossy(&out.stdout).trim(),
                    latency_ms,
                    "Codex health OK"
                );
                Ok(HealthStatus::Healthy { latency_ms })
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                warn!(name = %self.display_name, stderr = %stderr, "Codex CLI returned error");
                Ok(HealthStatus::Degraded {
                    reason: format!("CLI exited with {}: {}", out.status, stderr.trim()),
                })
            }
            Err(e) => {
                warn!(name = %self.display_name, error = %e, "Codex CLI not found");
                Ok(HealthStatus::Unreachable {
                    last_seen: None,
                })
            }
        }
    }

    async fn get_status(&self) -> Result<AgentStatus> {
        let health = self.health_check().await?;

        let version = Command::new(&self.config.cli_path)
            .arg("--version")
            .output()
            .await
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        Ok(AgentStatus {
            agent_type: AgentType::Codex,
            health,
            active_sessions: 0,
            uptime_secs: None,
            version,
            extra: serde_json::json!({}),
        })
    }

    async fn get_sessions(&self) -> Result<Vec<AgentSession>> {
        Ok(vec![])
    }

    async fn send_task(&self, request: TaskRequest) -> Result<TaskHandle> {
        let mut cmd = Command::new(&self.config.cli_path);
        cmd.arg(&request.prompt);

        if let Some(ref dir) = request.working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd.spawn().map_err(|e| {
            AdapterError::Internal(format!("failed to spawn codex: {e}"))
        })?;

        let pid = child.id().unwrap_or(0);

        Ok(TaskHandle {
            task_id: format!("codex-{pid}"),
            session_id: format!("codex-session-{pid}"),
        })
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_capabilities() {
        let adapter = CodexAdapter::new(CodexConfig::default(), "test".into());
        let caps = adapter.capabilities();
        assert!(caps.contains(&AgentCapability::TaskExecution));
        assert!(!caps.contains(&AgentCapability::CronScheduling));
    }

    #[test]
    fn codex_agent_type() {
        let adapter = CodexAdapter::new(CodexConfig::default(), "test".into());
        assert_eq!(adapter.agent_type(), AgentType::Codex);
    }

    #[tokio::test]
    async fn codex_health_with_invalid_path() {
        let adapter = CodexAdapter::new(
            CodexConfig {
                cli_path: "/nonexistent/codex".into(),
            },
            "test".into(),
        );
        let health = adapter.health_check().await.unwrap();
        assert!(matches!(health, HealthStatus::Unreachable { .. }));
    }
}
