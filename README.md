# cap-monorepo

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

Note: `build:web` requires Supabase env values (`NEXT_PUBLIC_SUPABASE_URL`, `NEXT_PUBLIC_SUPABASE_ANON_KEY`).

## Checks

```bash
pnpm run lint:web
pnpm run typecheck:web
pnpm run lint:cli
pnpm run test:cli
pnpm run fmt:cli
```

## Environment variables

- Web env files stay under `apps/web` (`.env.local`, `.env*`)
- CLI reads `SUPABASE_URL` and `SUPABASE_ANON_KEY` from shell or env files (`apps/cli/.env*` and `~/.capmind/.env*`)

## CI

- `web-ci`: path-scoped workflow for `apps/web/**`
- `cli-ci`: path-scoped workflow for `apps/cli/**`
- `cli-release`: independent CLI release workflow (manual or push tag `cli-v*`)
