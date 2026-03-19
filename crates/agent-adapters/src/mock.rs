//! Mock adapter for testing.

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use futures::Stream;
use tokio::sync::Mutex;

use crate::{
    AdapterError, AgentAdapter, AgentCapability, AgentSession, AgentStatus, AgentType,
    HealthStatus, LogEntry, LogLevel, Result, SessionStatus, StatusUpdate,
    TaskHandle, TaskRequest,
};

/// A configurable mock adapter for unit and integration tests.
#[derive(Debug, Clone)]
pub struct MockAdapter {
    pub name: String,
    pub health: Arc<Mutex<HealthStatus>>,
    pub sessions: Arc<Mutex<Vec<AgentSession>>>,
    pub fail_on_task: bool,
}

impl MockAdapter {
    pub fn healthy(name: &str) -> Self {
        Self {
            name: name.to_string(),
            health: Arc::new(Mutex::new(HealthStatus::Healthy { latency_ms: 1 })),
            sessions: Arc::new(Mutex::new(vec![])),
            fail_on_task: false,
        }
    }

    pub fn unreachable(name: &str) -> Self {
        Self {
            name: name.to_string(),
            health: Arc::new(Mutex::new(HealthStatus::Unreachable { last_seen: None })),
            sessions: Arc::new(Mutex::new(vec![])),
            fail_on_task: false,
        }
    }

    /// Inject a session into the mock.
    pub async fn add_session(&self, id: &str, status: SessionStatus) {
        self.sessions.lock().await.push(AgentSession {
            id: id.to_string(),
            name: Some(format!("mock-session-{id}")),
            status,
            created_at: Utc::now(),
            updated_at: None,
        });
    }

    /// Change health status at runtime (for testing transitions).
    pub async fn set_health(&self, h: HealthStatus) {
        *self.health.lock().await = h;
    }
}

#[async_trait]
impl AgentAdapter for MockAdapter {
    fn agent_type(&self) -> AgentType {
        AgentType::Custom("mock".into())
    }

    fn display_name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &[AgentCapability] {
        &[
            AgentCapability::Chat,
            AgentCapability::TaskExecution,
            AgentCapability::HealthEndpoint,
        ]
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(self.health.lock().await.clone())
    }

    async fn get_status(&self) -> Result<AgentStatus> {
        let health = self.health.lock().await.clone();
        let sessions = self.sessions.lock().await;
        let active = sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Active)
            .count() as u32;

        Ok(AgentStatus {
            agent_type: self.agent_type(),
            health,
            active_sessions: active,
            uptime_secs: Some(42),
            version: Some("mock-1.0".into()),
            extra: serde_json::json!({}),
        })
    }

    async fn get_sessions(&self) -> Result<Vec<AgentSession>> {
        Ok(self.sessions.lock().await.clone())
    }

    async fn send_task(&self, request: TaskRequest) -> Result<TaskHandle> {
        if self.fail_on_task {
            return Err(AdapterError::Internal("mock task failure".into()));
        }
        let session_id = request
            .session_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        Ok(TaskHandle {
            task_id: uuid::Uuid::new_v4().to_string(),
            session_id,
        })
    }

    async fn stop_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        if let Some(s) = sessions.iter_mut().find(|s| s.id == session_id) {
            s.status = SessionStatus::Completed;
            Ok(())
        } else {
            Err(AdapterError::Internal(format!(
                "session {session_id} not found"
            )))
        }
    }

    async fn send_message(&self, _session_id: &str, _message: &str) -> Result<()> {
        Ok(())
    }

    async fn subscribe_logs(
        &self,
        _session_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogEntry> + Send>>> {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            message: "mock log entry".into(),
            source: self.name.clone(),
        };
        Ok(Box::pin(futures::stream::once(async { entry })))
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

    #[tokio::test]
    async fn healthy_mock_returns_healthy() {
        let mock = MockAdapter::healthy("test-agent");
        let health = mock.health_check().await.unwrap();
        assert!(matches!(health, HealthStatus::Healthy { .. }));
    }

    #[tokio::test]
    async fn unreachable_mock_returns_unreachable() {
        let mock = MockAdapter::unreachable("test-agent");
        let health = mock.health_check().await.unwrap();
        assert!(matches!(health, HealthStatus::Unreachable { .. }));
    }

    #[tokio::test]
    async fn mock_session_management() {
        let mock = MockAdapter::healthy("test");
        assert_eq!(mock.get_sessions().await.unwrap().len(), 0);

        mock.add_session("s1", SessionStatus::Active).await;
        mock.add_session("s2", SessionStatus::Idle).await;

        let sessions = mock.get_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);

        let status = mock.get_status().await.unwrap();
        assert_eq!(status.active_sessions, 1);
    }

    #[tokio::test]
    async fn mock_stop_session() {
        let mock = MockAdapter::healthy("test");
        mock.add_session("s1", SessionStatus::Active).await;

        mock.stop_session("s1").await.unwrap();

        let sessions = mock.get_sessions().await.unwrap();
        assert_eq!(sessions[0].status, SessionStatus::Completed);
    }

    #[tokio::test]
    async fn mock_send_task() {
        let mock = MockAdapter::healthy("test");
        let handle = mock
            .send_task(TaskRequest {
                prompt: "hello".into(),
                working_dir: None,
                session_id: None,
            })
            .await
            .unwrap();
        assert!(!handle.task_id.is_empty());
    }

    #[tokio::test]
    async fn mock_task_failure() {
        let mut mock = MockAdapter::healthy("test");
        mock.fail_on_task = true;
        let result = mock
            .send_task(TaskRequest {
                prompt: "hello".into(),
                working_dir: None,
                session_id: None,
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_health_transition() {
        let mock = MockAdapter::healthy("test");
        assert!(matches!(
            mock.health_check().await.unwrap(),
            HealthStatus::Healthy { .. }
        ));

        mock.set_health(HealthStatus::Degraded {
            reason: "test".into(),
        })
        .await;
        assert!(matches!(
            mock.health_check().await.unwrap(),
            HealthStatus::Degraded { .. }
        ));
    }

    #[tokio::test]
    async fn mock_capabilities() {
        let mock = MockAdapter::healthy("test");
        let caps = mock.capabilities();
        assert!(caps.contains(&AgentCapability::Chat));
        assert!(caps.contains(&AgentCapability::TaskExecution));
        assert!(caps.contains(&AgentCapability::HealthEndpoint));
    }

    #[tokio::test]
    async fn mock_subscribe_logs() {
        use futures::StreamExt;
        let mock = MockAdapter::healthy("test");
        let mut stream = mock.subscribe_logs("s1").await.unwrap();
        let entry = stream.next().await.unwrap();
        assert_eq!(entry.message, "mock log entry");
    }
}
