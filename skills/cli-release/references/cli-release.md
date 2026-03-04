# Capmind CLI Release Reference

This reference captures the canonical release behavior for `apps/cli`.
Use it for command details and release policy decisions.

## Version and Tag Rules

- Use semantic versioning: `MAJOR.MINOR.PATCH`.
- Use release tag format: `capmind-v<version>`.
- Only `capmind-v*` tags trigger GitHub Release publishing.
- Do not use lightweight tags for releases.

## Signing Setup (Per Machine, Once)

1. Generate a key:

```bash
gpg --full-generate-key
```

2. List secret keys and copy key ID:

```bash
gpg --list-secret-keys --keyid-format=long
```

3. Configure Git signing:

```bash
git config --global user.signingkey <KEY_ID>
git config --global commit.gpgsign true
git config --global tag.gpgsign true
```

4. Export public key for GitHub:

```bash
gpg --armor --export <KEY_ID>
```

## Pre-release Checks

Run from repo root:

```bash
pnpm run lint:cli
pnpm run test:cli
pnpm run fmt:cli
```

Optional build check:

```bash
pnpm run build:cli
```

## Standard Release Steps

1. Ensure branch is correct and up to date.
2. Update CLI version in `apps/cli/Cargo.toml`.
3. Commit version changes with signature.
4. Create signed annotated tag and verify signature.
5. Push commit and tag.

Commands:

```bash
git add apps/cli/Cargo.toml
git commit -S -m "chore(cli): release vX.Y.Z"
git tag -s capmind-vX.Y.Z -m "capmind-vX.Y.Z"
git tag -v capmind-vX.Y.Z
git push origin HEAD
git push origin capmind-vX.Y.Z
```

## Workflow Behavior

Workflow: `.github/workflows/cli-release.yml`

- Builds on `ubuntu-latest`, `macos-latest`, `windows-latest`
- Produces:
  - `capmind-Linux`
  - `capmind-macOS`
  - `capmind-Windows.exe`
- On tag `capmind-v*`, publishes GitHub Release with generated notes
- Uploads `SHA256SUMS` for binaries

Workflow: `.github/workflows/cli-homebrew-tap-sync.yml`

- Triggers:
  - `push` with tag `capmind-v*`
  - `release.published`
  - `workflow_dispatch`
- Downloads release artifacts, computes SHA-256
- Updates Homebrew formula `version`, `url`, `sha256` in tap repo

Required repository settings:

- Secret `HOMEBREW_TAP_TOKEN`: token with write access to tap repo
- Variable `HOMEBREW_TAP_REPO`: example `huyixi/homebrew-tap`
- Variable `HOMEBREW_FORMULA_PATH`: example `Formula/capmind.rb`

## Manual Trigger Notes

- `cli-release` workflow with `workflow_dispatch` runs build + artifact upload only.
- It does not publish GitHub Release because publish job is tag-gated.
- Tap sync can still be run manually via `workflow_dispatch`.

## Rollback and Hotfix

1. Fix issue in a new commit.
2. Bump patch version (for example `0.2.1` to `0.2.2`).
3. Create and push new signed tag `capmind-v0.2.2`.
4. Mark broken release as deprecated in GitHub Release notes.

Never reuse or move an existing release tag.

## Checksum Verification After Publish

```bash
gh release download capmind-vX.Y.Z -p "capmind-*" -p "SHA256SUMS"
sha256sum -c SHA256SUMS
```

macOS fallback:

```bash
shasum -a 256 -c SHA256SUMS
```
