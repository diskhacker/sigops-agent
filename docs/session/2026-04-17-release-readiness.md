# Session: Release Readiness
Date: 2026-04-17

## Scope
SigOps-agent slice of the cross-repo Release Readiness sprint. Verification only — no file changes this session.

## PRs merged
None. No changes required.

## Verification done
- README: honest (Sprint 1.5 already stripped WSS / 8-layer / mTLS / signed-command claims). Only Roadmap section mentions these.
- `.github/workflows/ci.yml`: present. 3-OS matrix (Ubuntu / macOS / Windows) runs `fmt --check`, `clippy -D warnings`, `test --all-targets`, `build --release`.
- `.github/workflows/release.yml`: present. Tag-triggered (`v*`) + manual dispatch. Cross-compiles 5 targets (linux musl x86_64/aarch64 via `cross`, macOS x86_64/aarch64, windows x86_64). Never fired yet (no tag cut).
- Code: matches README (HTTP long-polling heartbeat, command whitelist + path deny + timeout + TokenStore, outbound-only, optional `/health` on :9100).

## Release readiness
- [x] README honest
- [x] CI workflow present and healthy
- [x] Release workflow present (cross-compile to 5 targets)
- [ ] First `v0.1.0` tag — pending, Vivek to push. Fires `release.yml` → binaries uploaded to GitHub Release.
- [ ] (Roadmap) `cargo audit` / SBOM / code-signing — out of scope for v0.1.0.

## Still outstanding (not blocking v0.1.0)
All tracked in the README Roadmap already: WSS transport, mTLS, ed25519 signed commands, namespace/cgroup sandbox, auto-update, pre-built installers, Docker image, code-signed releases, `pm2`/`kubectl` adapters, `cargo audit` in CI.

## Next steps
1. Vivek tags `v0.1.0` — release.yml produces 5 cross-compiled binaries + checksums.
