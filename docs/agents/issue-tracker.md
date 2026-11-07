# Issue tracker: specs/ convention

Issues for this repo live as markdown spec files under `specs/`, as defined
by ground rule 16 in `CLAUDE.md`. Never use GitHub Issues, TodoWrite, or
scratch markdown files for task tracking.

## Conventions

- Features: `specs/features/yqr-fNNN-*.md`, referenced as `yqr.fNNN` in prose
- Bugs: `specs/bugs/yqr-bNNN-*.md`
- Architecture / cross-cutting: `specs/architecture/yqr-aNNN-*.md`
- Implementation / ops: `specs/implementation/yqr-mNNN-*.md`
- Research: `specs/research/yqr-rNNN-*.md`
- Marketing: `specs/marketing/yqr-kNNN-*.md`
- Pick the next free number by listing the directory
- Every spec carries a `**Status:**` field: Draft / In Progress / Done /
  Resolved / Superseded / Historical
- Status trackers: `specs/features/yqr-f0000-feature-status.md` and
  `specs/bugs/yqr-b000-bug-status.md` must stay in sync with spec statuses
- Code traceability: `// Feature fNNN` code comments (never in `///` doc
  comments; see ground rules 13 and 19)

## When a skill says "publish to the issue tracker"

Create a new spec file in the matching `specs/` subdirectory with the next
free identifier, a `**Status:** Draft` field, and update the relevant
status tracker.

## When a skill says "fetch the relevant ticket"

Read the spec file. The user normally passes the `yqr.fNNN` / `yqr-bNNN`
identifier or the path directly; resolve identifiers by globbing the
subdirectory.

## PRs as a request surface

Off. External pull requests are not part of any triage queue.
