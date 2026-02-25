# CLI Release Guide

This document describes how to release `apps/cli` in this monorepo.

## Version and tag format

- Use semantic versioning: `MAJOR.MINOR.PATCH`
- Git tag format must be: `cli-v<version>`
- Example: `cli-v0.2.1`

Only tags that match `cli-v*` trigger GitHub Release publishing.

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
3. Commit version changes.
4. Create and push a tag:

```bash
git tag cli-v0.2.1
git push origin cli-v0.2.1
```

## What the workflow does

Workflow file: `.github/workflows/cli-release.yml`

- Builds on `ubuntu-latest`, `macos-latest`, and `windows-latest`
- Produces binaries:
  - `cap-cli-Linux`
  - `cap-cli-macOS`
  - `cap-cli-Windows.exe`
- On tag trigger (`cli-v*`), publishes a GitHub Release with generated notes
- Generates and uploads `SHA256SUMS` for all release binaries

## Manual trigger behavior

`workflow_dispatch` runs build and artifact upload only.
It does **not** publish a GitHub Release because publish job is tag-gated.

## Rollback / hotfix

1. Fix the issue in a new commit.
2. Bump patch version (for example `0.2.1` -> `0.2.2`).
3. Tag and push `cli-v0.2.2`.
4. Mark the broken release as deprecated in GitHub Release notes.

Do not reuse or move an existing release tag.

## Verify checksums after publish

```bash
gh release download cli-v0.2.1 -p "cap-cli-*" -p "SHA256SUMS"
sha256sum -c SHA256SUMS
```

On macOS (if `sha256sum` is unavailable):

```bash
shasum -a 256 -c SHA256SUMS
```
