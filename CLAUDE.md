# SigOps Agent — Rust Binary

> Lightweight outbound-only agent for infrastructure execution. ~5MB. 8-layer security.

## Product Identity
| Key | Value |
|-----|-------|
| Repo | `sigops-agent` |
| Visibility | PUBLIC (MIT License) |
| Language | Rust 2021 edition (1.75+) |
| Binary Size | ~5MB statically linked (musl) |
| Connection | Outbound-only WSS to sigops Agent Gateway |
| Platforms | linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, windows-x86_64 |

## PROTOCOLS — MANDATORY
```
AUDIT → REVIEW → CONFIRM → REUSE → IMPLEMENT
Feature = Code + Tests (>90% coverage via cargo test)
Session: /docs/session/ | Memory: /docs/memory/memory.md
```

## Architecture Reference
`docs/architecture/SigOps-Architecture-v1.2.0.pdf` — Section 6 (Agent), Section 18 (Security Model)

## What This Binary Does

1. **Connects** to SigOps platform via outbound WSS (no inbound ports)
2. **Heartbeats** every 30s (hostname, OS, tools, labels, health)
3. **Discovers** services and tools on the host (systemctl, docker, pm2, k8s)
4. **Receives** execution commands from platform
5. **Executes** tool steps in sandboxed context
6. **Reports** step results (output, error, duration) back via WSS

## Repo Structure

```
sigops-agent/
├── Cargo.toml
├── Cargo.lock
├── CLAUDE.md                    # This file
├── README.md
├── LICENSE                      # MIT
├── SECURITY.md                  # Vulnerability reporting
├── CONTRIBUTING.md
├── rust-toolchain.toml          # Pin Rust version
├── .github/
│   └── workflows/
│       ├── ci.yml               # Build + test all platforms
│       └── release.yml          # Binary builds + GitHub Release
│
├── src/
│   ├── main.rs                  # Entry: config → connect → loop
│   ├── config.rs                # TOML + env config parsing
│   │
│   ├── ws/                      # WebSocket client
│   │   ├── mod.rs
│   │   ├── client.rs            # tokio-tungstenite WSS client
│   │   ├── reconnect.rs         # Exponential backoff reconnect
│   │   └── messages.rs          # Message types (serde)
│   │       # Inbound: ExecuteStep, CancelExecution, Ping
│   │       # Outbound: StepResult, Heartbeat, ToolList, Pong
│   │
│   ├── tools/                   # Tool execution
│   │   ├── mod.rs
│   │   ├── executor.rs          # Run tool command, capture stdout/stderr
│   │   ├── registry.rs          # Track available tools on this host
│   │   └── builtin/             # Built-in tool adapters
│   │       ├── restart.rs       # systemctl/docker/pm2 restart
│   │       ├── http.rs          # HTTP request tool
│   │       └── wait.rs          # Sleep N seconds
│   │
│   ├── discovery/               # Auto-discovery
│   │   ├── mod.rs
│   │   ├── service.rs           # Discover running services
│   │   ├── tool.rs              # Discover available tools
│   │   └── system.rs            # OS, CPU, memory, disk
│   │
│   ├── security/                # 8-layer security
│   │   ├── mod.rs
│   │   ├── sandbox.rs           # Restricted execution context
│   │   ├── whitelist.rs         # Command whitelist enforcement
│   │   ├── token.rs             # Token storage + rotation
│   │   └── tls.rs               # mTLS certificate management
│   │
│   └── heartbeat/               # Health reporting
│       ├── mod.rs
│       └── reporter.rs          # Periodic heartbeat sender
│
├── tests/
│   ├── unit/
│   │   ├── config_test.rs
│   │   ├── message_test.rs
│   │   ├── whitelist_test.rs
│   │   ├── executor_test.rs
│   │   └── discovery_test.rs
│   └── integration/
│       ├── ws_connect_test.rs   # Mock WSS server → agent connects
│       ├── execute_test.rs      # Mock command → agent runs → result
│       └── heartbeat_test.rs    # Agent sends heartbeat on schedule
│
└── docs/
    ├── architecture/
    ├── session/
    └── memory/memory.md
```

## Crate Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.24", features = ["native-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
sha2 = "0.10"                    # Command signature verification
ring = "0.17"                    # mTLS + token crypto
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }  # CLI args
```

## WebSocket Message Protocol

```rust
// Agent → Platform (outbound)
enum AgentMessage {
    Heartbeat { hostname, os, tools: Vec<ToolInfo>, labels, timestamp },
    StepResult { execution_id, step_index, status, output, error, duration_ms },
    ToolList { tools: Vec<ToolInfo> },
    Pong { timestamp },
}

// Platform → Agent (inbound)
enum PlatformMessage {
    ExecuteStep { execution_id, step_index, tool_name, input: Value, timeout_ms },
    CancelExecution { execution_id },
    Ping { timestamp },
    UpdateConfig { config: Value },
}
```

## 8-Layer Security Implementation

```
Layer 1 — Outbound-only:    Agent initiates WSS. No listen ports.
Layer 2 — mTLS:             src/security/tls.rs — mutual TLS cert exchange
Layer 3 — Token rotation:   src/security/token.rs — auto-rotate every 24h
Layer 4 — Command whitelist: src/security/whitelist.rs — only allowed commands
Layer 5 — Path deny-list:   Config: denied_paths = ["/etc/shadow", "/root"]
Layer 6 — Execution timeout: executor.rs — tokio::time::timeout per step
Layer 7 — Sandbox:          src/security/sandbox.rs — restricted env, no network
Layer 8 — Signed commands:  Verify platform signature on every ExecuteStep
```

## Build & Test

```bash
# Development
cargo run -- --config dev.toml

# Tests (>90% coverage)
cargo test
cargo tarpaulin --out html  # Coverage report

# Release build (statically linked)
cargo build --release --target x86_64-unknown-linux-musl

# Cross-compile all platforms
cargo install cross
cross build --release --target x86_64-unknown-linux-musl
cross build --release --target aarch64-unknown-linux-musl
cross build --release --target x86_64-apple-darwin
cross build --release --target aarch64-apple-darwin
cross build --release --target x86_64-pc-windows-msvc
```

## Module Build Order

```
1. config.rs          — Parse TOML + env, validate
2. ws/messages.rs     — Define message types (serde)
3. ws/client.rs       — Connect to WSS, send/receive
4. ws/reconnect.rs    — Auto-reconnect with backoff
5. heartbeat/         — Periodic health reporter
6. discovery/         — Service + tool + system info
7. tools/executor.rs  — Run commands, capture output
8. tools/builtin/     — restart, http, wait adapters
9. security/          — All 8 layers
10. main.rs           — Wire everything together
```

## Config File (agent.toml)

```toml
[connection]
url = "wss://your-sigops.com/agent/ws"
token = "agent-token-from-dashboard"
reconnect_interval_sec = 5
heartbeat_interval_sec = 30

[identity]
hostname = ""                        # auto-detect if empty
labels = { env = "production", region = "us-east" }

[security]
allowed_commands = ["systemctl", "docker", "pm2", "kubectl"]
denied_paths = ["/etc/shadow", "/root", "/home/*/.ssh"]
max_execution_time_sec = 300
sandbox = true

[logging]
level = "info"
file = "/var/log/sigops-agent.log"
```

## HARD RULES
1. OUTBOUND ONLY — agent NEVER listens on any port
2. No data exfiltration — agent sends ONLY step results and heartbeats
3. No shell injection — all commands go through whitelist + parameterized execution
4. Every step has a hard timeout — no infinite hangs
5. Token never stored in plaintext — encrypted at rest
6. All platform commands must be signature-verified before execution
