# Claude Code — Universal Build & Audit Prompt

> Drop this into ANY ClusterAssets repo. It enforces the same procedure everywhere.
> Works for: UAP, SigOps, SigOps-SDK, SigOps-Cloud, Credora, Assera, Talentra,
> Armexa, Futurevo, Movana, Lifetra, Paynex, Novix, cluster-shared, sigops-agent

---

## STEP 0 — READ BEFORE DOING ANYTHING

```
1. Read CLAUDE.md in this repo root (build instructions, schemas, roles, endpoints)
2. Read docs/memory/memory.md (what's been built, what's decided)
3. Read docs/session/*.md (all previous session logs — what was completed)
4. Read docs/architecture/*.pdf (the FULL architecture spec — table columns, API design, business rules)
5. If any of these files are missing, CREATE them before proceeding.
```

**Do NOT write a single line of code until all 4 files are read.**

---

## STEP 1 — PROTOCOLS (MANDATORY, NEVER SKIP)

```
AUDIT    → Check what exists (memory, sessions, codebase, git log)
REVIEW   → Cross-check against architecture doc for requirements
CONFIRM  → Confirm approach before writing code
REUSE    → Check @cluster/* packages, existing modules, shared code first
IMPLEMENT → Write code following ALL standards below
```

### Feature Completion Definition
A feature is **NOT COMPLETE** until ALL of these pass:

```
✅ Drizzle schema in server/src/db/schema.ts (id, tenant_id, timestamps, indexes)
✅ Zod validation in server/src/modules/<module>/validation.ts (all request + response)
✅ Service layer in server/src/modules/<module>/service.ts (business logic)
✅ Hono routes in server/src/modules/<module>/routes.ts (ALL endpoints: CRUD + SEARCH)
✅ React pages in ui/src/pages/<Module>/ (List + Detail + Form + Search)
✅ TanStack Query hooks in ui/src/hooks/use<Module>.ts
✅ Unit tests in server/src/modules/<module>/__tests__/service.test.ts (>90% coverage)
✅ Integration tests in server/src/modules/<module>/__tests__/routes.test.ts (all endpoints)
✅ Readiness test (health, auth, tenant isolation)
✅ Responsive UI (mobile 375px + tablet 768px + desktop 1440px)
✅ Cross-browser tested (Chrome, Firefox, Safari, Edge)
✅ Session log updated in docs/session/
✅ Memory updated in docs/memory/memory.md
```

**Backend-only or UI-only is NEVER considered complete. Both must exist and be connected.**

### Every Endpoint MUST Have
```
GET    /api/v1/<resource>           → List + SEARCH (pagination, filter, sort)
GET    /api/v1/<resource>/:id       → Get by ID
POST   /api/v1/<resource>           → Create (Zod validated)
PUT    /api/v1/<resource>/:id       → Update (Zod validated)
DELETE /api/v1/<resource>/:id       → Delete (soft or hard per domain)
GET    /api/v1/<resource>/search    → Full-text search (optional, for complex search)
```

No partial CRUD. If the resource exists, ALL operations must exist.

---

## STEP 2 — NAMING CONVENTION (ENFORCED)

```
Files:      kebab-case       (client-vault.ts, gst-module.ts)
Tables:     snake_case       (gst_returns, shift_handovers)
Columns:    snake_case       (created_at, tenant_id)
Routes:     kebab-case       (/api/v1/gst-returns, /api/v1/shift-handovers)
Components: PascalCase       (ClientList.tsx, ShiftHandoverForm.tsx)
Stores:     camelCase        (useClientStore, useShiftStore)
Types:      PascalCase       (Client, ShiftHandover)
Tests:      *.test.ts        (service.test.ts, routes.test.ts)
Env vars:   SCREAMING_SNAKE  (DATABASE_URL, JWT_SECRET)
```

---

## STEP 3 — WIRING STANDARD (EVERY MODULE FOLLOWS THIS)

```
server/src/db/schema.ts (Drizzle tables)
    ↓
server/src/modules/<module>/validation.ts (Zod schemas)
    ↓
server/src/modules/<module>/service.ts (business logic — queries Drizzle, validates Zod)
    ↓
server/src/modules/<module>/routes.ts (Hono routes — calls service, returns JSON)
    ↓
ui/src/hooks/use<Module>.ts (TanStack Query — calls API endpoints)
    ↓
ui/src/pages/<Module>/List.tsx + Detail.tsx + Form.tsx (React + MUI)
```

**Never skip a layer. Never call Drizzle directly from routes. Never call API directly from components.**

---

## STEP 4 — SECURITY (EVERY REQUEST)

```typescript
// Every route file starts with:
import { authMiddleware, tenantMiddleware, rbacMiddleware } from "@cluster/auth-middleware";
import { auditLog } from "@cluster/drizzle-utils";

// Every route group:
app.use("/*", authMiddleware());     // JWT HS256 verification
app.use("/*", tenantMiddleware());   // Extract tenant_id from JWT
app.use("/*", rbacMiddleware());     // Check permissions

// Every query:
.where(eq(table.tenantId, ctx.tenantId))  // ALWAYS filter by tenant

// Every mutation:
await auditLog(ctx, "resource.action", { before, after });  // ALWAYS audit
```

### Security Checklist (EVERY repo):
- [ ] JWT HS256 verification on ALL non-public endpoints
- [ ] Tenant isolation: EVERY query filters by tenant_id
- [ ] Rate limiting: Redis sliding window on ALL endpoints
- [ ] Input validation: Zod on EVERY request body and query param
- [ ] Audit logging: EVERY create/update/delete
- [ ] Password hashing: Argon2id (memory=65536, iterations=3, parallelism=4)
- [ ] API key hashing: Argon2 (shown only once on creation)
- [ ] CORS: configured per product
- [ ] No raw SQL — ALL queries via Drizzle ORM
- [ ] Error responses: NEVER expose stack traces in production

---

## STEP 5 — UAP INTEGRATION (EVERY PRODUCT)

```typescript
// server/src/index.ts — at boot
import { UapClient } from "@cluster/uap-client";

const uap = new UapClient({ baseUrl: process.env.UAP_URL, apiKey: process.env.UAP_API_KEY });

// Register product (idempotent — safe to call every boot)
await uap.registerProduct({
  name: process.env.PRODUCT_ID,
  displayName: process.env.PRODUCT_NAME,
  plans: [...],     // from CLAUDE.md
  roles: [...],     // from CLAUDE.md
});
```

**What UAP handles (NEVER rebuild in any product):**
Auth, users, tenants, billing, RBAC, API keys, notifications, audit, settings, AI, App Store, sessions

**What each product builds:** Domain-specific modules ONLY

---

## STEP 6 — SHARED PACKAGES (ALWAYS USE, NEVER DUPLICATE)

```json
// server/package.json
{
  "dependencies": {
    "@cluster/uap-client": "workspace:*",
    "@cluster/auth-middleware": "workspace:*",
    "@cluster/drizzle-utils": "workspace:*",
    "@cluster/zod-schemas": "workspace:*"
  }
}

// ui/package.json
{
  "dependencies": {
    "@cluster/ui-kit": "workspace:*"
  }
}

// Both
{
  "devDependencies": {
    "@cluster/test-utils": "workspace:*",
    "@cluster/eslint-config": "workspace:*"
  }
}
```

**Before writing ANY code, check if @cluster/* already provides it.**

---

## STEP 7 — TEST COVERAGE (>90% MANDATORY)

```typescript
// vitest.config.ts — in every repo
export default defineConfig({
  test: {
    coverage: {
      thresholds: {
        lines: 90,
        functions: 90,
        branches: 85,
        statements: 90,
      },
    },
  },
});
```

### Test Types Required:
```
1. Unit tests (service.test.ts):
   - Every service function
   - Valid input → expected output
   - Invalid input → proper error
   - Edge cases (empty, null, max length)

2. Integration tests (routes.test.ts):
   - Every endpoint with valid auth → 200/201
   - No auth → 401
   - Wrong role → 403
   - Wrong tenant → empty result (never 403, just filtered)
   - Invalid body → 400 with Zod error
   - Not found → 404

3. Readiness tests (readiness.test.ts):
   - GET /health → 200 with { db: "ok", redis: "ok" }
   - JWT verification works
   - Tenant isolation works (user A can't see user B data)
```

---

## STEP 8 — UI REQUIREMENTS (EVERY PAGE)

```
Every list page:
  - Search bar (debounced, server-side)
  - Filter dropdowns (status, date range, type)
  - Sortable columns
  - Pagination (page numbers + per-page selector)
  - Empty state (illustration + "Create your first X" button)
  - Loading skeleton (not spinner)
  - Bulk actions (select multiple → delete/export)

Every detail page:
  - Breadcrumb navigation
  - Edit button → opens form
  - Delete button → confirmation dialog
  - Activity timeline (audit log for this resource)
  - Related resources (linked items)

Every form:
  - Zod validation errors displayed per field
  - Loading state on submit button
  - Success toast notification
  - Redirect to detail page after create

Global:
  - Responsive (mobile 375px, tablet 768px, desktop 1440px)
  - Dark mode (MUI theme toggle)
  - Error boundary on every route
  - 404 page
  - Keyboard navigation (tab order, Enter to submit)
```

---

## STEP 9 — SESSION LOG + MEMORY (AFTER EVERY MODULE)

### Session Log (create new file each session)
```markdown
# Session XXX — <What was done>
## Date: YYYY-MM-DD
## Product: <Product Name>

### Completed
- [ ] Module: <name> — schema ✅ validation ✅ service ✅ routes ✅ UI ✅ tests ✅

### Decisions Made
- <Any architectural decisions>

### Issues Found
- <Any bugs or gaps discovered>

### Next Steps
- <What to build next session>
```

### Memory (update existing file)
```markdown
## <Product> Build Status
| Module | Schema | Routes | UI | Tests | Status |
|--------|--------|--------|-----|-------|--------|
| <name> | ✅ | ✅ | ✅ | 92% | COMPLETE |
```

---

## STEP 10 — BUILD ORDER (PER PRODUCT)

```
Phase 1 — Foundation (EVERY repo):
  1. CLAUDE.md verified
  2. Docker compose (PostgreSQL + Redis)
  3. Config + env validation (Zod)
  4. Drizzle schema (ALL tables at once)
  5. DB migrations + seed data
  6. UAP client + JWT middleware
  7. Health endpoint + readiness test

Phase 2 — Core Modules (iterate per module):
  For each module in the order specified in CLAUDE.md:
    1. Schema (already done in Phase 1)
    2. Zod validation schemas
    3. Service layer
    4. Hono routes (ALL CRUD + SEARCH)
    5. Unit tests (>90%)
    6. Integration tests
    7. React pages (List + Detail + Form)
    8. TanStack Query hooks
    9. Responsive + cross-browser
    10. Update session log + memory

Phase 3 — Integration:
  1. UAP integration tests
  2. Inter-product API tests (if applicable)
  3. E2E tests (Cypress)

Phase 4 — Polish:
  1. Error handling audit
  2. Loading states, empty states, error boundaries
  3. Accessibility audit
  4. Security audit
```

---

## HARD RULES — NEVER BREAK

1. **NEVER rebuild auth/billing/RBAC** — use UAP via @cluster/uap-client
2. **NEVER skip tests** — untested code does NOT count as complete
3. **NEVER ship backend without UI** — or UI without backend
4. **NEVER skip tenant isolation** — every query filters by tenant_id
5. **NEVER skip audit logging** — every mutation gets logged
6. **NEVER skip Zod validation** — every request body and query param
7. **NEVER use raw SQL** — all queries via Drizzle ORM
8. **NEVER hardcode secrets** — .env only, validated by Zod at boot
9. **NEVER skip session log** — update after every module completed
10. **NEVER skip memory** — update docs/memory/memory.md after every session
11. **NEVER hallucinate** — if unsure, read the architecture PDF
12. **NEVER deviate from continuity** — check previous sessions before starting

---

## FRONTEND COMPLETENESS CHECKLIST

Every product with a `ui/` directory MUST have these pages per module:

```
For EACH backend module that has routes:

ui/src/pages/<Module>/
├── List.tsx            — DataTable with search, filter, sort, pagination, bulk actions
├── Detail.tsx          — Full resource view, breadcrumb, edit/delete actions, activity log
├── Form.tsx            — Create/Edit form with Zod validation, error display, loading state
└── __tests__/
    ├── List.test.tsx   — Renders, loads data, paginates, searches, filters
    ├── Detail.test.tsx — Renders with mock data, edit/delete actions work
    └── Form.test.tsx   — Validation errors display, submit creates/updates
```

**Minimum UI test count per product:**
- UAP (20 modules): ~60 test files (3 per module)
- Credora (34 modules): ~102 test files
- SigOps (8 modules): ~24 test files
- SigOps Cloud (20+ modules): ~60 test files
- Talentra (20 modules): ~60 test files
- Assera (7 modules): ~21 test files
- Paynex (10 modules): ~30 test files
- Movana (10 modules): ~30 test files
- Lifetra (9 modules): ~27 test files
- Futurevo (7 modules): ~21 test files
- Novix (7 modules): ~21 test files

**UI testing stack:**
```json
{
  "devDependencies": {
    "@testing-library/react": "^16.0.0",
    "@testing-library/user-event": "^14.0.0",
    "jsdom": "^25.0.0",
    "msw": "^2.0.0",
    "vitest": "^3.0.0"
  }
}
```

Use `msw` (Mock Service Worker) for API mocking — NEVER mock fetch/axios directly.

**Global UI components (from @cluster/ui-kit) must also have tests:**
```
DataTable.test.tsx, FormDialog.test.tsx, ConfirmDialog.test.tsx,
PageHeader.test.tsx, StatusBadge.test.tsx, EmptyState.test.tsx,
LoadingSkeleton.test.tsx, AppShell.test.tsx, ThemeProvider.test.tsx
```
