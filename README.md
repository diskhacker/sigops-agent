# SigOps Agent

Lightweight Rust binary for executing SigOps workflows on your infrastructure.

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)

---

## What It Does

The agent runs on your servers (bare metal, VMs, containers) and executes commands dispatched by the SigOps control plane. It **phones home** — your firewall only needs outbound HTTPS to your SigOps server.

Only features that exist in code on `main` today are documented here. Features planned but not shipped are listed under **Roadmap**.

---

## Why Open Source?

The agent runs **inside your private infrastructure** — on your servers, in your network, with access to your services. Open source lets you:

- **Audit the code** — verify it does only what it claims.
- **Verify there are no backdoors** — outbound-only connection; no data exfiltration.
- **Build from source** — compile it yourself if policy requires.
- **Understand the attack surface** — know exactly what the binary does.

---

## Security Model

Current security layers (implemented today):

- **Command whitelist** — only pre-approved commands can execute.
- **Path deny list** — blocks access to sensitive directories.
- **Execution timeout** — hard kill (SIGKILL) after a configurable limit.
- **Token rotation** — API tokens refresh on each heartbeat cycle.
- **Outbound-only** — the agent initiates all connections; no inbound ports are required.
  The optional local `/health` endpoint on `:9100` can be disabled with `--no-health`.

Items like mTLS, namespace/cgroup sandboxing, and ed25519 command-signature verification are on the **Roadmap** — they are not shipped yet. Do not rely on them for threat modeling.

---

## Transport

The agent communicates with the SigOps control plane via **HTTP long-polling** with exponential backoff. It requires outbound HTTPS to your SigOps server URL. WebSocket transport is planned (see Roadmap) but not implemented today.

Flow:

```
┌──────────────────────────┐              ┌──────────────────────────┐
│  Your infrastructure     │              │  SigOps control plane    │
│                          │              │                          │
│  ┌──────────────────┐    │   outbound   │  ┌──────────────────┐   │
│  │  sigops-agent    │───  HTTPS ───────▶│  │  /agent/heartbeat │   │
│  │  (Rust binary)   │                   │  │  /agent/fetch     │   │
│  │                  │◀── responses ─────│  │  /agent/report    │   │
│  │  ● heartbeat     │                   │  └──────────────────┘   │
│  │  ● fetch-next    │                   │                          │
│  │  ● execute       │                   │  Dashboard shows:        │
│  │  ● report        │                   │   ● agent status         │
│  └──────────────────┘    │              │   ● available tools      │
│                          │              │   ● execution results    │
└──────────────────────────┘              └──────────────────────────┘
```

---

## Built-in Tools

- `restart_service` — `systemctl restart <unit>`
- `http_request` — arbitrary HTTP calls
- `notify_slack` — Slack webhook
- `wait` — timed pause
- `condition` — conditional branching

---

## Installation

### From source

```bash
# Prerequisites: Rust 1.75+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/diskhacker/sigops-agent.git
cd sigops-agent
cargo build --release

# Run it
./target/release/sigops-agent \
  --server-url https://sigops.example.com \
  --api-key ag_xxx
```

Pre-built installers and container images are planned (see Roadmap) but not published today.

### Configuration

| Env var | CLI flag | Description | Default |
|---|---|---|---|
| `SIGOPS_SERVER_URL` | `--server-url` | Control plane URL | required |
| `SIGOPS_API_KEY` | `--api-key` | Agent API key | required |
| `SIGOPS_POLL_INTERVAL` | `--poll-interval` | Heartbeat interval (seconds) | `30` |
| `SIGOPS_HEALTH_PORT` | `--health-port` | Local health endpoint port | `9100` |
| `SIGOPS_NO_HEALTH` | `--no-health` | Disable the local health endpoint | `false` |
| `SIGOPS_LOG_LEVEL` | `--log-level` | `debug` \| `info` \| `warn` \| `error` | `info` |

---

## Requirements

- **Linux** (x86_64, aarch64), **macOS** (x86_64, aarch64), or **Windows** (x86_64)
- **Outbound HTTPS** connectivity to your SigOps server

---

## Development

```bash
# Run in debug mode
cargo run -- --server-url http://localhost:4200 --api-key dev-token

# Run tests
cargo test

# Build release binary
cargo build --release

# Cross-compile for Linux musl (from macOS)
cargo install cross
cross build --release --target x86_64-unknown-linux-musl
```

CI runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --all-targets`, and `cargo build --release` on Ubuntu, macOS, and Windows for every push/PR to `main`.

---

## Running as a Service

### systemd (Linux)

```ini
# /etc/systemd/system/sigops-agent.service
[Unit]
Description=SigOps Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
Environment=SIGOPS_SERVER_URL=https://sigops.example.com
Environment=SIGOPS_API_KEY=ag_xxx
ExecStart=/usr/local/bin/sigops-agent
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

The systemd unit file above is an example — a packaged unit is part of the installer work on the Roadmap.

---

## Roadmap

Planned but not yet shipped:

- [ ] **WebSocket transport** (replace HTTP polling for lower latency)
- [ ] **mTLS** client certificates
- [ ] **Command signature verification** (ed25519)
- [ ] **Namespace / cgroup sandboxing** (Linux)
- [ ] **Auto-update** with version pinning
- [ ] **Pre-built installers** (`.deb`, `.rpm`, Homebrew, MSI)
- [ ] **Container image** on GHCR
- [ ] **Code-signed releases** (Authenticode + macOS notarization)
- [ ] **Additional executor adapters** (`pm2`, `kubectl`)
- [ ] **`cargo audit` / SBOM** in CI

---

## License

MIT — ClusterAssets Innovation Pvt. Ltd.
