# ClusterAssets — Master Repository Plan

## Session: 2026-04-10 | Architecture → Implementation Bridge

---

## 1. Repository Structure (15 Repos)

| # | Repo Name | Product | Type | Status | Priority |
|---|-----------|---------|------|--------|----------|
| 1 | `universal-admin-platform` | UAP | Backend | **EXISTS** — Hono+Drizzle built | Set 1 |
| 2 | `admin-platform-ui` | UAP | Frontend | **EXISTS** — Partial React+MUI | Set 1 |
| 3 | `sigops-core` | SigOps | Backend | **EXISTS** — Phase 1 built | Set 1 |
| 4 | `sigops-ui` | SigOps | Frontend | **NEW** — needs creation | Set 1 |
| 5 | `credora` | Credora | Full-stack | **EXISTS** — refactoring for UAP | Set 1 |
| 6 | `assera` | Assera | Full-stack | **NEW** — needs creation | Set 1 |
| 7 | `talentra` | Talentra | Full-stack | **NEW** — needs creation | Set 2 |
| 8 | `armexa` | Armexa | Backend (API-only) | **NEW** — needs creation | Set 2 |
| 9 | `futurevo` | Futurevo | Full-stack | **NEW** — needs creation | Set 2 |
| 10 | `movana` | Movana | Full-stack | **NEW** — needs creation | Set 3 |
| 11 | `lifetra` | Lifetra | Full-stack | **NEW** — needs creation | Set 3 |
| 12 | `paynex` | Paynex | Full-stack | **NEW** — needs creation | Set 4 |
| 13 | `novix` | Novix | Internal tool | **NEW** — lowest priority | As needed |
| 14 | `sigops-agent` | SigOps | Rust binary | **NEW** — needs creation | Set 1 |
| 15 | `cluster-shared` | Shared | NPM packages | **NEW** — @cluster/* packages | Set 1 |

---

## 2. Shared NPM Packages (`cluster-shared` monorepo)

```
cluster-shared/
├── packages/
│   ├── @cluster/uap-client/      # HTTP client for UAP APIs (used by ALL products)
│   ├── @cluster/ui-kit/          # Shared React components (MUI-based)
│   ├── @cluster/auth-middleware/  # JWT HS256 verification middleware for Hono
│   ├── @cluster/drizzle-utils/   # Common Drizzle helpers (tenant isolation, audit, pagination)
│   ├── @cluster/zod-schemas/     # Shared Zod schemas (pagination, error response, etc.)
│   ├── @cluster/test-utils/      # Test helpers (JWT minting, DB seeding, API client)
│   └── @cluster/eslint-config/   # Shared ESLint + Prettier config
├── package.json                  # pnpm workspace
└── turbo.json                    # Turborepo config
```

---

## 3. Standard Repo Structure (Every Product)

```
<product>/
├── CLAUDE.md                     # ⭐ THE build instruction file for Claude Code
├── README.md                     # Setup + run instructions
├── package.json
├── tsconfig.json
├── drizzle.config.ts
├── docker-compose.yml            # PostgreSQL + Redis
├── .env.example
├── docs/
│   ├── session/                  # Session logs (one per Claude Code session)
│   │   └── session-001-*.md
│   ├── memory/                   # Persistent memory (what's built, what's decided)
│   │   └── memory.md
│   └── architecture/             # Architecture PDF/DOCX (reference)
│       └── <Product>-Architecture-<version>.pdf
├── src/
│   ├── index.ts                  # Hono app entry
│   ├── config/
│   │   └── index.ts              # Env validation (Zod)
│   ├── db/
│   │   ├── schema.ts             # ⭐ ALL Drizzle table definitions
│   │   ├── migrations/           # Drizzle migrations
│   │   └── seed.ts               # Seed data
│   ├── lib/
│   │   ├── uap-client.ts         # UAP HTTP client (@cluster/uap-client)
│   │   └── auth.ts               # JWT verification middleware
│   ├── modules/
│   │   ├── <module>/
│   │   │   ├── routes.ts         # Hono routes (all CRUD + SEARCH)
│   │   │   ├── service.ts        # Business logic
│   │   │   ├── validation.ts     # Zod schemas (request/response)
│   │   │   └── __tests__/
│   │   │       ├── routes.test.ts      # Integration tests
│   │   │       ├── service.test.ts     # Unit tests
│   │   │       └── readiness.test.ts   # Readiness tests
│   │   └── ...
│   └── shared/
│       ├── types.ts              # Shared TypeScript types
│       ├── errors.ts             # Error classes
│       ├── pagination.ts         # Pagination helper
│       └── middleware.ts         # Tenant isolation, audit logging
├── ui/                           # Frontend (same repo, separate Vite app)
│   ├── package.json
│   ├── vite.config.ts
│   ├── src/
│   │   ├── App.tsx
│   │   ├── main.tsx
│   │   ├── stores/               # Zustand stores
│   │   ├── hooks/                # TanStack Query hooks
│   │   ├── components/           # Shared components
│   │   ├── pages/                # Route pages
│   │   │   ├── <Module>/
│   │   │   │   ├── List.tsx      # List + Search view
│   │   │   │   ├── Detail.tsx    # Detail view
│   │   │   │   ├── Form.tsx      # Create/Edit form
│   │   │   │   └── __tests__/
│   │   │   └── ...
│   │   └── lib/
│   │       ├── api.ts            # Axios/fetch wrapper
│   │       └── auth.ts           # JWT handling
│   └── cypress/                  # E2E + cross-browser tests
│       ├── e2e/
│       └── support/
└── vitest.config.ts              # Test config (>90% coverage target)
```

---

## 4. Protocol Enforcement (MANDATORY in Every CLAUDE.md)

### Build Protocol: Audit → Review → Confirm → Reuse → Implement

```
For EVERY feature:
1. AUDIT    — Check what exists (memory.md, session logs, codebase)
2. REVIEW   — Review architecture doc for requirements
3. CONFIRM  — Confirm approach before writing code
4. REUSE    — Check @cluster/* packages, existing modules, shared code
5. IMPLEMENT — Write code following standards below
```

### Completion Definition

```
A feature is COMPLETE only when ALL of these pass:
✅ Backend API: all CRUD + SEARCH endpoints working
✅ Zod validation on all request/response
✅ Frontend UI: List + Detail + Create + Edit + Search pages
✅ UI ↔ Backend wired via TanStack Query hooks
✅ Unit tests (service.test.ts) — >90% coverage
✅ Integration tests (routes.test.ts) — all endpoints tested
✅ Readiness tests (readiness.test.ts) — health, auth, tenant isolation
✅ Responsive (mobile + tablet + desktop)
✅ Cross-browser tested (Chrome, Firefox, Safari, Edge)
✅ Session log updated
✅ Memory updated
```

### Naming Convention

```
Files:      kebab-case (client-vault.ts, gst-module.ts)
Tables:     snake_case (gst_returns, tds_deductees)
Columns:    snake_case (created_at, tenant_id)
Routes:     kebab-case (/api/v1/gst-returns)
Components: PascalCase (ClientList.tsx, GstReturnForm.tsx)
Stores:     camelCase (useClientStore, useGstStore)
Types:      PascalCase (Client, GstReturn, TdsDeductee)
Tests:      *.test.ts / *.test.tsx
```

### Wiring Standard

```
Every module follows this exact wiring:

schema.ts (Drizzle) → service.ts (logic) → routes.ts (Hono) → validation.ts (Zod)
                                                                      ↕
ui/hooks/use<Module>.ts (TanStack Query) → ui/pages/<Module>/*.tsx (React+MUI)
```

---

## 5. Build Order (Per Product)

```
Phase 1: Foundation
  1. CLAUDE.md + architecture doc in /docs/
  2. Docker + env + config
  3. Drizzle schema (ALL tables at once)
  4. DB migrations + seed
  5. UAP client + JWT middleware
  6. Health endpoint + readiness test

Phase 2: Core Modules (iterate per module)
  For each module:
    1. Drizzle schema (already done in Phase 1)
    2. Zod validation schemas
    3. Service layer (business logic)
    4. Hono routes (all CRUD + SEARCH)
    5. Unit tests (>90%)
    6. Integration tests (all endpoints)
    7. React pages (List + Detail + Form)
    8. TanStack Query hooks
    9. Responsive + cross-browser
    10. Update session log + memory

Phase 3: Integration
  1. UAP integration tests
  2. Inter-product API tests
  3. E2E tests (Cypress)
  4. Performance baseline

Phase 4: Polish
  1. Error handling audit
  2. Loading states, empty states
  3. Accessibility audit
  4. Security audit
```

---

## 6. Environment Variables (Standard Across All Products)

```bash
# Database
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/<product>_dev

# Redis
REDIS_URL=redis://localhost:6379

# UAP Integration
UAP_URL=http://localhost:4100/api/v1
UAP_API_KEY=<service-key>
JWT_SECRET=<shared-hs256-secret>

# Product Identity
PRODUCT_ID=<product-slug>
PRODUCT_NAME=<Product Name>
PORT=<port>

# AI (via UAP)
AI_ENABLED=true

# Environment
NODE_ENV=development
LOG_LEVEL=debug
```

### Port Assignments

| Product | Backend Port | Frontend Port |
|---------|-------------|---------------|
| UAP | 4100 | 4101 |
| SigOps | 4200 | 4201 |
| Credora | 4300 | 4301 |
| Assera | 4400 | 4401 |
| Talentra | 4500 | 4501 |
| Armexa | 4600 | — (API only) |
| Futurevo | 4700 | 4701 |
| Movana | 4800 | 4801 |
| Lifetra | 4900 | 4901 |
| Paynex | 5000 | 5001 |
| Novix | 5100 | 5101 |
