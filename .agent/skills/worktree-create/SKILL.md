---
name: worktree-create
description: Create a new git worktree for parallel agent development. Use when starting work on a new feature or fix.
---

# Create Git Worktree

Create a new git worktree for parallel development, allowing multiple agents to work on different branches simultaneously.

## Usage

Run from the repository root:

```bash
scripts/worktree-create.sh <branch-name> [base-branch]
```

## Arguments

- `branch-name` (required): Name for the new branch (e.g., `feature/my-feature`, `fix/bug-123`)
- `base-branch` (optional): Branch to base off of (default: `master`)

## Examples

```bash
# Create worktree for a new feature based on master
scripts/worktree-create.sh feature/add-auth

# Create worktree based on a different branch
scripts/worktree-create.sh fix/bug-123 develop
```

## What It Does

1. Fetches latest from origin
2. Creates a new directory at `../accent-<branch-name>/`
3. Creates a new branch from the base branch (or uses existing branch)
4. Sets up the worktree ready for development

## Output Location

Worktrees are created in the parent directory with the naming pattern:
`../accent-<branch-name>/` (slashes in branch names become dashes)

## When to Use

- Starting work on a new feature or fix
- When multiple agents need to work in parallel
- To isolate changes from the main working directory
