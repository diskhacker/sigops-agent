use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::discovery::{DiscoveredTool, HostInfo};
use crate::executor::{TaskCommand, TaskResult};
use crate::security::TokenStore;

/// Calculate exponential backoff delay for retry attempts.
/// Backoff sequence: 2s, 4s, 8s, 16s, capped at 60s.
pub fn backoff_delay_secs(attempt: u32) -> u64 {
    let delay = 2u64.saturating_pow(attempt + 1); // 2^1=2, 2^2=4, 2^3=8, 2^4=16, ...
    delay.min(60)
}

/// Maximum consecutive failures before logging a warning.
const MAX_CONSECUTIVE_FAILURES: u32 = 5;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatPayload {
    pub agent_id: String,
    pub tenant_id: String,
    pub hostname: String,
    pub os: String,
    pub agent_version: String,
    pub tools: Vec<DiscoveredTool>,
    pub labels: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatResponse {
    pub agent_id: String,
    pub status: String,
    /// Optional server-issued rotated bearer token. When present, the agent
    /// swaps its in-memory + on-disk token before the next request.
    #[serde(default)]
    pub rotated_token: Option<String>,
}

pub struct HeartbeatClient {
    client: Client,
    server_url: String,
    tokens: TokenStore,
}

impl HeartbeatClient {
    pub fn new(server_url: &str, api_token: &str) -> Self {
        Self {
            client: Client::new(),
            server_url: server_url.trim_end_matches('/').to_string(),
            tokens: TokenStore::new(api_token),
        }
    }

    /// Construct a client using a shared [`TokenStore`] so rotations from
    /// the server propagate to any other subsystem that holds the store.
    #[allow(dead_code)]
    pub fn with_token_store(server_url: &str, tokens: TokenStore) -> Self {
        Self {
            client: Client::new(),
            server_url: server_url.trim_end_matches('/').to_string(),
            tokens,
        }
    }

    /// Access the underlying token store (for rotation, inspection in tests,
    /// or sharing with other subsystems).
    #[allow(dead_code)]
    pub fn token_store(&self) -> TokenStore {
        self.tokens.clone()
    }

    pub async fn send_heartbeat(
        &self,
        agent_id: &str,
        tenant_id: &str,
        host_info: &HostInfo,
        tools: &[DiscoveredTool],
    ) -> Result<HeartbeatResponse, HeartbeatError> {
        let payload = HeartbeatPayload {
            agent_id: agent_id.to_string(),
            tenant_id: tenant_id.to_string(),
            hostname: host_info.hostname.clone(),
            os: host_info.os.clone(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            tools: tools.to_vec(),
            labels: serde_json::json!({}),
        };

        let url = format!("{}/api/v1/agents/heartbeat", self.server_url);
        let res = self
            .client
            .post(&url)
            .bearer_auth(self.tokens.current())
            .json(&payload)
            .send()
            .await
            .map_err(|e| HeartbeatError::Network(e.to_string()))?;

        if res.status().is_success() {
            let body = res
                .json::<HeartbeatResponse>()
                .await
                .map_err(|e| HeartbeatError::Parse(e.to_string()))?;
            info!(agent_id = %body.agent_id, status = %body.status, "heartbeat OK");
            // If the server issued a rotated token, apply it. Invalid tokens
            // are logged and discarded so we stay on the previous one.
            if let Some(new_token) = body.rotated_token.as_deref() {
                match self.tokens.maybe_rotate(Some(new_token)) {
                    Ok(true) => info!("bearer token rotated from heartbeat response"),
                    Ok(false) => {}
                    Err(e) => warn!(error = %e, "rejected rotated token from server"),
                }
            }
            Ok(body)
        } else {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            warn!(status, body = %body, "heartbeat failed");
            Err(HeartbeatError::Server { status, body })
        }
    }

    /// Send a heartbeat with retry and exponential backoff.
    /// Retries up to `MAX_CONSECUTIVE_FAILURES` times on failure.
    /// Backoff: 2s, 4s, 8s, 16s, max 60s.
    pub async fn send_heartbeat_with_retry(
        &self,
        agent_id: &str,
        tenant_id: &str,
        host_info: &HostInfo,
        tools: &[DiscoveredTool],
    ) -> Result<HeartbeatResponse, HeartbeatError> {
        let mut last_err = None;
        for attempt in 0..MAX_CONSECUTIVE_FAILURES {
            match self
                .send_heartbeat(agent_id, tenant_id, host_info, tools)
                .await
            {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    let delay = backoff_delay_secs(attempt);
                    if attempt + 1 >= MAX_CONSECUTIVE_FAILURES {
                        warn!(
                            attempt = attempt + 1,
                            max = MAX_CONSECUTIVE_FAILURES,
                            "heartbeat exceeded max consecutive failures"
                        );
                    }
                    warn!(
                        attempt = attempt + 1,
                        delay_secs = delay,
                        error = %e,
                        "heartbeat failed, retrying with backoff"
                    );
                    last_err = Some(e);
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }
        error!(
            "heartbeat failed after {} retries, continuing to next cycle",
            MAX_CONSECUTIVE_FAILURES
        );
        Err(last_err.unwrap())
    }

    /// Fetch the next pending command for this agent from the server.
    /// Returns `Ok(None)` when the server has no pending work (HTTP 204).
    pub async fn fetch_next_command(
        &self,
        agent_id: &str,
    ) -> Result<Option<TaskCommand>, HeartbeatError> {
        let url = format!(
            "{}/api/v1/agents/{}/command/next",
            self.server_url, agent_id
        );
        let res = self
            .client
            .get(&url)
            .bearer_auth(self.tokens.current())
            .send()
            .await
            .map_err(|e| HeartbeatError::Network(e.to_string()))?;

        let status = res.status();
        if status.as_u16() == 204 {
            return Ok(None);
        }
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            warn!(status = status.as_u16(), body = %body, "fetch_next_command failed");
            return Err(HeartbeatError::Server {
                status: status.as_u16(),
                body,
            });
        }
        let cmd = res
            .json::<TaskCommand>()
            .await
            .map_err(|e| HeartbeatError::Parse(e.to_string()))?;
        Ok(Some(cmd))
    }

    /// Report a completed task result back to the server.
    pub async fn report_result(&self, result: &TaskResult) -> Result<(), HeartbeatError> {
        let url = format!(
            "{}/api/v1/agents/command/{}/result",
            self.server_url, result.task_id
        );
        let res = self
            .client
            .post(&url)
            .bearer_auth(self.tokens.current())
            .json(result)
            .send()
            .await
            .map_err(|e| HeartbeatError::Network(e.to_string()))?;

        if res.status().is_success() {
            info!(task_id = %result.task_id, "result reported");
            Ok(())
        } else {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            warn!(status, body = %body, "report_result failed");
            Err(HeartbeatError::Server { status, body })
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HeartbeatError {
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("server error (HTTP {status}): {body}")]
    Server { status: u16, body: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_payload_serialization() {
        let payload = HeartbeatPayload {
            agent_id: "a1".to_string(),
            tenant_id: "t1".to_string(),
            hostname: "web-01".to_string(),
            os: "linux".to_string(),
            agent_version: "0.1.0".to_string(),
            tools: vec![],
            labels: serde_json::json!({"env": "prod"}),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("agentId"));
        assert!(json.contains("tenantId"));
        assert!(json.contains("agentVersion"));
    }

    #[test]
    fn test_heartbeat_response_deserialization() {
        let json = r#"{"agentId":"a1","status":"ONLINE"}"#;
        let resp: HeartbeatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.agent_id, "a1");
        assert_eq!(resp.status, "ONLINE");
    }

    #[test]
    fn test_heartbeat_client_creation() {
        let client = HeartbeatClient::new("http://localhost:4200/", "token");
        assert_eq!(client.server_url, "http://localhost:4200");
        assert_eq!(client.token_store().current(), "token");
    }

    #[test]
    fn test_heartbeat_response_parses_rotated_token() {
        let json = r#"{"agentId":"a1","status":"ONLINE","rotatedToken":"new-token-9999"}"#;
        let resp: HeartbeatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.rotated_token.as_deref(), Some("new-token-9999"));
    }

    #[test]
    fn test_heartbeat_response_rotated_token_optional() {
        let json = r#"{"agentId":"a1","status":"ONLINE"}"#;
        let resp: HeartbeatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.rotated_token.is_none());
    }

    #[test]
    fn test_heartbeat_client_applies_rotation() {
        // Simulate the post-response code path: a rotated token on the
        // response swaps the store's current token.
        let client = HeartbeatClient::new("http://localhost:4200", "initial-token-abcd");
        let resp = HeartbeatResponse {
            agent_id: "a1".to_string(),
            status: "ONLINE".to_string(),
            rotated_token: Some("rotated-token-efgh".to_string()),
        };
        // Mirror the logic from send_heartbeat().
        if let Some(new_token) = resp.rotated_token.as_deref() {
            client
                .token_store()
                .maybe_rotate(Some(new_token))
                .expect("valid token");
        }
        assert_eq!(client.token_store().current(), "rotated-token-efgh");
    }

    #[test]
    fn test_backoff_delay_calculation() {
        // Backoff sequence: 2, 4, 8, 16, 32, capped at 60
        assert_eq!(backoff_delay_secs(0), 2); // 2^1 = 2
        assert_eq!(backoff_delay_secs(1), 4); // 2^2 = 4
        assert_eq!(backoff_delay_secs(2), 8); // 2^3 = 8
        assert_eq!(backoff_delay_secs(3), 16); // 2^4 = 16
        assert_eq!(backoff_delay_secs(4), 32); // 2^5 = 32
        assert_eq!(backoff_delay_secs(5), 60); // 2^6 = 64, capped at 60
        assert_eq!(backoff_delay_secs(10), 60); // large value capped at 60
    }
}
