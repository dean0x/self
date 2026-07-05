# self

A self-learning & self-improving layer for coding agents (Claude Code, Codex).
Fully ambient: two background subagents — a **learner** and an **improver** —
dispatched at session start, learning procedures as native skills. Everything of
substance is in **[spec.md](spec.md)**; read that first.

## Layout

- `spec.md` — the system spec. §13.1 is the executor-ready M1 runbook.
- `templates/` — canonical install artifacts (M1: plain markdown; M2 turns them
  into `.mds` sources compiled into the `self` CLI):
  - `preamble.md` — marker block installed into `~/.claude/CLAUDE.md` (M3: `~/.codex/AGENTS.md`)
  - `constitution.md` — invariants, installed to `~/.self/constitution.md`
  - `agents/` — background agent definitions (`self-learner`, `self-improver`)
  - `seed/` — initial `~/.self` corpus, including the `ci-gate` seed skill (S-0001)

The installed instance lives at `~/.self` (git-tracked, agent-owned) — never
inside this repo.

## Status

M0 spec ✅ · **M1 manual pilot — ready to execute, see spec §13.1** (includes a
copy-paste `/goal` invocation) · M2 `self` CLI · M3 Codex adapter · M4
evidence-driven extensions.
