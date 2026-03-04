---
name: cli-release
description: Release and hotfix workflow for the capmind Rust CLI in this monorepo, including version bumping, signed commit/tag creation, GitHub Release publishing expectations, checksum verification, and Homebrew tap sync constraints. Use when preparing, validating, or troubleshooting any `capmind-v*` release for `apps/cli`.
---

# CLI Release

## Overview

Execute a safe, repeatable release process for `apps/cli` with signed tags.
Follow this workflow whenever the task involves shipping or fixing a CLI release.

## Required Inputs

- Read `apps/cli/Cargo.toml` for current version.
- Confirm target branch, target version, and whether this is standard release or hotfix.
- Ensure GPG signing is available on the machine before creating release commit/tag.

## Workflow

1. Validate repository state
- Ensure working tree is clean or confirm intentional local changes.
- Confirm release tag does not already exist.

2. Run pre-release checks from repo root
- Run `pnpm run lint:cli`
- Run `pnpm run test:cli`
- Run `pnpm run fmt:cli`
- Optionally run `pnpm run build:cli`

3. Bump version and create signed release commit
- Update `apps/cli/Cargo.toml` to target version.
- Stage only intended release files.
- Create signed commit with message format: `chore(cli): release vX.Y.Z`.

4. Create and verify signed annotated tag
- Use tag format exactly: `capmind-vX.Y.Z`.
- Create signed annotated tag; do not use lightweight tags.
- Verify signature before push.

5. Push commit and tag
- Push branch head first, then push the tag.
- Confirm CI workflows are triggered.

6. Verify release outputs
- Confirm binaries and `SHA256SUMS` are present in GitHub Release artifacts.
- Download artifacts and validate checksums.

## Decision Rules

- Always use semantic versioning `MAJOR.MINOR.PATCH`.
- Always use `capmind-v*` tags for publish behavior.
- Never reuse, move, or force-update an existing release tag.
- Treat workflow behavior as source of truth:
  - `.github/workflows/cli-release.yml`
  - `.github/workflows/cli-homebrew-tap-sync.yml`
- If release is broken, ship new patch version instead of mutating prior tag.

## Commands Template

Use this sequence with version substituted:

```bash
git add apps/cli/Cargo.toml
git commit -S -m "chore(cli): release vX.Y.Z"
git tag -s capmind-vX.Y.Z -m "capmind-vX.Y.Z"
git tag -v capmind-vX.Y.Z
git push origin HEAD
git push origin capmind-vX.Y.Z
```

Checksum validation:

```bash
gh release download capmind-vX.Y.Z -p "capmind-*" -p "SHA256SUMS"
sha256sum -c SHA256SUMS
```

macOS fallback:

```bash
shasum -a 256 -c SHA256SUMS
```

## Reference

For full rationale, workflow notes, signing setup, and rollback rules, read:
- `references/cli-release.md`
