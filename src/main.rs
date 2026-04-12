mod config;
mod discovery;
mod executor;
mod heartbeat;

use clap::Parser;
use std::time::Duration;
use tracing::{error, info};

use config::Config;
use discovery::{collect_host_info, discover_tools};
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

    // Start heartbeat loop
    let hb_client = HeartbeatClient::new(&config.server_url, &config.api_token);
    let interval = Duration::from_secs(config.heartbeat_interval);

    loop {
        match hb_client
            .send_heartbeat(&agent_id, &config.tenant_id, &host_info, &tools)
            .await
        {
            Ok(resp) => {
                info!(status = %resp.status, "heartbeat acknowledged");
            }
            Err(e) => {
                error!(error = %e, "heartbeat failed — will retry");
            }
        }
        tokio::time::sleep(interval).await;
    }
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
