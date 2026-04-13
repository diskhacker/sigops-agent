# SigOps Agent

> Lightweight Rust binary that executes automation on your infrastructure.

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)

The SigOps Agent is a ~5MB statically-linked Rust binary that runs on your servers, VMs, or containers. It connects to the SigOps platform via **outbound-only WebSocket** (no inbound ports needed), discovers available tools, executes automation steps, and reports results — all with an 8-layer security model.

---

## Quick Reference

| | |
|---|---|
| **Language** | Rust (2021 edition) |
| **Binary Size** | ~5MB (statically linked, musl) |
| **Visibility** | PUBLIC (MIT License) |
| **Connection** | Outbound-only WSS (no open ports on your infra) |
| **Security** | 8-layer model (mTLS, token rotation, sandboxed execution) |
| **Platforms** | Linux x86_64, aarch64 · macOS x86_64, aarch64 · Windows x86_64 |

---

## Why Open Source?

The agent runs **inside your private infrastructure** — on your servers, in your network, with access to your services. You need to:

- **Audit the code** — verify it does only what it claims
- **Verify no backdoors** — outbound-only connection, no data exfiltration
- **Build from source** — compile it yourself if your security policy requires it
- **Understand the attack surface** — know exactly what the binary does

Closed-source agents in your infrastructure is a trust problem. Open source solves it.

---

## How It Works

```
┌─────────────────────────┐         ┌─────────────────────────┐
│  Your Infrastructure    │         │  SigOps Platform        │
│                         │         │                         │
│  ┌─────────────────┐   │  WSS    │  ┌─────────────────┐   │
│  │  SigOps Agent    │───────────→│  │  Agent Gateway   │   │
│  │  (Rust binary)   │   │ outbound│  │  (WebSocket)     │   │
│  │                  │←──────────│  │                  │   │
│  │  ● Heartbeat     │   │  only  │  │  ● Route commands│   │
│  │  ● Tool discover │   │        │  │  ● Track status  │   │
│  │  ● Execute steps │   │        │  │  ● Collect results│  │
│  │  ● Report results│   │        │  └─────────────────┘   │
│  └─────────────────┘   │         │                         │
│                         │         │  Dashboard shows:       │
│  Services it can reach: │         │  ● Agent status         │
│  ● nginx, postgres      │         │  ● Available tools      │
│  ● docker, systemctl    │         │  ● Execution results    │
│  ● kubernetes, pm2      │         │                         │
└─────────────────────────┘         └─────────────────────────┘
```

**Key principle:** The agent initiates ALL connections outbound. Your firewall rules stay unchanged. No ports to open, no ingress rules to add.

---

## Install

### Pre-built binaries

```bash
# Linux x86_64
curl -fsSL https://get.sigops.dev/agent | sh

# Or download directly
wget https://github.com/<your-org>/sigops-agent/releases/latest/download/sigops-agent-linux-x86_64
chmod +x sigops-agent-linux-x86_64
sudo mv sigops-agent-linux-x86_64 /usr/local/bin/sigops-agent
```

### Build from source

```bash
# Prerequisites: Rust 1.75+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/<your-org>/sigops-agent.git
cd sigops-agent
cargo build --release

# Binary at target/release/sigops-agent (~5MB)
```

### Docker

```bash
docker run -d \
  --name sigops-agent \
  -e SIGOPS_URL=wss://your-sigops-instance.com/agent/ws \
  -e SIGOPS_TOKEN=<agent-token-from-dashboard> \
  ghcr.io/<your-org>/sigops-agent:latest
```

---

## Configuration

```bash
# /etc/sigops/agent.toml (or environment variables)

[connection]
url = "wss://your-sigops-instance.com/agent/ws"    # SIGOPS_URL
token = "<agent-token>"                              # SIGOPS_TOKEN
reconnect_interval_sec = 5
heartbeat_interval_sec = 30

[identity]
hostname = "web-01"                                  # auto-detected if empty
labels = { env = "production", region = "us-east", tier = "web" }

[security]
allowed_commands = ["systemctl", "docker", "pm2"]    # whitelist
denied_paths = ["/etc/shadow", "/root"]              # never access these
max_execution_time_sec = 300                          # 5 min hard limit
sandbox = true                                        # enable sandboxed execution

[logging]
level = "info"                                        # debug, info, warn, error
file = "/var/log/sigops-agent.log"
max_size_mb = 100
```

---

## Repository Structure

```
sigops-agent/
├── Cargo.toml                # Rust project config
├── Cargo.lock
├── LICENSE                   # MIT
├── README.md                 # This file
├── SECURITY.md               # Vulnerability reporting
├── CONTRIBUTING.md
│
├── src/
│   ├── main.rs               # Entry point
│   ├── config.rs             # Configuration parsing (TOML + env)
│   ├── ws/
│   │   ├── client.rs         # WebSocket client (tokio-tungstenite)
│   │   ├── reconnect.rs      # Auto-reconnect with backoff
│   │   └── messages.rs       # Message types (command, result, heartbeat)
│   ├── tools/
│   │   ├── executor.rs       # Tool execution engine
│   │   ├── registry.rs       # Available tools on this host
│   │   └── builtin.rs        # Built-in tools (restart, http, wait)
│   ├── discovery/
│   │   ├── service.rs        # Auto-discover running services
│   │   ├── tool.rs           # Auto-discover available tools
│   │   └── system.rs         # OS, CPU, memory, disk info
│   ├── security/
│   │   ├── sandbox.rs        # Sandboxed execution
│   │   ├── whitelist.rs      # Command whitelist enforcement
│   │   ├── token.rs          # Token rotation
│   │   └── tls.rs            # mTLS certificate management
│   └── heartbeat/
│       └── reporter.rs       # Health + tool list reporting
│
├── tests/
│   ├── integration/
│   └── unit/
│
└── .github/
    └── workflows/
        ├── build.yml          # CI: build + test all platforms
        └── release.yml        # CD: build binaries + publish
```

---

## 8-Layer Security Model

| Layer | Protection |
|-------|-----------|
| 1. Outbound-only | Agent initiates all connections — no open ports |
| 2. mTLS | Mutual TLS between agent and platform |
| 3. Token rotation | Agent tokens auto-rotate every 24h |
| 4. Command whitelist | Only whitelisted commands can execute |
| 5. Path deny-list | Sensitive paths (/etc/shadow, /root) blocked |
| 6. Execution timeout | Hard 5-minute limit per step |
| 7. Sandboxed execution | Commands run in restricted context |
| 8. Signed commands | Platform signs commands, agent verifies signature |

---

## Development

```bash
# Run in development mode
cargo run -- --config dev.toml

# Run tests
cargo test

# Build release binary
cargo build --release

# Cross-compile for Linux (from macOS)
cargo install cross
cross build --release --target x86_64-unknown-linux-musl
```

---

## Systemd Service

```ini
# /etc/systemd/system/sigops-agent.service
[Unit]
Description=SigOps Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/sigops-agent --config /etc/sigops/agent.toml
Restart=always
RestartSec=5
User=sigops
Group=sigops

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable sigops-agent
sudo systemctl start sigops-agent
sudo journalctl -u sigops-agent -f
```

---

## License

MIT — see [LICENSE](LICENSE)

---
