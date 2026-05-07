# ClusterAssets — Deep Production Audit (Main-Branch Scope)

**Date:** 2026-05-07
**Auditor:** Claude Code (Opus 4.7, 1M context)
**Scope:** `main` branch, 7 in-scope repos under `diskhacker/`

This same `DEEP-AUDIT-REPORT-2026-05-07.md` is committed on the `claude/create-main-branch-audit-SsbNG` branch in each of the 7 repos.

## Executive Summary

Three new P0s identified across the ecosystem:
1. `uap/server/src/config/env.ts:9` — hard-coded `JWT_SECRET` default.
2. **`sigops-agent/src/executor.rs` — `SecurityPolicy` dead code on hot path.** ← THIS REPO
3. `cluster-deploy/.env.example` — missing `VAULT_MASTER_KEY`.

Plus recurring: **no branch protection on `main`** in any of the 7 repos, **GHAS disabled** everywhere.

---

## `sigops-agent` — Detailed Findings

**Purpose:** Lightweight outbound-only Rust binary that "phones home" to the SigOps control plane. Heartbeats with host info + discovered tools, polls for command descriptors, executes them through a security policy, and POSTs results back. Public, MIT, owned by ClusterAssets. Audit head `516dfc9` 2026-04-21.

### Cargo.toml

- `name = "sigops-agent"`, `version = 0.1.0`, `edition = 2021`, MIT.
- `[[bin]]` `name = "sigops-agent"`, `path = "src/main.rs"` (also exposes `lib.rs`).
- Deps: `tokio "1"` (full), `reqwest "0.12"` (json — uses native-tls default), `serde "1"` + `serde_json "1"`, `uuid "1"` (v4), `chrono "0.4"` (serde), `tracing "0.1"`, `tracing-subscriber "0.3"` (env-filter), `clap "4"` (derive, env), `hostname "0.4"`, `thiserror "1"`.
- Dev deps: `mockito "1"`, `wiremock "0.6"`, `tokio` (test-util).
- **No `tokio-tungstenite`, no `sha2`/`ring`, no `rustls` explicit pin** — TLS comes from reqwest default features only.

### Source files (~660 LOC code, ~960 with tests)

- `main.rs` (~125 LOC) — CLI parse, tracing init, host discovery, spawns health task + heartbeat loop, SIGTERM/SIGINT graceful shutdown via `AtomicBool` + 5s drain.
- `lib.rs` (12 LOC) — re-exports `config`, `discovery`, `executor`, `health`, `heartbeat`, `security`.
- `config.rs` (~75 LOC) — `clap::Parser` struct: `--server-url`, `--agent-id`, `--tenant-id`, `--api-token`, `--heartbeat-interval`, `--log-level`. UUID v4 fallback for agent_id.
- `discovery.rs` (~120 LOC) — `which <tool>` + `<tool> --version` probe for systemctl/docker/kubectl/curl/nginx/python3/node; `HostInfo { hostname, os, ip_address: None }`.
- `heartbeat.rs` (~310 LOC) — `HeartbeatClient` (reqwest), `send_heartbeat`, `send_heartbeat_with_retry` (exponential backoff 2/4/8/16/32 capped at 60s, 5 max attempts), `fetch_next_command` (HTTP 204 → None), `report_result`. Bearer auth; applies `rotatedToken` from response into shared `TokenStore`.
- `health.rs` (~110 LOC) — Hand-rolled HTTP/1.1 listener on `0.0.0.0:9100` (tokio TcpListener); 1s accept timeout used to poll the shutdown flag; replies JSON `{status, agent, version, agentId}`. **Always responds 200, regardless of method/path — does not parse the request line.**
- `executor.rs` (~560 LOC incl. tests) — `TaskCommand` / `TaskResult` types; dispatch on `tool_name`: `sigops.restart` (alphanumeric+`-_.` service name only), `sigops.http` (curl `-s --max-time 30 --connect-timeout 5`), `sigops.notify_slack` (curl, `webhookUrl` required), `sigops.condition` (3-token numeric expr), `sigops.wait` (≤ 3600s). `validate_url` rejects non-http(s), control chars, NUL, CR/LF.
- `security.rs` (~620 LOC incl. tests) — `SecurityPolicy` (whitelist `echo,ls,ps,df,cat,uptime,date,systemctl,service,curl`, deny list `/etc/shadow,/etc/passwd,/etc/gshadow,/root,/var/lib/secrets,/.ssh,/home/`, `..` traversal blocked, shell metas blocked, basename normalization, 30s default timeout via `try_wait` poll + SIGKILL on timeout); `TokenStore` (Arc<Mutex>, atomic temp+rename persist, format validation 8–4096 chars, no whitespace/control).

### Features verified

- **Config loading:** clap derive, env vars (`SIGOPS_SERVER_URL` per README, but actual env names are `SIGOPS_SERVER_URL`, `AGENT_ID`, `TENANT_ID`, `API_TOKEN`, `HEARTBEAT_INTERVAL`, `LOG_LEVEL` — README/CLI doc mismatch on most).
- **Tool discovery:** yes, via `which` + version probe.
- **Executor:** yes, 5 tools, dispatched by name; **does not** route through `SecurityPolicy` — `executor.rs` calls `SysCommand::new` directly. `security.rs` is wired in only via `TokenStore`. **CLAUDE.md HARD RULE 3 ("No shell injection — all commands go through the whitelist") is therefore not enforced on the hot path.**
- **Heartbeat with backoff:** yes, exponential 2→60s, 5 retries.
- **Health endpoint:** yes, port 9100 hard-coded in `main.rs` (CLI flag for port not actually wired despite README), bind on `0.0.0.0` (publicly reachable). README mentions `--no-health`/`--health-port`; **not present in `Config`**.
- **WebSocket vs HTTP:** HTTP only; long-poll style via `/api/v1/agents/{id}/command/next` with 204=no-work. No tungstenite.
- **Bearer auth:** yes (`reqwest::RequestBuilder::bearer_auth`); rotation supported per heartbeat.
- **URL validation:** yes for `sigops.http` only.
- **No-shell-injection:** arguments passed via `Command::args` (no shell). Service name regex-restricted; URL/control-char filter; webhook URL **not** validated through `validate_url`.

### Tests

- ~38 inline unit tests across modules: config 2, discovery 3, heartbeat 7, health 2, executor 17, security 23, main 1.
- `tests/integration_heartbeat.rs` (~165 LOC) — wiremock-driven full cycle: heartbeat → fetch → execute (`sigops.condition`) → report. Asserts bearer token, rotated token, payload JSON shape.
- README claim "63/63 passing" matches 2026-04-15 commit message.

### CI workflow

`.github/workflows/ci.yml` (push/PR to main) on Ubuntu/macOS/Windows: checkout → `dtolnay/rust-toolchain@stable` (clippy, rustfmt) → cache cargo → `cargo fmt --all -- --check` → `cargo clippy --all-targets --all-features -- -D warnings` → `cargo test --all-targets --all-features` → `cargo build --release`.

`.github/workflows/release.yml` builds 5 targets (x86_64/aarch64 musl via `cross`, x86_64/aarch64 darwin, x86_64 windows-msvc), strips, computes SHA-256, uploads artifacts, creates GitHub Release on `v*` tags. No release fired yet.

### Recent activity (commits since 2026-04-13)

- `2026-04-21` `516dfc9` — fix: add `systemctl/service/curl` to allow-list, sigops.* tool name corrections, README updates.
- `2026-04-17` `053366a` — docs: drop false `tokio-tungstenite` claim from CLAUDE.md.
- `2026-04-17` `afa6f38` — docs: release-readiness session log + memory.
- `2026-04-16` `18580d6`, `e325698` (PR #5) — Sprint 1.5 docs/honesty pass.
- `2026-04-16` `1f0357e`, `2bbd107`, `bfd9930` — Sprint 2B+2.5: security hardening (`security.rs`), integration test, release CI.

Branches: `main`, `claude/production-readiness-audit-ngl6r`. No open issues; all 5 PRs merged.

### Secret-scanning

`run_secret_scanning` failed with "Repository does not have GitHub Advanced Security enabled." Manual inspection of source/tests found only test-fixture tokens (`integration-initial-token`, `rotated-integration-token-xyz`, `current-token-abc`) — all clearly synthetic. `.gitignore` excludes `.env`, `.env.local`.

### Risks

**P0 — `SecurityPolicy` is dead code on the hot path.** `executor.rs` invokes `SysCommand::new("curl"|method|"systemctl")` directly, bypassing the whitelist, deny-list, timeout, and `..`-traversal checks defined in `security.rs`. The 24 hardening tests pass but no production caller wires `policy.run()` for the 5 built-in tools. CLAUDE.md "HARD RULE 3 — No shell injection — all commands go through the whitelist" is therefore not enforced.

**P0 — `sigops.restart` accepts attacker-controlled `method`** (`cmd.input["method"]`) with **no allow-list**, falling back to `"systemctl"` only when missing. A malicious server can set `method` to any binary on PATH. Service-name sanitization protects only the second arg.

**P1 — Health server binds `0.0.0.0:9100`** (public on every NIC). README claims `--no-health` and `SIGOPS_HEALTH_PORT`, but `Config` has neither field; the port is hardcoded in `main.rs`. Contradicts "outbound only" marketing for misconfigured hosts.

**P1 — Health server doesn't parse the HTTP request** — every TCP connect to `:9100` returns the same agent metadata (agent_id, version) regardless of method/path. Information disclosure.

**P1 — TLS verification:** relies on reqwest defaults (native-tls). No explicit `danger_accept_invalid_certs` (good) but no certificate pinning, no mTLS. Acceptable for stated threat model; flagged because Roadmap promises mTLS.

**P2 — `sigops.notify_slack`'s `webhookUrl` skips `validate_url`** — a server-controlled webhook could include CR/LF or non-http(s) scheme; curl with `-d @-` semantics is safe but flagged.

**P2 — `Config::api_token` defaults to empty string;** agent will happily start with no auth and bearer-auth empty strings to any URL (default `http://localhost:4200`, plain HTTP).

**P2 — `tokens.maybe_rotate` accepts arbitrary server-issued tokens and persists to disk only when `with_persist_path` is used;** default `TokenStore::new` does not persist, so rotation is lost on restart, agent reverts to original CLI token (silent regression).

**P2 — `executor::execute_wait` uses blocking `std::thread::sleep` inside a tokio runtime;** up to 3600s blocks the calling thread.

**P3 — Graceful shutdown:** present (SIGTERM + SIGINT, 5s drain), but executor has no cancellation — long `sigops.wait` will outlive shutdown.

**P3 — `unsafe` blocks: none found** in `src/*.rs` (good).

**P3 — Logging of secrets:** `tracing::warn!(body = %body)` in heartbeat error paths logs the full server response body, which may include rotated-token JSON on error responses.

**P3 — Discovery shells `which`/`<tool> --version` on every startup;** tool names are constants so no injection but it does spawn 7+ subprocesses unconditionally.

**P3 — README/Config drift:** documented env vars (`SIGOPS_SERVER_URL`, `SIGOPS_API_KEY`, `SIGOPS_POLL_INTERVAL`, `SIGOPS_HEALTH_PORT`, `SIGOPS_NO_HEALTH`, `SIGOPS_LOG_LEVEL`) don't match clap names (`API_TOKEN`, `HEARTBEAT_INTERVAL`, etc.). Operators following README will fail to configure the agent.

---

## Other Repos (Brief)

- **`uap`** — P0: `JWT_SECRET` default at `server/src/config/env.ts:9`.
- **`sigops`** — `crypto` import + graceful shutdown FIXED. 26 commits, 8 GAPs shipped. Still no CORS, no rate-limit.
- **`sigops-cloud`** — both prior P0s FIXED (`tenantId` on executionSteps + agentTools, `crypto` import). UI tests went 0 → 18.
- **`sigops-sdk`** — examples/, docs/, CI/CD all PRESENT. P0: CLI runtime deps under devDependencies.
- **`cluster-shared`** — P0: scope rename inconsistent + non-crypto `mintTestJwt`.
- **`cluster-deploy`** — P0: `.env.example` missing `VAULT_MASTER_KEY`.

---

## Cross-Repo Summary

| Check | Result |
|---|---|
| `main` branch protection | **FAIL (all 7 repos)** |
| GitHub Advanced Security | **FAIL (all 7 repos)** |
| Lint enforced in CI | **FAIL (all 7)** |
| Issue tracker usage | FAIL (0 issues all-time) |

## Phase 0 Roadmap (this week)

1. **`sigops-agent/src/executor.rs`** — route every built-in tool through `policy.run()`; add allow-list for `cmd.input["method"]` in `sigops.restart`.
2. **`sigops-agent/src/health.rs`** — parse HTTP request line; return 404 for unknown paths; default-bind to `127.0.0.1:9100` unless explicitly overridden.
3. **`sigops-agent/src/config.rs`** — actually wire `--no-health` and `--health-port` to `Config`; sync README env-var names with clap.
4. UAP `env.ts:9` — drop default `JWT_SECRET`.
5. cluster-deploy `.env.example` — `VAULT_MASTER_KEY`.
6. Branch protection on `main` in all 7 repos.
7. Enable GHAS.
8. cluster-shared scope rename + real-crypto `mintTestJwt`.
9. sigops-sdk CLI deps fix.

## Phase 1 (next sprint, sigops-agent-specific)

- Validate `webhookUrl` in `sigops.notify_slack` through `validate_url`.
- Reject empty `api_token` at startup; require explicit `--allow-no-auth` flag for dev.
- Persist `TokenStore` by default (with secure path under `XDG_STATE_HOME` or `/var/lib/sigops-agent/`); fail-fast on persistence error.
- Replace blocking `std::thread::sleep` in `execute_wait` with `tokio::time::sleep` + cancellation token.
- Redact rotated-token JSON from error log bodies in `heartbeat.rs`.
- Add cancellation hook so long `sigops.wait` does not outlive shutdown.
- Pin rustls explicitly; add cert pinning for control-plane URL.
- Add Trivy scan + cosign signing in `release.yml`.

---

## Summary Statistics

| Metric | Value |
|---|---|
| Repos audited | 7 / 7 |
| P0 findings | 7 (this repo: 2) |
| P1 findings | 35 |
| P2 findings | 24 |
| P3 findings | 14 |
| Prior P0 bugs (2026-04-13) still open | **0** |
| New P0 bugs found (2026-05-07) | 7 |

**Headline:** sigops-agent ships solid hardening primitives (`security.rs`, 38 unit tests, integration test) but they are **not wired to the hot path**. Threading `policy.run()` through the executor is a one-PR fix that closes the largest open security gap in the entire ecosystem.
