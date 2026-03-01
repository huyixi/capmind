# Repository Guidelines

## Scope and Priority
- This file applies to the entire monorepo.
- If a nested `AGENTS.md` exists, follow the more specific file for that subtree.
- For work in `apps/web`, also follow `apps/web/AGENTS.md` (includes critical `memo-composer` performance requirements).

## Path-Based Edit Scope Policy
- Determine edit scope from current working directory (`cwd`) using this exact precedence:
  - If `cwd` contains `/multi/`, mode is `multi-app`.
  - Else if `cwd` contains `/cli/`, scope is `apps/cli/**`.
  - Else if `cwd` contains `/web/`, scope is `apps/web/**`.
  - Else if `cwd` contains `/apple/`, scope is `apps/apple/**`.
  - Else scope is `unknown` and the agent must ask the user to confirm scope before editing.
- In single-app scope (`/cli/`, `/web/`, `/apple/`):
  - Edit only files inside the mapped app directory.
  - Do not edit other apps.
  - Treat root/shared paths (for example repo root files, `supabase/**`, `docs/**`) as blocked by default.
  - If a shared/root edit is required, ask for user confirmation first with explicit file list and reason, then proceed only after confirmation.
- In `multi-app` scope (`*/multi/*`):
  - Cross-app edits are allowed across `apps/cli/**`, `apps/web/**`, and `apps/apple/**`.
  - Shared/root edits are allowed when relevant to the task; include a brief rationale in the response.

## Monorepo Layout
- `apps/web`: Next.js web app (App Router, Supabase-backed, offline-first memo flows).
- `apps/cli`: Rust CLI (`capmind` crate).
- `apps/apple`: Swift packages and tests.
- `supabase/`: SQL migrations and security checks.
- `docs/`: operational docs (for example CLI release notes).

## Prerequisites
- Node.js 22+
- `pnpm` 10+
- Rust stable toolchain
- Xcode/Swift toolchain only when touching `apps/apple`

## Environment and Secrets
- Shared env is rooted at `./.env.local`.
- Canonical shared keys:
  - `SUPABASE_URL`
  - `SUPABASE_ANON_KEY`
- Never commit real secrets. Use placeholders in docs and examples.
- Web builds require Supabase env values.

## Common Commands (Run from Repo Root)
- Install deps: `pnpm install`
- Dev:
  - `pnpm run dev:web`
  - `pnpm run dev:cli`
- Build:
  - `pnpm run build:web`
  - `pnpm run build:cli`
- Quality checks:
  - `pnpm run lint:web`
  - `pnpm run typecheck:web`
  - `pnpm run lint:cli`
  - `pnpm run test:cli`
  - `pnpm run fmt:cli`

## Code Expectations
- Enforce the path-based scope rules above for all edits.
- Prefer small, reviewable commits.
- Follow existing naming/style conventions in each app:
  - Web: TypeScript/React conventions in `apps/web`.
  - CLI: Rust formatting/clippy cleanliness in `apps/cli`.
  - Apple: Swift style consistent with neighboring files.
- Add or update tests when behavior changes.

## Database and API Safety
- Treat `supabase/migrations/*` as append-only history; do not rewrite existing migrations.
- When schema or policy behavior changes, add a new migration and document impact.
- Preserve memo data isolation and auth expectations across web/cli clients.

## Pull Request Checklist
- Explain user-visible behavior changes.
- List commands run for verification.
- Call out env vars, migrations, or release steps required by reviewers.
