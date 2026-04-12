use serde::{Deserialize, Serialize};
use std::process::Command;

/// Represents a discovered tool on the host system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveredTool {
    pub name: String,
    pub version: Option<String>,
    pub path: String,
}

/// Well-known tools to probe for on the host.
const WELL_KNOWN_TOOLS: &[(&str, &[&str])] = &[
    ("systemctl", &["--version"]),
    ("docker", &["--version"]),
    ("kubectl", &["version", "--client", "--short"]),
    ("curl", &["--version"]),
    ("nginx", &["-v"]),
    ("python3", &["--version"]),
    ("node", &["--version"]),
];

/// Discover available tools on the host system.
pub fn discover_tools() -> Vec<DiscoveredTool> {
    let mut tools = Vec::new();
    for (name, version_args) in WELL_KNOWN_TOOLS {
        if let Some(tool) = probe_tool(name, version_args) {
            tools.push(tool);
        }
    }
    tools
}

fn probe_tool(name: &str, version_args: &[&str]) -> Option<DiscoveredTool> {
    // Check if the tool exists using `which`
    let which_output = Command::new("which").arg(name).output().ok()?;
    if !which_output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&which_output.stdout)
        .trim()
        .to_string();

    // Try to get version
    let version = Command::new(name)
        .args(version_args)
        .output()
        .ok()
        .and_then(|o| {
            let out = if o.status.success() {
                String::from_utf8_lossy(&o.stdout).to_string()
            } else {
                // Some tools (like nginx) output version to stderr
                String::from_utf8_lossy(&o.stderr).to_string()
            };
            let line = out.lines().next()?.trim().to_string();
            if line.is_empty() {
                None
            } else {
                Some(line)
            }
        });

    Some(DiscoveredTool {
        name: name.to_string(),
        version,
        path,
    })
}

/// Collect host metadata (OS, hostname, IP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub hostname: String,
    pub os: String,
    pub ip_address: Option<String>,
}

pub fn collect_host_info() -> HostInfo {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let os = std::env::consts::OS.to_string();

    HostInfo {
        hostname,
        os,
        ip_address: None, // filled by server from connection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_tools_returns_vec() {
        let tools = discover_tools();
        // We can't guarantee which tools are installed, but it should return a valid vec
        let _ = tools.len();
    }

    #[test]
    fn test_collect_host_info() {
        let info = collect_host_info();
        assert!(!info.hostname.is_empty());
        assert!(!info.os.is_empty());
    }

    #[test]
    fn test_discovered_tool_serialize() {
        let tool = DiscoveredTool {
            name: "docker".to_string(),
            version: Some("24.0.0".to_string()),
            path: "/usr/bin/docker".to_string(),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("docker"));
        let deserialized: DiscoveredTool = serde_json::from_str(&json).unwrap();
        assert_eq!(tool, deserialized);
    }
}
