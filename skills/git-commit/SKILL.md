---
name: git-commit
description: Git commit workflow for this monorepo: inspect local changes, split mixed edits into logical commits, stage only intended hunks, draft Conventional Commit messages, and execute safe commits. Use when Codex is asked to commit changes, clean up commit history quality, or prepare commit messages from an existing diff.
---

# Git Commit

Create small, reviewable commits with clear intent and no unrelated noise.
Prefer multiple focused commits over one mixed commit.

## Workflow

1. Inspect repository state.

```bash
git status --short
git diff --name-only
git diff --cached --name-only
```

2. Build a commit plan before staging.
- Group files by one intent per commit (`feat`, `fix`, `refactor`, `docs`, `test`, `chore`).
- Split unrelated edits into separate commits.
- Exclude generated or incidental edits unless the user asked to include them.

3. Stage only intended content.

```bash
git add -p <path>
```

When the full file belongs to the same intent:

```bash
git add <path>
```

4. Verify staged content matches only this commit's intent.

```bash
git diff --cached
git status --short
```

5. Draft a Conventional Commit message.
- Preferred format: `<type>(<scope>): <summary>`
- If scope is unclear: `<type>: <summary>`
- Keep subject concise and specific.
- Use imperative summary text.

Scope hints for this monorepo:
- `apps/cli/**` -> `cli`
- `apps/web/**` -> `web`
- `skills/**` -> `skills`
- cross-cutting root changes -> `main` or omit scope

Examples:
- `feat(cli): add vim mode keymap`
- `fix(web): handle empty workspace state`
- `chore(skills): add git commit skill`

6. Commit staged changes.

```bash
git commit -m "<type>(<scope>): <summary>"
```

When signing is required:

```bash
git commit -S -m "<type>(<scope>): <summary>"
```

7. Validate commit result.

```bash
git show --stat --oneline -1
git status --short
```

## Safety Rules

- Never commit unrelated files just to clean the tree.
- Never use `git commit -a` when the tree contains mixed changes.
- Keep generated artifacts and source changes in separate commits when practical.
- Do not amend, rebase, or force-push history unless explicitly requested.

## Quick Recipes

Single-file commit:

```bash
git add path/to/file
git commit -m "fix(scope): concise summary"
```

Partial commit from one file:

```bash
git add -p path/to/file
git commit -m "refactor(scope): concise summary"
```

Undo wrong staging:

```bash
git restore --staged <path>
git add -p <path>
```
