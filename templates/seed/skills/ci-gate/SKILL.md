---
name: ci-gate
description: Use when merging or squash-merging any branch into main, or pushing to main — verify CI is green first.
---

# ci-gate

When a merge to main is requested:

1. Check CI on the PR or branch: `gh pr checks` (or `gh run list --branch <branch>`).
2. Run the repo's formatting/linting gates locally when it defines them (e.g.
   `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`).
3. If CI is failing: fix the failure before merging. `--admin` bypasses branch
   protection rules, not this gate — use it only when explicitly requested and the
   failure is understood.
4. Merge only when green.

## Why

The user required this across independent sessions and PR types (obs-0001,
obs-0002); merging on red cost a revert cycle. Migrated from the pre-system
command `self-learning/ci-gate-before-merge` as seed learned skill S-0001.
