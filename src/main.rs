mod config;
mod discovery;
mod executor;
mod health;
mod heartbeat;
mod security;

use clap::Parser;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use config::Config;
use discovery::{collect_host_info, discover_tools};
use health::serve_health;
use heartbeat::HeartbeatClient;

#[tokio::main]
async fn main() {
    let config = Config::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.clone().into()),
        )
        .init();

    let agent_id = config.agent_id();
    info!(
        agent_id = %agent_id,
        server = %config.server_url,
        tenant = %config.tenant_id,
        interval = config.heartbeat_interval,
        "SigOps Agent starting"
    );

    // Discover host info and tools
    let host_info = collect_host_info();
    let tools = discover_tools();
    info!(
        hostname = %host_info.hostname,
        os = %host_info.os,
        tools_found = tools.len(),
        "Host discovery complete"
    );

    // Shared shutdown flag
    let shutdown = Arc::new(AtomicBool::new(false));

    // Start health endpoint
    let health_shutdown = shutdown.clone();
    let health_agent_id = agent_id.clone();
    let health_handle = tokio::spawn(async move {
        serve_health(9100, health_shutdown, health_agent_id).await;
    });

    // Start heartbeat loop
    let hb_client = HeartbeatClient::new(&config.server_url, &config.api_token);
    let interval = Duration::from_secs(config.heartbeat_interval);

    let hb_shutdown = shutdown.clone();
    let hb_handle = tokio::spawn(async move {
        while !hb_shutdown.load(Ordering::Relaxed) {
            match hb_client
                .send_heartbeat_with_retry(&agent_id, &config.tenant_id, &host_info, &tools)
                .await
            {
                Ok(resp) => {
                    info!(status = %resp.status, "heartbeat acknowledged");
                }
                Err(e) => {
                    error!(error = %e, "heartbeat failed after retries — will try next cycle");
                }
            }
            tokio::time::sleep(interval).await;
        }
        info!("heartbeat loop stopped");
    });

    // Wait for shutdown signal (SIGTERM or SIGINT)
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            warn!("received SIGINT, shutting down gracefully");
        }
        _ = async {
            #[cfg(unix)]
            {
                let mut sigterm = tokio::signal::unix::signal(
                    tokio::signal::unix::SignalKind::terminate(),
                ).expect("failed to register SIGTERM handler");
                sigterm.recv().await;
            }
            #[cfg(not(unix))]
            {
                // On non-unix platforms, just wait forever (ctrl_c will fire)
                std::future::pending::<()>().await;
            }
        } => {
            warn!("received SIGTERM, shutting down gracefully");
        }
    }

    // Signal all tasks to stop
    shutdown.store(true, Ordering::Relaxed);
    info!("waiting for tasks to finish...");

    // Give tasks time to finish
    let _ = tokio::time::timeout(Duration::from_secs(5), async {
        let _ = hb_handle.await;
        let _ = health_handle.await;
    })
    .await;

    info!("SigOps Agent stopped");
}

#[cfg(test)]
mod tests {
    use super::config::Config;

    #[test]
    fn test_agent_id_generation() {
        let config = Config {
            server_url: "http://localhost:4200".to_string(),
            agent_id: None,
            tenant_id: "t1".to_string(),
            api_token: "token".to_string(),
            heartbeat_interval: 30,
            log_level: "info".to_string(),
        };
        let id1 = config.agent_id();
        let id2 = config.agent_id();
        // Each call generates a new UUID when agent_id is None
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID format
    }
}
