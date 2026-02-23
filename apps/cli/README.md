# cap-cli

Rust CLI to insert memo text into the existing Supabase `memos` table used by `memo.huyixi.com`.

## Requirements

- Rust 1.80+ (tested with `rustc 1.93.1`)
- Supabase project values:
  - `SUPABASE_URL` (or `NEXT_PUBLIC_SUPABASE_URL`)
  - `SUPABASE_ANON_KEY` (or `NEXT_PUBLIC_SUPABASE_ANON_KEY`)
- A valid user account for email/password sign-in

## Setup

```bash
cp .env.example .env
```

Set environment variables in your shell:

```bash
export SUPABASE_URL="https://YOUR_PROJECT.supabase.co"
export SUPABASE_ANON_KEY="YOUR_ANON_KEY"
```

The CLI auto-loads env files in this order (first found values are used):

1. `../../.env.local` (monorepo root)
2. `../../.env` (monorepo root)
3. `./.env.local`
4. `./.env`
5. `~/.capmind/.env.local`
6. `~/.capmind/.env`

## Usage

### 1) Compose in TUI (default)

```bash
cargo run --
```

TUI keys:

- `Enter`: insert newline
- `Ctrl+Enter`: submit memo
- `Shift+Enter`: submit memo
- `Ctrl+S`: submit memo (fallback for terminals that don't emit `Ctrl+Enter`)
- `Tab`: switch focus between Composer (top) and History (bottom) panes
- `↑` / `k` (in History): move selection up
- `↓` / `j` (in History): move selection down
- `Enter` (in History): load selected memo into Composer for edit mode
- `q` (in History): quit TUI
- `r` (in History): refresh memo list
- `Esc`: press twice to quit TUI (with confirmation)
- `Ctrl+C`: quit TUI immediately
- `d` (in History): open delete confirmation for selected memo
- `Enter` / `y` / `d` (in delete confirmation): confirm delete
- `n` / `Esc` (in delete confirmation): cancel delete

TUI history keeps up to the latest 100 entries.
Latest memos are loaded into history in the background after startup (non-blocking).
Submitting from History edit mode updates the original memo by version.
On version conflict, CLI follows Web behavior: keep server-latest memo and fork your edits into a new memo.
Deleting from History is a soft delete (`deleted_at` + version bump), aligned with Web behavior.
On delete conflict, CLI refreshes that memo from server state instead of hard-removing it.

### 2) Login (interactive, one-time)

```bash
cargo run -- login
```

### 3) Text via `--text`

```bash
cargo run -- add --text "hello from rust cli"
```

### 4) Quick shortcut

```bash
cargo run -- "hello from shortcut"
```

This is equivalent to:

```bash
cargo run -- add --text "hello from shortcut"
```

Shortcut scope is intentionally narrow: only a single positional text argument is rewritten.

### 5) Text via stdin

```bash
echo "hello from stdin" | cargo run -- add
```

## Auth/session behavior

- CLI first attempts refresh-token login from `~/.capmind/auth.json`.
- If refresh fails or missing, CLI asks you to run `cap login`.
- Successful login stores refresh token to `~/.capmind/auth.json` (field: `refresh_token`).
- Email/password are only entered interactively in `cap login`.

## Error handling

- Exit code `2`: invalid input or missing env
- Exit code `3`: auth failure
- Exit code `4`: Supabase API failure (RLS / insert errors)
- Exit code `5`: network failure

## Common troubleshooting

- `Missing env: SUPABASE_URL` / `SUPABASE_ANON_KEY`:
  - Set them in one of the auto-loaded env files above, or export in shell.
- `Supabase auth failed`:
  - Verify email/password sign-in is enabled in Supabase Auth settings.
- `No saved token found`:
  - Run `cargo run -- login` once.
- `Insert memo failed (401/403...)`:
  - Check RLS policy for `memos` and ensure `user_id` matches `auth.uid()`.
