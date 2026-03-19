//! Claude Code adapter — wraps the Claude Code CLI for health, status, and task execution.

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

/// Configuration for the Claude Code CLI adapter.
#[derive(Debug, Clone)]
pub struct ClaudeCodeConfig {
    /// Path to the `claude` CLI binary.
    pub cli_path: String,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        Self {
            cli_path: "claude".into(),
        }
    }
}

pub struct ClaudeCodeAdapter {
    config: ClaudeCodeConfig,
    display_name: String,
}

impl ClaudeCodeAdapter {
    pub fn new(config: ClaudeCodeConfig, display_name: String) -> Self {
        Self {
            config,
            display_name,
        }
    }
}

impl std::fmt::Debug for ClaudeCodeAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeCodeAdapter")
            .field("cli_path", &self.config.cli_path)
            .field("name", &self.display_name)
            .finish()
    }
}

static CAPABILITIES: &[AgentCapability] = &[
    AgentCapability::Chat,
    AgentCapability::TaskExecution,
    AgentCapability::SessionPersist,
];

#[async_trait]
impl AgentAdapter for ClaudeCodeAdapter {
    fn agent_type(&self) -> AgentType {
        AgentType::ClaudeCode
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
                    "Claude Code health OK"
                );
                Ok(HealthStatus::Healthy { latency_ms })
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                warn!(name = %self.display_name, stderr = %stderr, "Claude Code CLI returned error");
                Ok(HealthStatus::Degraded {
                    reason: format!("CLI exited with {}: {}", out.status, stderr.trim()),
                })
            }
            Err(e) => {
                warn!(name = %self.display_name, error = %e, "Claude Code CLI not found");
                Ok(HealthStatus::Unreachable {
                    last_seen: None,
                })
            }
        }
    }

    async fn get_status(&self) -> Result<AgentStatus> {
        let health = self.health_check().await?;

        // Extract version from CLI
        let version = Command::new(&self.config.cli_path)
            .arg("--version")
            .output()
            .await
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        Ok(AgentStatus {
            agent_type: AgentType::ClaudeCode,
            health,
            active_sessions: 0, // TODO: detect running claude processes
            uptime_secs: None,
            version,
            extra: serde_json::json!({}),
        })
    }

    async fn get_sessions(&self) -> Result<Vec<AgentSession>> {
        // Claude Code sessions are managed by the Vibe Kanban executor layer.
        // For now, return empty. Future: parse ~/.claude/ for session history.
        Ok(vec![])
    }

    async fn send_task(&self, request: TaskRequest) -> Result<TaskHandle> {
        // TODO: integrate with the existing ClaudeCode executor from
        // crates/executors/src/executors/claude.rs for full task execution.
        //
        // For now, spawn a simple one-shot command.
        let mut cmd = Command::new(&self.config.cli_path);
        cmd.arg("--print").arg(&request.prompt);

        if let Some(ref dir) = request.working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd.spawn().map_err(|e| {
            AdapterError::Internal(format!("failed to spawn claude: {e}"))
        })?;

        let pid = child.id().unwrap_or(0);

        Ok(TaskHandle {
            task_id: format!("claude-{pid}"),
            session_id: format!("claude-session-{pid}"),
        })
    }

    async fn subscribe_logs(
        &self,
        _session_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogEntry> + Send>>> {
        // TODO: pipe PTY stdout into a log stream
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
    fn claude_code_capabilities() {
        let adapter = ClaudeCodeAdapter::new(ClaudeCodeConfig::default(), "test".into());
        let caps = adapter.capabilities();
        assert!(caps.contains(&AgentCapability::TaskExecution));
        assert!(caps.contains(&AgentCapability::SessionPersist));
        assert!(!caps.contains(&AgentCapability::MultiChannel));
    }

    #[test]
    fn claude_code_agent_type() {
        let adapter = ClaudeCodeAdapter::new(ClaudeCodeConfig::default(), "test".into());
        assert_eq!(adapter.agent_type(), AgentType::ClaudeCode);
    }

    #[tokio::test]
    async fn claude_code_health_with_invalid_path() {
        let adapter = ClaudeCodeAdapter::new(
            ClaudeCodeConfig {
                cli_path: "/nonexistent/claude".into(),
            },
            "test".into(),
        );
        let health = adapter.health_check().await.unwrap();
        assert!(matches!(health, HealthStatus::Unreachable { .. }));
    }
}
