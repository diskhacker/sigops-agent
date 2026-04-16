# SigOps Agent — Rust Binary

> Lightweight outbound-only agent for infrastructure execution.

## Recent Changes (Sprint 1.5 — 2026-04-17)

**README rewritten to match shipped code.** Do NOT claim features from this CLAUDE.md spec that are not yet implemented in `src/`.

Current reality:
- Transport: **HTTP long-polling**, not WebSocket. `tokio-tungstenite` is in `Cargo.toml` but not on the heartbeat path.
- Security layers implemented: command whitelist, path deny list, execution timeout (SIGKILL), token rotation per heartbeat, outbound-only HTTP.
- Security layers **NOT implemented** (despite legacy 8-layer docs below): mTLS (Layer 2), sandbox (Layer 7), signed commands (Layer 8). `sha2`/`ring` are not in `Cargo.toml`.
- Module layout is **flat**: `config.rs`, `discovery.rs`, `heartbeat.rs`, `executor.rs`, `security.rs`, `health.rs`, `main.rs`, `lib.rs`. The nested `ws/`, `tools/`, `discovery/`, `security/` tree below is aspirational, not current.
- No release has been cut. `release.yml` has never fired. The `get.sigops.dev/agent` installer URL in legacy docs is aspirational.

When landing a feature from the spec below, update this note and the public README in the same PR.

See `docs/session/2026-04-17-sprint-1.5.md` and `docs/memory/2026-04-17-sprint-1.5.md`.

---

## Product Identity
| Key | Value |
|-----|-------|
| Repo | `sigops-agent` |
| Visibility | PUBLIC (MIT License) |
| Language | Rust 2021 edition (1.75+) |
| Binary Size | ~5MB statically linked (musl) |
| Connection | **HTTP long-polling today**; WSS planned |
| Platforms | linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, windows-x86_64 |

## PROTOCOLS — MANDATORY
```
AUDIT → REVIEW → CONFIRM → REUSE → IMPLEMENT
Feature = Code + Tests (>90% coverage via cargo test)
Session: /docs/session/ | Memory: /docs/memory/memory.md
```

## Architecture Reference
`docs/architecture/SigOps-Architecture-v1.2.0.pdf` — Section 6 (Agent), Section 18 (Security Model)

## What This Binary Does (today)

1. **Connects** to SigOps platform via outbound HTTPS (polling `/agent/fetch`).
2. **Heartbeats** every 30s with host identity and tool list.
3. **Discovers** services and tools on the host.
4. **Receives** command descriptors from the platform.
5. **Executes** commands through the whitelist + path-deny layers with a hard timeout.
6. **Reports** step results back via HTTP POST.

## Built-in Tools (5)
`restart_service`, `http_request`, `notify_slack`, `wait`, `condition`.

## HARD RULES
1. OUTBOUND ONLY — agent does not listen on any port other than the optional local `/health` (disablable).
2. No data exfiltration — agent sends only step results and heartbeats.
3. No shell injection — all commands go through the whitelist.
4. Every step has a hard timeout — no infinite hangs.
5. Token never stored in plaintext — `TokenStore` persists atomically to disk.
6. **README claims must match shipped code.** If you haven't written it, you can't advertise it.

---

## Legacy 8-Layer Spec (aspirational, not yet shipped)

> The full 8-layer model, WebSocket message protocol, and nested module tree that used to live here have been moved to Roadmap items. See `README.md` § Roadmap for the canonical list and priority order. When a layer lands, bring its spec back into this CLAUDE.md in the same PR.
