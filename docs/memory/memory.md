# ClusterAssets — Project Memory

> Single source of truth. Read FIRST in every session. Updated: 2026-04-12

## Ecosystem
11 SaaS products + UAP foundation. 15 repos (3 PUBLIC, 12 PRIVATE). ~362 tables, ~730 endpoints.

## Products
UAP (43t/98e), SigOps (10t/25e), Credora (114t/190e), Assera (26t/72e), Talentra (54t/110e), Armexa (18t/48e), Futurevo (12t/30e), Movana (22t/65e), Lifetra (18t/45e), Paynex (35t/72e), Novix (10t/20e).

## SigOps SDK (6 packages, ALL PUBLIC MIT)
@sigops/cli, @sigops/tool-sdk, @sigops/template-sdk, @sigops/sel-tools, @sigops/vscode-ext, @sigops/plugin-sdk. Supports tool + template + connector + plugin development. sel-tools PUBLIC because cli+template-sdk+vscode-ext depend on it. vscode-ext PUBLIC because VS Code Marketplace policy + developer trust.

## Repos (15): cluster-shared, uap, sigops, sigops-sdk, sigops-agent, sigops-cloud, credora, assera, talentra, armexa, futurevo, movana, lifetra, paynex, novix
Each product = 1 repo with server/+ui/ (monorepo). sigops-sdk + cluster-shared = pnpm monorepos (no DB).

## Stack: Hono+Drizzle+PostgreSQL+Redis+BullMQ+React18+Vite+Zustand+MUI+Zod+jose HS256

## Launch: Set1=UAP+SigOps+Credora+Assera, Set2=Armexa+Talentra+Futurevo+sigops-cloud+sigops-sdk, Set3=Movana+Lifetra, Set4=Paynex

## Protocols: Audit→Review→Confirm→Reuse→Implement. Feature=UI+Backend+Tests(>90%). CRUD+SEARCH. Session logs + memory updates mandatory.

## CLAUDE.md Status: COMPLETE for UAP, SigOps, SigOps-SDK, Credora, Assera. STUB for Talentra, Armexa, Paynex, Movana, Lifetra, Futurevo, Novix.

## Sessions
- session-001 (2026-04-10): Architecture docs (28 files), repo plan, CLAUDE.md Set 1
- session-002 (2026-04-12): SDK arch+manual (4 files), deep audit, PS1 fixed (15 repos), audit prompt
