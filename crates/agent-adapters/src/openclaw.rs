//! OpenClaw adapter — connects to the OpenClaw gateway via WebSocket RPC.
//!
//! The gateway exposes methods like `health`, `sessions.list`,
//! `channels.status`, etc. on `ws://127.0.0.1:<port>`.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use futures::Stream;
use serde_json::json;
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::{
    AdapterError, AgentAdapter, AgentCapability, AgentSession, AgentStatus, AgentType,
    HealthStatus, LogEntry, Result, SessionStatus, StatusUpdate, TaskHandle, TaskRequest,
};

/// Configuration for connecting to an OpenClaw gateway.
#[derive(Debug, Clone)]
pub struct OpenClawConfig {
    pub url: String,
    pub token: Option<String>,
}

/// Adapter that communicates with an OpenClaw gateway over WebSocket RPC.
pub struct OpenClawAdapter {
    pub config: OpenClawConfig,
    display_name: String,
    last_health: Arc<Mutex<Option<HealthStatus>>>,
}

impl OpenClawAdapter {
    pub fn new(config: OpenClawConfig, display_name: String) -> Self {
        Self {
            config,
            display_name,
            last_health: Arc::new(Mutex::new(None)),
        }
    }

    /// Send an RPC call to the OpenClaw gateway via a short-lived WebSocket.
    ///
    /// Protocol summary (OpenClaw gateway v3):
    ///
    /// 1. After WS upgrade the gateway sends a `connect.challenge` event.
    /// 2. The client replies with a `connect` **request** that includes auth,
    ///    client metadata, and the protocol version range.
    /// 3. On success the gateway responds with `hello-ok`.
    /// 4. Subsequent messages use `{"type":"req","id":"…","method":"…","params":{…}}`
    ///    and the gateway replies with `{"type":"res","id":"…","ok":true,"payload":{…}}`.
    async fn rpc_call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        use futures::{SinkExt, StreamExt};
        use tokio::time::{timeout, Duration};
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        use tokio_tungstenite::tungstenite::Message;

        // Normalise http(s) to ws(s) — users may provide HTTP URLs.
        let ws_url = self
            .config
            .url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        let connect_url = format!("{ws_url}/rpc");

        let mut request = connect_url.clone().into_client_request().map_err(|e| {
            AdapterError::ConnectionFailed(format!("invalid WS URL {connect_url}: {e}"))
        })?;

        if let Some(ref token) = self.config.token {
            request.headers_mut().insert(
                "Authorization",
                format!("Bearer {token}").parse().unwrap(),
            );
        }
        // Origin header is validated by the gateway for control-ui clients;
        // for `gateway-client` it is not required but set anyway.
        request
            .headers_mut()
            .insert("Origin", "http://localhost".parse().unwrap());

        let (ws, _) =
            tokio_tungstenite::connect_async(request)
                .await
                .map_err(|e| {
                    AdapterError::ConnectionFailed(format!(
                        "WebSocket connect to {connect_url} failed: {e}"
                    ))
                })?;

        let (mut sink, mut stream) = ws.split();

        // Helper: read the next text frame, skipping pings/pongs.
        async fn read_text(
            stream: &mut futures::stream::SplitStream<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            >,
        ) -> Result<String> {
            use futures::StreamExt;
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Text(t)) => return Ok(t.to_string()),
                    Ok(Message::Close(_)) => {
                        return Err(AdapterError::Protocol(
                            "WS closed unexpectedly".into(),
                        ))
                    }
                    Err(e) => {
                        return Err(AdapterError::Protocol(format!("WS read error: {e}")))
                    }
                    _ => {} // ping/pong/binary — skip
                }
            }
            Err(AdapterError::Protocol("WS stream ended".into()))
        }

        // 1. Read the `connect.challenge` event.
        let challenge_text = timeout(Duration::from_secs(5), read_text(&mut stream))
            .await
            .map_err(|_| {
                AdapterError::Protocol("timeout waiting for connect.challenge".into())
            })??;

        // We don't need to parse the nonce for the connect request — it's
        // only used by the control-ui for signed device tokens.
        let _ = challenge_text;

        // 2. Send the `connect` request.
        let instance_id = uuid::Uuid::new_v4().to_string();
        let auth = self
            .config
            .token
            .as_ref()
            .map(|t| json!({ "token": t }))
            .unwrap_or(json!(null));

        let connect_msg = json!({
            "type": "req",
            "id": "connect-1",
            "method": "connect",
            "params": {
                "minProtocol": 3,
                "maxProtocol": 3,
                "client": {
                    "id": "gateway-client",
                    "version": env!("CARGO_PKG_VERSION"),
                    "platform": "rust",
                    "mode": "backend",
                    "instanceId": instance_id,
                },
                "role": "operator",
                "scopes": [
                    "operator.admin",
                    "operator.read",
                    "operator.approvals",
                    "operator.pairing"
                ],
                "caps": [],
                "auth": auth,
            }
        });

        sink.send(Message::Text(connect_msg.to_string().into()))
            .await
            .map_err(|e| {
                AdapterError::Protocol(format!("failed to send connect request: {e}"))
            })?;

        // 3. Read the connect response.
        let connect_resp_text = timeout(Duration::from_secs(5), read_text(&mut stream))
            .await
            .map_err(|_| AdapterError::Protocol("timeout waiting for connect response".into()))??;

        let connect_resp: serde_json::Value =
            serde_json::from_str(connect_resp_text.as_ref()).map_err(|e| {
                AdapterError::Protocol(format!("invalid connect response JSON: {e}"))
            })?;

        if connect_resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let err_msg = connect_resp
                .pointer("/error/message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(AdapterError::Protocol(format!(
                "connect handshake failed: {err_msg}"
            )));
        }

        // 4. Send the actual RPC request.
        let req_id = uuid::Uuid::new_v4().to_string();
        let rpc_msg = json!({
            "type": "req",
            "id": req_id,
            "method": method,
            "params": params,
        });

        sink.send(Message::Text(rpc_msg.to_string().into()))
            .await
            .map_err(|e| AdapterError::Protocol(format!("failed to send RPC message: {e}")))?;

        // 5. Read the RPC response (skip any interleaved event frames).
        let resp = timeout(Duration::from_secs(10), async {
            loop {
                let text = read_text(&mut stream).await?;
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(text.as_ref()) {
                    let msg_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if msg_type == "res" {
                        // Check for RPC-level errors.
                        if obj.get("ok").and_then(|v| v.as_bool()) == Some(false) {
                            let err_msg = obj
                                .pointer("/error/message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("RPC error");
                            return Err(AdapterError::Protocol(format!(
                                "RPC {method} failed: {err_msg}"
                            )));
                        }
                        // Return the payload (or the full envelope if no payload key).
                        return Ok(
                            obj.get("payload")
                                .cloned()
                                .unwrap_or(obj.clone()),
                        );
                    }
                    // Skip event frames silently.
                }
            }
        })
        .await
        .map_err(|_| AdapterError::Protocol("timeout waiting for RPC response".into()))??;

        // 6. Gracefully close — best-effort, don't propagate errors.
        let mut ws = sink.reunite(stream).unwrap();
        let _ = ws.close(None).await;

        debug!(method, "OpenClaw RPC response received");
        Ok(resp)
    }
}

impl std::fmt::Debug for OpenClawAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenClawAdapter")
            .field("url", &self.config.url)
            .field("name", &self.display_name)
            .finish()
    }
}

static CAPABILITIES: &[AgentCapability] = &[
    AgentCapability::Chat,
    AgentCapability::TaskExecution,
    AgentCapability::CronScheduling,
    AgentCapability::MultiChannel,
    AgentCapability::SessionPersist,
    AgentCapability::HealthEndpoint,
];

#[async_trait]
impl AgentAdapter for OpenClawAdapter {
    fn agent_type(&self) -> AgentType {
        AgentType::OpenClaw
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn capabilities(&self) -> &[AgentCapability] {
        CAPABILITIES
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        let start = Instant::now();

        match self.rpc_call("health", json!({})).await {
            Ok(_) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let status = HealthStatus::Healthy { latency_ms };
                *self.last_health.lock().await = Some(status.clone());
                debug!(name = %self.display_name, latency_ms, "OpenClaw health OK");
                Ok(status)
            }
            Err(e) => {
                warn!(name = %self.display_name, error = %e, "OpenClaw health check failed");
                let status = HealthStatus::Unreachable {
                    last_seen: None, // TODO: track last successful check
                };
                *self.last_health.lock().await = Some(status.clone());
                Ok(status)
            }
        }
    }

    async fn get_status(&self) -> Result<AgentStatus> {
        let health = self.health_check().await?;
        let sessions = self.get_sessions().await.unwrap_or_default();
        let active = sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Active)
            .count() as u32;

        // Try to get version from health response
        let version = self
            .rpc_call("health", json!({}))
            .await
            .ok()
            .and_then(|v| v.get("version").and_then(|v| v.as_str()).map(String::from));

        Ok(AgentStatus {
            agent_type: AgentType::OpenClaw,
            health,
            active_sessions: active,
            uptime_secs: None,
            version,
            extra: json!({}),
        })
    }

    async fn get_sessions(&self) -> Result<Vec<AgentSession>> {
        let resp = self.rpc_call("sessions.list", json!({})).await?;

        let sessions = resp
            .get("sessions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        let id = s.get("id")?.as_str()?.to_string();
                        let name = s.get("name").and_then(|v| v.as_str()).map(String::from);
                        Some(AgentSession {
                            id,
                            name,
                            status: SessionStatus::Idle,
                            created_at: Utc::now(), // TODO: parse from response
                            updated_at: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(sessions)
    }

    async fn send_task(&self, request: TaskRequest) -> Result<TaskHandle> {
        let resp = self
            .rpc_call(
                "agent",
                json!({
                    "message": request.prompt,
                }),
            )
            .await?;

        let session_id = resp
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(TaskHandle {
            task_id: uuid::Uuid::new_v4().to_string(),
            session_id,
        })
    }

    async fn send_message(&self, session_id: &str, message: &str) -> Result<()> {
        self.rpc_call(
            "agent",
            json!({
                "sessionId": session_id,
                "message": message,
            }),
        )
        .await?;
        Ok(())
    }

    async fn subscribe_logs(
        &self,
        _session_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogEntry> + Send>>> {
        // TODO: implement WebSocket-based log streaming from gateway
        Err(AdapterError::NotSupported)
    }

    async fn subscribe_status(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = StatusUpdate> + Send>>> {
        // TODO: implement periodic status polling as a stream
        Err(AdapterError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openclaw_adapter_debug_format() {
        let adapter = OpenClawAdapter::new(
            OpenClawConfig {
                url: "ws://127.0.0.1:18789".into(),
                token: Some("secret".into()),
            },
            "Test OpenClaw".into(),
        );
        let debug = format!("{adapter:?}");
        assert!(debug.contains("OpenClawAdapter"));
        assert!(debug.contains("18789"));
        // Token should NOT appear in debug output
        assert!(!debug.contains("secret"));
    }

    #[test]
    fn openclaw_capabilities() {
        let adapter = OpenClawAdapter::new(
            OpenClawConfig {
                url: "ws://127.0.0.1:18789".into(),
                token: None,
            },
            "test".into(),
        );
        let caps = adapter.capabilities();
        assert!(caps.contains(&AgentCapability::MultiChannel));
        assert!(caps.contains(&AgentCapability::CronScheduling));
        assert!(caps.contains(&AgentCapability::HealthEndpoint));
    }
}
