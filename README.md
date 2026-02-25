# cap-mind

Monorepo containing:

- `apps/web`: Next.js app (migrated from `cap.huyixi.com`)
- `apps/cli`: Rust CLI (migrated from `cap-cli`)

## Structure

```txt
apps/
  web/
  cli/
```

## Prerequisites

- Node.js 22+
- pnpm 10+
- Rust stable

## Install

```bash
pnpm install
```

## Development

```bash
pnpm run dev:web
pnpm run dev:cli
```

## Build

```bash
pnpm run build:web
pnpm run build:cli
```

Note: `build:web` requires Supabase env values (`SUPABASE_URL`, `SUPABASE_ANON_KEY`).

## Checks

```bash
pnpm run lint:web
pnpm run typecheck:web
pnpm run lint:cli
pnpm run test:cli
pnpm run fmt:cli
```

## Environment variables

- Shared env file: `./.env.local` (repo root), used by both Web and CLI
- Web commands (`dev/build/start`) load env in this order:
  - `./.env.local`, `./.env` (repo root)
  - `apps/web/.env.local`, `apps/web/.env` (fallback)
- CLI reads root env first, then falls back to app/home env files
- Canonical shared keys:
  - `SUPABASE_URL`
  - `SUPABASE_ANON_KEY`
- Web auto-maps these to `NEXT_PUBLIC_SUPABASE_URL` / `NEXT_PUBLIC_SUPABASE_ANON_KEY` when missing.
- CI secret for DB security checks:
  - `SUPABASE_DB_HOST` (for example `db.<project-ref>.supabase.co`)
  - `SUPABASE_DB_USER` (for example `postgres`)
  - `SUPABASE_DB_PASSWORD` (database password)
  - Optional: `SUPABASE_DB_PORT` (defaults to `5432`)
  - Optional: `SUPABASE_DB_NAME` (defaults to `postgres`)

## CI

- `web-ci`: path-scoped workflow for `apps/web/**`
- `cli-ci`: path-scoped workflow for `apps/cli/**`
- `cli-release`: independent CLI release workflow (manual or push tag `cli-v*`)
- `db-security-check`: validates Supabase RLS/policies/privileges for memo isolation (`supabase/**`)

## Release docs

- CLI release guide: `docs/cli-release.md`
