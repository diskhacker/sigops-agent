# Memory: Final Audit Fixes 2026-04-17

## Status
partial — CLAUDE.md corrected; no release tag

## Key facts for next session
- Transport: HTTP long-polling ONLY — tokio-tungstenite NOT in Cargo.toml (WSS aspirational)
- Security layers shipped: command whitelist, path deny list, execution timeout, token rotation
- Security layers NOT shipped: mTLS, sandbox, signed commands — sha2/ring NOT in Cargo.toml
- Module layout: FLAT (config, discovery, heartbeat, executor, security, health, main, lib)
- Binaries: never published (release.yml never fired, no tag)
- No database, no server port — outbound-only agent
- Next action: Vivek pushes tag to trigger release.yml and publish binaries
