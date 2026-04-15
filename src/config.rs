use clap::Parser;

/// SigOps Agent — infrastructure execution agent
#[derive(Parser, Debug, Clone)]
#[command(name = "sigops-agent", version, about)]
pub struct Config {
    /// SigOps server URL
    #[arg(
        long,
        env = "SIGOPS_SERVER_URL",
        default_value = "http://localhost:4200"
    )]
    pub server_url: String,

    /// Agent ID (auto-generated if not set)
    #[arg(long, env = "AGENT_ID")]
    pub agent_id: Option<String>,

    /// Tenant ID
    #[arg(long, env = "TENANT_ID", default_value = "default")]
    pub tenant_id: String,

    /// API token for authenticating with the server
    #[arg(long, env = "API_TOKEN", default_value = "")]
    pub api_token: String,

    /// Heartbeat interval in seconds
    #[arg(long, env = "HEARTBEAT_INTERVAL", default_value_t = 30)]
    pub heartbeat_interval: u64,

    /// Log level
    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    pub log_level: String,
}

impl Config {
    pub fn agent_id(&self) -> String {
        self.agent_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config {
            server_url: "http://localhost:4200".to_string(),
            agent_id: None,
            tenant_id: "t1".to_string(),
            api_token: "tok".to_string(),
            heartbeat_interval: 30,
            log_level: "info".to_string(),
        };
        assert!(!config.agent_id().is_empty());
        assert_eq!(config.server_url, "http://localhost:4200");
    }

    #[test]
    fn test_explicit_agent_id() {
        let config = Config {
            server_url: "http://localhost:4200".to_string(),
            agent_id: Some("my-agent".to_string()),
            tenant_id: "t1".to_string(),
            api_token: "tok".to_string(),
            heartbeat_interval: 30,
            log_level: "info".to_string(),
        };
        assert_eq!(config.agent_id(), "my-agent");
    }
}
