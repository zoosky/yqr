---
name: worktree-sync
description: Sync a worktree with the latest master branch using rebase. Use to update your branch with upstream changes.
---

# Sync Git Worktree

Sync a worktree with the latest master branch using rebase.

## Usage

Run from the repository root:

```bash
scripts/worktree-sync.sh [branch-name]
```

## Arguments

- `branch-name` (optional): Name of the branch/worktree to sync. If omitted, syncs the current directory's worktree.

## Examples

```bash
# Sync a specific worktree by branch name
scripts/worktree-sync.sh feature/add-auth

# Sync current worktree (when inside a worktree directory)
scripts/worktree-sync.sh
```

## What It Does

1. Checks for uncommitted changes (requires clean working directory)
2. Fetches latest from `origin/master`
3. Shows how many commits ahead/behind master
4. Rebases your branch on top of `origin/master`

## Conflict Handling

If rebase conflicts occur, the script will:
1. Stop and show the conflict
2. Provide instructions for resolving:
   - Fix conflicts in listed files
   - `git add <fixed-files>`
   - `git rebase --continue`
3. Or abort with `git rebase --abort`

## When to Use

- Before starting significant new work
- When master has important updates you need
- Before creating a PR (to ensure clean merge)
- Regularly during long-running feature work
