# Memory: Release Readiness

## Key facts for future sessions
- Agent ships HTTP long-polling transport, not WSS. Any WSS work must update README + CLAUDE.md in the same PR.
- No shipped release binaries yet (release.yml wired, never fired).
- Security model today: whitelist + path deny + timeout + token rotation per heartbeat + outbound-only. mTLS / sandbox / signed commands still Roadmap.
- No Dockerfile, no installer script in repo. Both listed on Roadmap.

## PRs merged this session
None. Verification-only.
