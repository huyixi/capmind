---
name: git-worktree
description: Git worktree workflow for this monorepo: create, attach, sync, and clean worktrees under `.worktree/`, normalize branch-to-directory names (`feat/cli/vim` -> `feat-cli-vim`), and use `origin/main` as the default base for new branches. Use when Codex needs to manage branch-isolated work directories safely.
---

# Git Worktree

Use this skill to manage branch-isolated work directories with one convention:
- Keep branch names unchanged.
- Store worktrees in `.worktree/`.
- Map folder names with slash-to-dash normalization.

Example mapping:
- `feat/cli/vim` -> `.worktree/feat-cli-vim`

## Inputs

- `branch` (required): target branch name
- `base_ref` (optional): defaults to `origin/main`

## Rules

1. Run from the repo root worktree (usually `main/` in this project).
2. Always start with:

```bash
git fetch origin --prune
mkdir -p .worktree
```

3. Compute target directory with:

```bash
worktree_name="${branch//\//-}"
worktree_dir=".worktree/${worktree_name}"
```

4. If `worktree_dir` already exists and is not reusable, append numeric suffixes (`-1`, `-2`, ...).
5. Never rename Git branches to match folder names.

## Workflow

1. Prepare variables.

```bash
branch="feat/cli/vim"
base_ref="${base_ref:-origin/main}"
worktree_name="${branch//\//-}"
worktree_dir=".worktree/${worktree_name}"
```

2. Create or attach worktree.

If local branch exists:

```bash
git show-ref --verify --quiet "refs/heads/${branch}" \
  && git worktree add "${worktree_dir}" "${branch}"
```

If local branch does not exist:

```bash
git show-ref --verify --quiet "refs/heads/${branch}" \
  || git worktree add "${worktree_dir}" -b "${branch}" "${base_ref}"
```

3. Sync inside target worktree.

```bash
git fetch origin --prune
git rebase origin/main
```

Use merge only when explicitly requested by user or team policy.

4. Cleanup with confirmation-first flow.

Preview merged branch candidates:

```bash
for branch in $(git branch --format='%(refname:short)' --merged origin/main); do
  case "$branch" in
    main|master|develop) continue ;;
  esac
  echo "${branch} -> .worktree/${branch//\//-}"
done
```

After explicit confirmation, remove each candidate in order:

```bash
branch="feat/cli/vim"
git worktree remove ".worktree/${branch//\//-}"
git branch -d "${branch}"
git worktree prune
```

## Safety

- Do not run cleanup while inside the target worktree path.
- Do not use force options by default (`git worktree remove --force`, `git branch -D`).
- Require explicit user instruction before any force deletion.

## Troubleshooting

- `fatal: 'X' is already checked out at ...`:
  Reuse that existing worktree path or remove it first.
- `fatal: path already exists`:
  Resolve folder collision with numeric suffix.
- `error: Cannot delete branch ... checked out at ...`:
  Remove linked worktree first, then delete branch.
- Removal blocked by local changes:
  Commit or stash changes before removing worktree.
