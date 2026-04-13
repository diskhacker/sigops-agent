# ClusterAssets — Deep Audit Report

## Date: 2026-04-12
## Scope: SigOps (all repos) + UAP — completeness check

---

## QUESTION: Why must @sigops/sel-tools and @sigops/vscode-ext be open source?

### Short answer: Because everything depends on them. Making them private breaks the entire SDK.

### @sigops/sel-tools — MUST be PUBLIC

```
Dependency chain:
  @sigops/cli (PUBLIC) ──depends──→ @sigops/sel-tools
  @sigops/template-sdk (PUBLIC) ──depends──→ @sigops/sel-tools
  @sigops/vscode-ext (PUBLIC) ──depends──→ @sigops/sel-tools
  sigops core (PUBLIC, MIT) ──shares parser with──→ @sigops/sel-tools
```

If sel-tools is PRIVATE:
- ❌ CLI can't lint or format .sel files — `sigops sel lint` breaks
- ❌ template-sdk can't validate rendered .sel — `defineTemplate().render()` output can't be parsed
- ❌ VS Code extension has no Language Server — autocomplete, errors, hover all break
- ❌ Third-party developers can't parse .sel in their own tools
- ❌ The SEL parser already lives in sigops core (MIT) — extracting to private creates a license conflict

Making sel-tools private is like making the JavaScript parser private but keeping Node.js open source. The language IS the platform.

### @sigops/vscode-ext — MUST be PUBLIC

1. **VS Code Marketplace policy**: Free extensions are expected to be open source. Closed-source free extensions get flagged and receive lower trust signals.
2. **Developer trust**: Engineers won't install a closed-source IDE extension — it sees ALL their code. Open source = auditable = trustworthy.
3. **Community contributions**: VS Code extensions need community PRs for edge cases (keybindings, themes, platform-specific bugs).
4. **Adoption funnel**: The VS Code extension is the #1 way developers discover SigOps. Friction here = fewer users.
5. **Zero proprietary value**: The extension just wraps sel-tools (LSP + syntax grammar). No business logic, no secrets, no competitive advantage in keeping it closed.

### What IS proprietary (correctly):
- **sigops-cloud**: AI advisor, marketplace storefront, creator dashboard, premium plugins — THIS is the revenue engine
- **The marketplace itself**: Publishing pipeline validation, review process, payout system
- **Plugin runtime hooks**: How plugins hook into the engine at runtime (defined in sigops-cloud)

### Summary:

| Package | Visibility | Why |
|---------|-----------|-----|
| @sigops/cli | PUBLIC | Developers install it globally |
| @sigops/tool-sdk | PUBLIC | Developers import it in their tool code |
| @sigops/template-sdk | PUBLIC | Developers import it in template code |
| @sigops/sel-tools | PUBLIC | CLI + template-sdk + vscode-ext ALL depend on it. Parser shared with MIT core. |
| @sigops/vscode-ext | PUBLIC | Published free on VS Code Marketplace. Devs need to trust IDE extensions. |
| @sigops/plugin-sdk | PUBLIC | Developers import it in plugin code |

ALL 6 packages = PUBLIC. The monetization is in sigops-cloud (premium features) and the marketplace (25% commission). The SDK is the adoption funnel, not the revenue source.

---

## DEEP AUDIT: What's Missing

### 1. PS1 Script — MISSING sigops-sdk repo entirely

The uploaded setup-repos.ps1 has 14 repos. sigops-sdk is NOT in:
- ❌ Repos creation list
- ❌ $portMap
- ❌ $productMap
- ❌ $hasUI list

**Fix**: Add sigops-sdk to the PS1

### 2. CLAUDE-SigOps-SDK.md — WRONG FILE

The file in downloaded-docs named `CLAUDE-SigOps-SDK.md` is actually the SIGOPS-SDK-AUDIT.md (the repo audit document), NOT a build instruction file for Claude Code. Claude Code needs:
- ❌ No Drizzle schema (SDK has no database)
- ❌ No Zod validation examples
- ❌ No package.json scaffolds
- ❌ No test patterns
- ❌ No build order specific to SDK

**Fix**: Create proper CLAUDE.md for sigops-sdk

### 3. memory.md — No SDK mention

The memory.md in downloaded-docs does NOT mention:
- ❌ sigops-sdk repo
- ❌ 6 @sigops/* packages
- ❌ SDK Architecture doc (v1.0.0)
- ❌ SDK Developer Manual (v1.0.0)

**Fix**: Update memory.md

### 4. session-001 — No SDK session entry

Session log doesn't cover SDK work done in later conversations.

**Fix**: Create session-002 for SDK work

### 5. SigOps CLAUDE.md — Missing features from v1.0.0

The CLAUDE-SigOps.md has only 10 tables and Phase 1 content. Missing:
- ❌ 43 features not mapped to modules (only 10 of 53 features are in Phase 1)
- ❌ No FlowBuilder specification
- ❌ No on-call/postmortem/war room specs
- ❌ No knowledge engine specs
- ❌ No adapter layer specs (PagerDuty/Datadog coexistence)
- ❌ Email polling for signals (added in v1.1.0)
- ❌ Health Score gamification spec
- ❌ Incident Replay spec
- ❌ Academy & Certification spec

These are Phase 2-6 features and go in sigops-cloud, but need to be documented in CLAUDE.md

### 6. UAP CLAUDE.md — Missing AI pipeline detail

The CLAUDE-UAP.md has 43 tables and 98 endpoints but:
- ❌ AI Management 10-component pipeline lacks implementation detail
  - Provider Registry: config per provider
  - Model Catalog: which models, when to route
  - Router: complexity-based routing rules
  - Anonymizer: PII detection patterns per product
  - Cache: semantic cache with TTL
  - Rate Limiter: per-tenant per-product limits
  - Executor: 8 adapter implementations (OpenAI, Anthropic, Ollama, etc.)
  - Usage Tracker: per-call cost tracking
  - Fallback Manager: chain of fallback models
  - Audit Logger: every AI call logged
- ❌ App Store publishing pipeline validation rules
- ❌ Cross-product workflow state machine
- ❌ Webhook retry backoff formula (exponential: 1min, 5min, 30min, 2hr, 12hr)
- ❌ Tenant lifecycle cron job specification

### 7. Original v1.0.0 → Current v1.2.0 Feature Delta

The original SigOps had 53 features + 6 plugins. Current docs only cover ~15 in detail. Mapping needed for ALL 53.

### 8. Inter-product API contracts

No document specifies the EXACT API contracts between products:
- ❌ Credora → Armexa: What endpoint? What payload?
- ❌ Assera → Credora: Legal escalation handoff format?
- ❌ Talentra → Armexa: Assessment signing request/response?
- ❌ All → Paynex: Payment creation format?
- ❌ UAP → All: Product registration exact schema?

---

## CORRECTED FILES FOLLOW IN SEPARATE OUTPUTS
