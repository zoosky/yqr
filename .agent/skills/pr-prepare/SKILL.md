---
name: pr-prepare
description: Prepare a pull request with quality checks, commit, and PR creation. Use when ready to submit changes for review.
---

# Pull Request Preparation

Complete workflow for preparing and submitting a PR.

## Pre-flight Checks

Run all quality gates:

```bash
cargo fmt && cargo clippy -- -D warnings && cargo test && cargo bench
```

## Create Feature Branch (if needed)

```bash
# Create and switch to feature branch
git checkout -b feature/your-feature-name

# Or for bug fixes
git checkout -b fix/bug-description
```

## Commit Changes

```bash
# Stage changes
git add .

# Commit with descriptive message
git commit -m "Add feature X

- Implemented Y
- Fixed Z
- Updated tests"
```

## Push and Create PR

```bash
# Push to remote
git push -u origin HEAD

# Create PR using GitHub CLI
gh pr create --title "Your PR title" --body "## Summary
- What this PR does

## Test Plan
- How to test the changes

## Checklist
- [ ] Tests pass
- [ ] Clippy clean
- [ ] Documentation updated"
```

## PR Checklist

Before submitting:
- [ ] `cargo fmt` - code formatted
- [ ] `cargo clippy -- -D warnings` - no warnings
- [ ] `cargo test` - all tests pass
- [ ] `cargo bench` - benchmarks pass
- [ ] Commit messages are clear
- [ ] PR description explains changes

## After PR Creation

- CI will run automatically
- Automated review runs if configured
- Address any feedback
- Squash and merge when approved
