# CLI Release Guide

This document describes how to release `apps/cli` in this monorepo.

## Version and tag format

- Use semantic versioning: `MAJOR.MINOR.PATCH`
- Git tag format must be: `cli-v<version>`
- Example: `cli-v0.2.1`

Only tags that match `cli-v*` trigger GitHub Release publishing.

## Signing setup (required once per machine)

GitHub Release "Verified" depends on commit/tag signatures that GitHub can verify.

1. Generate a signing key:

```bash
gpg --full-generate-key
```

2. Get your key ID:

```bash
gpg --list-secret-keys --keyid-format=long
```

3. Configure Git to sign commits and tags by default (`<KEY_ID>` example: `B5690EEEBB952194`):

```bash
git config --global user.signingkey <KEY_ID>
git config --global commit.gpgsign true
git config --global tag.gpgsign true
```

4. Export your public key and add it in GitHub Settings -> SSH and GPG keys:

```bash
gpg --armor --export <KEY_ID>
```

## Pre-release checks

Run checks from repo root:

```bash
pnpm run lint:cli
pnpm run test:cli
pnpm run fmt:cli
```

Optional local release build:

```bash
pnpm run build:cli
```

## Create a release

1. Ensure you are on the target branch and up to date.
2. Update CLI version in `apps/cli/Cargo.toml` if needed.
3. Commit version changes with a signature.
4. Create a signed annotated tag and verify it.
5. Push commit and tag:

```bash
git add apps/cli/Cargo.toml
git commit -S -m "chore(cli): release v0.2.1"
git tag -s cli-v0.2.1 -m "cli-v0.2.1"
git tag -v cli-v0.2.1
git push origin HEAD
git push origin cli-v0.2.1
```

Do not use lightweight tags (for example `git tag cli-v0.2.1`) for releases.

## What the workflow does

Workflow file: `.github/workflows/cli-release.yml`

- Builds on `ubuntu-latest`, `macos-latest`, and `windows-latest`
- Produces binaries:
  - `cap-Linux`
  - `cap-macOS`
  - `cap-Windows.exe`
- On tag trigger (`cli-v*`), publishes a GitHub Release with generated notes
- Generates and uploads `SHA256SUMS` for all release binaries

Homebrew tap sync workflow: `.github/workflows/cli-homebrew-tap-sync.yml`

- Trigger:
  - `push` tag `cli-v*` (primary trigger)
  - `release.published` (compatible trigger)
  - `workflow_dispatch` (manual)
- Downloads release assets (`cap-*`) and computes SHA-256
- Clones tap repo and updates formula `version`, `url`, and `sha256`
- Commits and pushes formula update to tap repo

Required repository settings in this repo:

- Secret: `HOMEBREW_TAP_TOKEN`
  - Personal access token with write access to the tap repo
- Variable: `HOMEBREW_TAP_REPO`
  - Example: `huyixi/homebrew-tap`
- Variable: `HOMEBREW_FORMULA_PATH`
  - Example: `Formula/cap.rb`

## Manual trigger behavior

For `cli-release` workflow, `workflow_dispatch` runs build and artifact upload only.
It does **not** publish a GitHub Release because publish job is tag-gated.

You can run the tap sync workflow manually via `workflow_dispatch` when needed.

## Rollback / hotfix

1. Fix the issue in a new commit.
2. Bump patch version (for example `0.2.1` -> `0.2.2`).
3. Create and push a signed tag `cli-v0.2.2`.
4. Mark the broken release as deprecated in GitHub Release notes.

Do not reuse or move an existing release tag.

## Verify checksums after publish

```bash
gh release download cli-v0.2.1 -p "cap-*" -p "SHA256SUMS"
sha256sum -c SHA256SUMS
```

On macOS (if `sha256sum` is unavailable):

```bash
shasum -a 256 -c SHA256SUMS
```
