# Repository Guidelines

## Core Performance Principle
- `memo-composer` startup speed is the top product priority: users must be able to focus the composer and start typing a memo immediately.
- Before any code change, evaluate whether it could slow down `memo-composer` mount time, input responsiveness, or first-keystroke latency.
- Reject or redesign any change that risks degrading `memo-composer` fast-start and fast-input behavior.

## Project Structure & Module Organization
- `app/` holds the Next.js App Router routes, layouts, and route handlers (see `app/api/`).
- `components/` contains shared React components; most UI primitives live under `components/ui/`.
- `hooks/` stores reusable React hooks.
- `lib/` contains domain logic (memos, utilities) and Supabase clients (`lib/supabase/`).
- `public/` is for static assets served by Next.js.
- `scripts/` includes one-off Node utilities (run with `node scripts/<file>.js`).

## App Function List (Routes)
- `/`: Memo list and composer UI (see `app/page.tsx` and `components/memo-container.tsx`).
- `/login` and `/signup`: Auth screens under `app/(auth)/`.
- `/auth/callback`: Supabase auth callback handler (`app/(auth)/callback/route.ts`).
- `/offline`: Offline fallback page (`app/offline/page.tsx`).
- `/api/memos`: API route for memo data (`app/api/memos/route.ts`).

## Key App Functions
- Supabase auth flow (login/signup + callback) with profile display name updates.
- Memo create/edit/delete/restore with optimistic updates.
- Offline-first memo workflow is critical: users must be able to start editing instantly, edit while offline, submit while offline (queued), and sync reliably after reconnecting.
- Image attachments and previews in memos.
- Search dialog with recent history and keyboard shortcuts.
- Trash view + manual refresh/sync.
- Export or copy memos by range (day/week/all).
- Offline queue + PWA update notice.

## Build, Test, and Development Commands
- `pnpm dev`: Run the dev server on `http://localhost:3000`.
- `pnpm build`: Build the production bundle.
- `pnpm start`: Serve the production build locally.
- `pnpm lint`: Run ESLint across the repo.

## Coding Style & Naming Conventions
- TypeScript + React (App Router). Use the `@/` path alias for local imports (configured in `tsconfig.json`).
- Follow existing formatting: TS/TSX uses semicolons, double quotes, and trailing commas.
- Prefer descriptive component names in `PascalCase`, hooks in `useCamelCase`, and constants in `SCREAMING_SNAKE_CASE`.
- Styling is Tailwind-first; global styles and tokens live in `app/globals.css`.

## Testing Guidelines
- No automated test runner is configured yet.
- If you add tests, use `*.test.ts`/`*.test.tsx` naming and place them near the code or in `__tests__/`.
- Document any new test command in `package.json` scripts.

## Commit & Pull Request Guidelines
- Commit history favors short, lowercase subjects; optional type prefixes are common (e.g., `fix:`, `refactor:`, `ui:`).
- Keep commits focused on a single change set.
- PRs should include a concise summary, testing notes, and screenshots/GIFs for UI changes.
- Call out any required environment variables (see `.env.local`) or data migrations in the PR description.

## Configuration & Security Notes
- Local secrets live in `.env.local`; never commit real credentials.
- Supabase keys are required for auth/data access (`NEXT_PUBLIC_SUPABASE_URL`, `NEXT_PUBLIC_SUPABASE_ANON_KEY`).
