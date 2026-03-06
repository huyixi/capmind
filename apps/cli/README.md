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
- `Esc` in `NORMAL`: cancel pending operator/overlay only; it does not quit
- If Composer has unsaved changes and you try to quit, footer shows: `[S]ubmit+quit / [D]iscard+quit / [C]/Esc continue`
- If quit-submit fails after retries, text is cached locally and will be retried next launch
- `NORMAL` mode navigation/edit keys: arrows, `h`/`j`/`k`/`l`, `b`, `0`, `$`, `i`/`a`/`I`/`A`/`o`/`O`, `x`, `dd`
- `NORMAL` mode prefixed commands use `:`:
  - `:w`/`:s`: submit and stay
  - `:wq`: submit in background (up to 3 attempts, retry delays `1s`, `3s`) and quit on success
  - `:q`: quit (if unsaved, enter submit/discard confirmation)
  - `:q!`/`:Q`: force quit
  - `:W`/`:!`: legacy aliases for `:wq`/`:q!`
  - `:l`: open full-page memo list
  - `:?`: open help popup (`?` / `Esc` / `q` to close)
  - `ZZ`: save+quit (same as `:wq`)
  - `ZQ`: force quit (same as `:q!`/`:Q`)
- `↑` / `k` (in History): move selection up
- `↓` / `j` (in History): move selection down
- `Enter` (in History): load selected memo into Composer for edit mode
- `q` (in History): quit TUI
- `r` (in History): refresh memo list
- `Ctrl+C`: quit TUI immediately
- `d` (in History): open delete confirmation for selected memo
- `Enter` / `y` / `d` (in delete confirmation): confirm delete
- `n` / `Esc` (in delete confirmation): cancel delete
- Memo list page: `j`/`k` or `↑`/`↓` move, `Ctrl+f`/`PageDown` next page, `PageUp` previous page, `:n`/`:p` next/previous page, `/` enters search, search is case-insensitive contains, `Enter` applies search / loads selected memo (outside search mode), `Esc` clears search (in search mode) or returns to composer page, `y` copies selected memo text, `r` refreshes memo list, `d` opens delete confirmation, `:c` returns to composer page, `:q` quits program, `?` opens help popup
- Paste in Composer `INSERT` mode is supported via bracketed paste. If your clipboard tool pastes image placeholders as text (for example `[image1]`, `[image2]`), they will be inserted directly into the composer.

Composer page starts in single-pane mode (no history pane shown).

All non-deleted memos are loaded into history in the background after startup (non-blocking).
Image-only memos are shown as `[Image-only memo]` in list/history views.
Submitting from History edit mode updates the original memo by version.
On version conflict, CLI follows Web behavior: conflict is resolved by backend RPC, keeping server-latest memo and forking your edits into a new memo.
Deleting from History is a soft delete (`deleted_at` + version bump), aligned with Web behavior.
On delete conflict, CLI resolves via backend RPC and refreshes that memo from server state instead of hard-removing it.
If `:wq` (or `:W`) fails after the final retry, the UI prompts to either quit without submit or continue editing.

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

### 9) Diagnose local install/session/cache state

Run doctor in text mode:

```bash
cargo run -- doctor
```

Run doctor in JSON mode:

```bash
cargo run -- doctor --json
```

`doctor` is read-only and reports:
- install source and current version detection
- latest release tag lookup status
- Homebrew availability/formula status
- session file and cache file health
- actionable findings for common upgrade/auth/cache issues

## Auth/session behavior

- CLI first attempts refresh-token login from `~/.capmind/auth.json`.
- If refresh fails or missing, CLI asks you to run `cap login`.
- `cap compose`/`cap list` open TUI immediately, then auth/history load runs in background.
- If save is attempted while auth is missing/expired, composer footer shows:
  - `Publish requires login. [L]ogin now  [S]ave draft  [C]ancel`
  - `L`: run interactive login flow and retry submit
  - `S`: keep current text as draft in editor (no publish)
  - `C`/`Esc`: cancel prompt and continue editing
- Successful login stores session data to `~/.capmind/auth.json`:
  `refresh_token`, `access_token`, `access_token_expires_at`, `user_id`.
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

## Release automation (maintainers)

- CLI releases are automated by GitHub Actions on `main` for CLI-related changes.
- Versioning/changelog are managed by release-please from commit history.
- Use Conventional Commits for intended version bumps:
  - `fix:` -> patch release
  - `feat:` -> minor release
  - `feat!:` or `BREAKING CHANGE:` -> major release
- Published GitHub Releases must include `capmind-Linux`, `capmind-macOS`,
  `capmind-Windows.exe`, and `SHA256SUMS` for `cap update`.
