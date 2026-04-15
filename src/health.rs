#![allow(dead_code)]
//! Minimal HTTP health endpoint for the SigOps Agent.
//!
//! Runs on a configurable port (default 9100) and responds
//! to `GET /health` with a JSON status object.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tracing::{debug, error, info};

/// Starts the health HTTP server. Returns when `shutdown` is set to true.
pub async fn serve_health(port: u16, shutdown: Arc<AtomicBool>, agent_id: String) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            info!(port = port, "health endpoint listening");
            l
        }
        Err(e) => {
            error!(error = %e, port = port, "failed to bind health endpoint");
            return;
        }
    };

    loop {
        if shutdown.load(Ordering::Relaxed) {
            debug!("health server shutting down");
            break;
        }

        let accept =
            tokio::time::timeout(std::time::Duration::from_secs(1), listener.accept()).await;

        let (mut stream, _addr) = match accept {
            Ok(Ok(pair)) => pair,
            Ok(Err(e)) => {
                error!(error = %e, "accept error");
                continue;
            }
            Err(_) => continue, // timeout — check shutdown flag
        };

        let body = serde_json::json!({
            "status": "ok",
            "agent": "sigops-agent",
            "version": env!("CARGO_PKG_VERSION"),
            "agentId": agent_id,
        });
        let body_str = serde_json::to_string(&body).unwrap_or_default();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_str.len(),
            body_str,
        );

        if let Err(e) = stream.write_all(response.as_bytes()).await {
            debug!(error = %e, "failed to write health response");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_server_starts_and_stops() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let handle = tokio::spawn(async move {
            serve_health(0, shutdown_clone, "test-agent-id".to_string()).await;
        });

        // Give it a moment to bind, then signal shutdown
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        shutdown.store(true, Ordering::Relaxed);

        // Should complete within 2 seconds (timeout loop)
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), handle).await;
        assert!(result.is_ok(), "health server should shut down");
    }

    #[test]
    fn test_health_response_format() {
        let agent_id = "test-agent-123";
        let body = serde_json::json!({
            "status": "ok",
            "agent": "sigops-agent",
            "version": env!("CARGO_PKG_VERSION"),
            "agentId": agent_id,
        });
        let s = serde_json::to_string(&body).unwrap();
        assert!(s.contains("\"status\":\"ok\""));
        assert!(s.contains("sigops-agent"));
        assert!(s.contains("\"agentId\":\"test-agent-123\""));
    }
}
