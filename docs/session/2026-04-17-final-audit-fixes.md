# Session: Final Audit Fixes
Date: 2026-04-17
Type: Targeted blocker fixes only

## Fixes applied this session

### Blocker 4 — CLAUDE.md corrected
- Removed false claim that `tokio-tungstenite` is in Cargo.toml
- Corrected to: tokio-tungstenite NOT in Cargo.toml — WebSocket transport not yet implemented
- Transport is HTTP long-polling today; WSS is aspirational

## Outstanding issues
- No release tag → release.yml never fired → no published binaries
- Installer URL (`get.sigops.dev/agent`) is aspirational — not deployed
- mTLS (Layer 2), sandbox (Layer 7), signed commands (Layer 8) not yet implemented
- Module layout is flat; nested ws/, tools/, discovery/ tree is aspirational
