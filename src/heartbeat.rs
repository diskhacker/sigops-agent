use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::discovery::{DiscoveredTool, HostInfo};

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
}

pub struct HeartbeatClient {
    client: Client,
    server_url: String,
    api_token: String,
}

impl HeartbeatClient {
    pub fn new(server_url: &str, api_token: &str) -> Self {
        Self {
            client: Client::new(),
            server_url: server_url.trim_end_matches('/').to_string(),
            api_token: api_token.to_string(),
        }
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
            .bearer_auth(&self.api_token)
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
        error!("heartbeat failed after {} retries, continuing to next cycle", MAX_CONSECUTIVE_FAILURES);
        Err(last_err.unwrap())
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
    }

    #[test]
    fn test_backoff_delay_calculation() {
        // Backoff sequence: 2, 4, 8, 16, 32, capped at 60
        assert_eq!(backoff_delay_secs(0), 2);  // 2^1 = 2
        assert_eq!(backoff_delay_secs(1), 4);  // 2^2 = 4
        assert_eq!(backoff_delay_secs(2), 8);  // 2^3 = 8
        assert_eq!(backoff_delay_secs(3), 16); // 2^4 = 16
        assert_eq!(backoff_delay_secs(4), 32); // 2^5 = 32
        assert_eq!(backoff_delay_secs(5), 60); // 2^6 = 64, capped at 60
        assert_eq!(backoff_delay_secs(10), 60); // large value capped at 60
    }
}
