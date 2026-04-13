# ClusterAssets — Repository Architecture Review & Corrected Plan

---

## YOUR 4 QUESTIONS — ANSWERED

---

### Q1: Are UI and Backend Separated?

**CORRECTED ANSWER: NO — they should be TOGETHER in each product repo.**

**Why the previous plan was wrong:**
- Had `universal-admin-platform` (backend) + `admin-platform-ui` (frontend) as separate repos
- This BREAKS the protocol: "Feature = UI + Backend complete" — one PR should contain BOTH
- Separate repos = separate PRs, separate versioning, drift between UI and API

**Correct approach: Monorepo per product with `server/` + `ui/` directories**

```
credora/
├── CLAUDE.md
├── server/                    ← Hono backend
│   ├── package.json
│   ├── src/
│   │   ├── index.ts
│   │   ├── db/schema.ts
│   │   └── modules/
│   └── vitest.config.ts
├── ui/                        ← React frontend
│   ├── package.json
│   ├── src/
│   │   ├── App.tsx
│   │   ├── pages/
│   │   └── hooks/
│   └── vite.config.ts
├── docs/
├── docker-compose.yml
└── pnpm-workspace.yaml        ← pnpm workspace links server + ui
```

**Benefits:**
- One PR = full feature (server route + UI page + tests)
- One `docker compose up` runs everything
- Shared types between server ↔ ui via workspace
- Follows protocol: Feature = UI + Backend + Tests

**Exception: SigOps** — open source core must be in a PUBLIC repo, cloud extensions in PRIVATE. See Q3.

---

### Q2: Are Backends Split as Microservices Where Needed?

**ANSWER: Modular monolith FIRST, extract microservices only when justified.**

**What stays as modular monolith (single Hono server, module folders):**
- Credora (114 tables but one domain — accounting)
- Assera (property + elderly — tightly coupled)
- Talentra (20 modules but one product experience)
- Movana, Lifetra, Futurevo, Novix (small enough, no benefit splitting)

**What IS already a microservice by nature:**
- UAP — standalone service consumed by ALL products via HTTP
- Armexa — API-only security service consumed by other products
- SigOps Agent — separate Rust binary on customer infra
- SigOps Cloud — premium extensions running as separate service

**When to extract a microservice (future):**
- A module needs independent scaling (e.g., Credora's GST filing during deadline rush)
- A module has a completely different deployment pattern (e.g., Movana's real-time GPS ingestion)
- Cross-product functionality grows (e.g., Armexa SOC might need its own scaling)

**Current architecture per product:**

```
┌─────────────────────────────────────────────────┐
│ Product Repo (e.g., credora)                    │
│                                                  │
│  server/ (Hono modular monolith)                │
│  ├── modules/clients/     ← route + service     │
│  ├── modules/ledger/      ← route + service     │
│  ├── modules/gst/         ← route + service     │
│  ├── modules/tds/         ← route + service     │
│  └── ... (each module = folder, NOT microservice)│
│                                                  │
│  ui/ (React SPA)                                │
│  ├── pages/Clients/                             │
│  ├── pages/Ledger/                              │
│  └── pages/Gst/                                 │
│                                                  │
│  Both deployed together. One DB. One Redis.      │
└──────────────────┬──────────────────────────────┘
                   │ HTTP (JWT)
                   ▼
┌──────────────────────────────────────────────────┐
│ UAP (separate service, port 4100)                │
│ Auth, Billing, RBAC, AI, Store, Audit            │
└──────────────────────────────────────────────────┘
                   │ HTTP (service key)
                   ▼
┌──────────────────────────────────────────────────┐
│ Armexa (separate service, port 4600)             │
│ HSM signing, device identity, SOC                │
└──────────────────────────────────────────────────┘
```

---

### Q3: Is Open Source Publishing Controlled?

**ANSWER: YES — strict separation via repo boundary.**

**The problem:** SigOps core is MIT, but we don't want proprietary features (AI advisor, marketplace, premium plugins, advanced analytics) leaking into the public repo.

**Solution: Two repos with clear boundary**

```
sigops (PUBLIC — MIT)                    sigops-cloud (PRIVATE — Proprietary)
├── server/                              ├── server/
│   ├── modules/signals/       ✅        │   ├── modules/ai-advisor/      🔒
│   ├── modules/execution/     ✅        │   ├── modules/marketplace/     🔒
│   ├── modules/sel-engine/    ✅        │   ├── modules/analytics/       🔒
│   ├── modules/agents/        ✅        │   ├── modules/sig-shield/      🔒
│   ├── modules/tools/         ✅        │   ├── modules/sig-chaos/       🔒
│   └── modules/workflows/     ✅        │   └── extends sigops core      🔒
├── ui/                                  ├── ui/
│   ├── pages/Signals/         ✅        │   ├── pages/AiAdvisor/         🔒
│   ├── pages/Executions/      ✅        │   ├── pages/Marketplace/       🔒
│   └── pages/Workflows/       ✅        │   └── pages/Analytics/         🔒
├── LICENSE (MIT)               ✅        └── NO LICENSE FILE              🔒
└── CONTRIBUTING.md             ✅

sigops-agent (PUBLIC — MIT)
├── src/main.rs                ✅
├── LICENSE (MIT)              ✅
└── Cargo.toml                 ✅
```

**Publishing controls:**
1. `sigops` and `sigops-agent` repos have MIT LICENSE file
2. `sigops-cloud` has NO license (all rights reserved)
3. GitHub branch protection: PRs to `sigops` (public) require review from 2 maintainers
4. CI check: automated scan that proprietary code/imports don't appear in public repo
5. `.github/CODEOWNERS` file: security-sensitive paths require senior review
6. `sigops-cloud` imports `sigops` as a dependency and EXTENDS it — never the reverse

---

### Q4: Is Code Reuse Correctly Maintained?

**ANSWER: YES — via `cluster-shared` monorepo with @cluster/* packages.**

```
cluster-shared/
├── packages/
│   ├── @cluster/uap-client/          ← Used by ALL 10 products
│   │   ├── src/
│   │   │   ├── client.ts             # HTTP client class
│   │   │   ├── types.ts              # Shared types (User, Tenant, Role, etc.)
│   │   │   └── index.ts              # Public API
│   │   └── package.json
│   │
│   ├── @cluster/auth-middleware/      ← Used by ALL 10 products
│   │   ├── src/
│   │   │   ├── jwt.ts                # JWT HS256 verify via jose
│   │   │   ├── rbac.ts               # Permission checker
│   │   │   ├── tenant.ts             # Tenant isolation middleware
│   │   │   └── index.ts
│   │   └── package.json
│   │
│   ├── @cluster/drizzle-utils/       ← Used by ALL 10 products
│   │   ├── src/
│   │   │   ├── pagination.ts         # Paginate + search helper
│   │   │   ├── audit.ts              # Auto audit log on mutations
│   │   │   ├── tenant-scope.ts       # .where(eq(table.tenantId, ctx.tenantId))
│   │   │   ├── soft-delete.ts        # Soft delete helper
│   │   │   └── index.ts
│   │   └── package.json
│   │
│   ├── @cluster/zod-schemas/         ← Used by ALL 10 products
│   │   ├── src/
│   │   │   ├── pagination.ts         # { page, limit, sortBy, sortOrder }
│   │   │   ├── error-response.ts     # { error: { code, message, details } }
│   │   │   ├── common.ts             # email, phone, pan, gstin, uuid schemas
│   │   │   └── index.ts
│   │   └── package.json
│   │
│   ├── @cluster/ui-kit/              ← Used by ALL product UIs
│   │   ├── src/
│   │   │   ├── DataTable.tsx          # Paginated table with search
│   │   │   ├── FormDialog.tsx         # Create/Edit dialog
│   │   │   ├── ConfirmDialog.tsx      # Delete confirmation
│   │   │   ├── PageHeader.tsx         # Title + breadcrumb + actions
│   │   │   ├── StatusBadge.tsx        # Colored status chip
│   │   │   ├── EmptyState.tsx         # No data illustration
│   │   │   ├── LoadingSkeleton.tsx    # Loading placeholder
│   │   │   ├── AppShell.tsx           # Sidebar + header layout
│   │   │   ├── ThemeProvider.tsx      # MUI theme (dark-first)
│   │   │   └── index.ts
│   │   └── package.json
│   │
│   ├── @cluster/test-utils/          ← Used by ALL product tests
│   │   ├── src/
│   │   │   ├── jwt.ts                # Mint test JWTs
│   │   │   ├── db.ts                 # Test DB setup/teardown
│   │   │   ├── api.ts                # Supertest wrapper with auth
│   │   │   ├── seed.ts               # Common seed data factories
│   │   │   └── index.ts
│   │   └── package.json
│   │
│   └── @cluster/eslint-config/       ← Used by ALL repos
│       ├── index.js                   # ESLint flat config
│       └── package.json
│
├── pnpm-workspace.yaml
├── turbo.json                         # Turborepo for build orchestration
└── package.json
```

**How products consume shared packages:**

```json
// credora/server/package.json
{
  "dependencies": {
    "@cluster/uap-client": "workspace:*",
    "@cluster/auth-middleware": "workspace:*",
    "@cluster/drizzle-utils": "workspace:*",
    "@cluster/zod-schemas": "workspace:*"
  }
}
```

**For GitHub (not local workspace), publish to GitHub Packages (private npm registry):**
```
npm config set @cluster:registry https://npm.pkg.github.com
```

---

## CORRECTED REPO LIST (14 Repos)

| # | Repo Name | Visibility | Contains | Why This Structure |
|---|-----------|-----------|----------|-------------------|
| 1 | `cluster-shared` | PRIVATE | @cluster/* NPM packages | Reuse across all products |
| 2 | `uap` | PRIVATE | server/ + ui/ | Foundation — one repo = one feature PR |
| 3 | `sigops` | PUBLIC | server/ + ui/ (MIT core) | Open source drives adoption |
| 4 | `sigops-cloud` | PRIVATE | Premium extensions | Proprietary features separated |
| 5 | `sigops-agent` | PUBLIC | Rust binary (MIT) | Customers must trust agent code |
| 6 | `credora` | PRIVATE | server/ + ui/ | Financial OS — single domain |
| 7 | `assera` | PRIVATE | server/ + ui/ | Property + elderly — coupled |
| 8 | `talentra` | PRIVATE | server/ + ui/ | Workforce platform |
| 9 | `armexa` | PRIVATE | server/ only (API) | Security infra — no end-user UI |
| 10 | `futurevo` | PRIVATE | server/ + ui/ | Child career guidance |
| 11 | `movana` | PRIVATE | server/ + ui/ | Vehicle intelligence |
| 12 | `lifetra` | PRIVATE | server/ + ui/ | Health platform |
| 13 | `paynex` | PRIVATE | server/ + ui/ | Payments — regulated |
| 14 | `novix` | PRIVATE | server/ + ui/ | Internal AI lab |

**Changes from previous plan:**
- ❌ Removed `universal-admin-platform` + `admin-platform-ui` (2 repos) → ✅ Combined into `uap` (1 repo)
- ❌ Removed `sigops-core` + `sigops-ui` (2 repos) → ✅ `sigops` (1 repo) + `sigops-cloud` (new, premium)
- ❌ 15 repos → ✅ 14 repos (cleaner)

---
