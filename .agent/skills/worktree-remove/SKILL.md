---
name: worktree-remove
description: Remove a git worktree and optionally its branch. Use after merging a PR or abandoning work.
---

# Remove Git Worktree

Remove a git worktree and optionally delete the associated branch.

## Usage

Run from the repository root:

```bash
scripts/worktree-remove.sh <branch-name> [--keep-branch]
```

## Arguments

- `branch-name` (required): Name of the branch/worktree to remove
- `--keep-branch` (optional): Don't delete the local branch after removing worktree

## Examples

```bash
# Remove worktree and delete branch (if merged)
scripts/worktree-remove.sh feature/add-auth

# Remove worktree but keep the branch
scripts/worktree-remove.sh feature/add-auth --keep-branch
```

## What It Does

1. Removes the worktree directory
2. Prunes worktree references from git
3. Attempts to delete the local branch (unless `--keep-branch`)
   - Uses safe delete (`-d`) first - only works if branch is merged
   - Prompts for force delete (`-D`) if branch has unmerged changes

## When to Use

- After a PR has been merged
- When abandoning work on a feature
- To clean up unused worktrees
- After completing parallel development tasks
