# capmind

Rust CLI to insert memo text into the existing Supabase `memos` table used by `memo.huyixi.com`.

## Requirements

- Rust 1.80+ (tested with `rustc 1.93.1`)
- A valid user account for email/password sign-in

## Setup

No env setup is required. The CLI ships with built-in Supabase URL and anon key.

## Usage

### 1) Compose in TUI (default)

```bash
cargo run --
```

Open list page directly:

```bash
cargo run -- list
```

TUI keys:

- `Enter`: insert newline
- `Ctrl+Enter`: submit memo
- `Alt+Enter`: submit memo
- `Shift+Enter`: submit memo
- `Ctrl+S`: submit memo (fallback for terminals that don't emit `Ctrl+Enter`)
- Composer vim mode: starts in `INSERT`, `Esc` switches to `NORMAL`
- `Esc` in `NORMAL` (or outside Composer insert mode): press twice to quit TUI (with confirmation)
- `NORMAL` mode navigation/edit keys: arrows, `h`/`j`/`k`/`l`, `b`, `0`, `$`, `i`/`a`/`I`/`A`/`o`/`O`, `x`, `dd`
- `NORMAL` mode direct commands (no `:`):
  - `w`/`s`: submit and stay
  - `W`: submit in background (up to 3 attempts, retry delays `1s`, `3s`) and quit on success
  - `q`: quit only if no unsaved changes
  - `Q`: quit without submit
  - `l`: open full-page memo list
  - `p`: toggle split composer+history layout
  - `?`: open help popup (`?` / `Esc` / `q` to close)
- `Tab`: switch focus between Composer and History panes (only when split layout is open)
- `↑` / `k` (in History): move selection up
- `↓` / `j` (in History): move selection down
- `Enter` (in History): load selected memo into Composer for edit mode
- `q` (in History): quit TUI
- `r` (in History): refresh memo list
- `Ctrl+C`: quit TUI immediately
- `d` (in History): open delete confirmation for selected memo
- `Enter` / `y` / `d` (in delete confirmation): confirm delete
- `n` / `Esc` (in delete confirmation): cancel delete
- Memo list page: `j`/`k` or `↑`/`↓` move, `Ctrl+f`/`Ctrl+b` (or `PageDown`/`PageUp`) page down/up, `/` enters search, search is case-insensitive contains, `Enter` applies search / loads selected memo (outside search mode), `Esc` clears search (in search mode) or returns to composer page, `y` copies selected memo text, `r` refreshes memo list, `d` opens delete confirmation, `?` opens help popup

Composer page starts in single-pane mode (no history pane shown).
Use `p` in `NORMAL` mode when you want to show the split composer/history layout.

All non-deleted memos are loaded into history in the background after startup (non-blocking).
Image-only memos are shown as `[Image-only memo]` in list/history views.
Submitting from History edit mode updates the original memo by version.
On version conflict, CLI follows Web behavior: conflict is resolved by backend RPC, keeping server-latest memo and forking your edits into a new memo.
Deleting from History is a soft delete (`deleted_at` + version bump), aligned with Web behavior.
On delete conflict, CLI resolves via backend RPC and refreshes that memo from server state instead of hard-removing it.
If `W` fails after the final retry, the UI prompts to either quit without submit or continue editing.

### 2) Login (interactive, one-time)

```bash
cargo run -- login
```

Login prompts are:

- `Email:`
- `Password:`

### 3) Logout

```bash
cargo run -- logout
```

### 4) Text via `--text`

```bash
cargo run -- add --text "hello from rust cli"
```

### 5) Quick shortcut

```bash
cargo run -- "hello from shortcut"
```

This is equivalent to:

```bash
cargo run -- add --text "hello from shortcut"
```

Shortcut scope is intentionally narrow: only a single positional text argument is rewritten.

### 6) Text via stdin

```bash
echo "hello from stdin" | cargo run -- add
```

### 7) Export memos

Run interactive export:

```bash
cargo run -- export
```

`cap export` opens an interactive range selector in terminal:
- Last 3 days
- Last week
- Last month
- All memos

After selecting a range, CLI writes a file in current directory:
- Filename format: `capmind-YYYYMMDD-HHmm.txt`
- If the filename already exists, CLI appends suffix: `-1`, `-2`, ...

In non-interactive mode (for example CI/pipes), `cap export` skips prompts and
uses default range `Last 3 days`.

### 8) Update with checksum verification

Update to latest release:

```bash
cargo run -- update
```

Update to a specific version:

```bash
cargo run -- update --version 0.2.1
```

`update` downloads the platform binary and `SHA256SUMS` from GitHub Release,
verifies SHA-256 before replacing the executable, and rolls back automatically if
replacement fails.

## Auth/session behavior

- CLI first attempts refresh-token login from `~/.capmind/auth.json`.
- If refresh fails or missing, CLI asks you to run `cap login`.
- `cap compose`/`cap list`: if not logged in, CLI prompts `Press Enter to login now`.
- Successful login stores refresh token to `~/.capmind/auth.json` (field: `refresh_token`).
- Email/password are only entered interactively in `cap login`.

## Error handling

- Exit code `2`: invalid input
- Exit code `3`: auth failure
- Exit code `4`: Supabase API failure (RLS / insert errors)
- Exit code `5`: network failure

## Common troubleshooting

- `Supabase ... request failed after 3 attempts`:
  - Check internet connection and whether `https://fpeudcmnzirzjjjqtjep.supabase.co` is reachable.
- `Supabase auth failed`:
  - Verify email/password sign-in is enabled in Supabase Auth settings.
  - Login uses your account email and password (not access/refresh tokens).
- `You are not logged in`:
  - Run `cargo run -- login` once.
- `Insert memo failed (401/403...)`:
  - Check RLS policy for `memos` and ensure `user_id` matches `auth.uid()`.
- `Update failed: checksum mismatch`:
  - Ensure the release includes `SHA256SUMS` and the matching platform binary.
- `Update failed: move current binary to backup`:
  - Current executable path is not writable. Re-run with proper permissions.
