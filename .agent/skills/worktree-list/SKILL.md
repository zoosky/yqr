---
name: worktree-list
description: List all git worktrees for this repository. Use to see active parallel development environments.
---

# List Git Worktrees

Display all git worktrees associated with this repository, showing their paths and branches.

## Usage

Run from the repository root:

```bash
scripts/worktree-list.sh
```

## Output

Shows for each worktree:

- Path on disk
- HEAD commit (short hash)
- Branch name (or detached HEAD status)
- Total worktree count

## Example Output

```
Git Worktrees for yqr
===========================================

Path: /Users/dev/projects/yqr
  HEAD: abc1234
  Branch: main

Path: /Users/dev/projects/yqr-feature-auth
  HEAD: def5678
  Branch: feature/auth

-------------------------------------------
Total worktrees: 2
```

## When to Use

- Before creating a new worktree (to check existing ones)
- To find where a branch's worktree is located
- To audit active parallel development environments
- Before removing worktrees
