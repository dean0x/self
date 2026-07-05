# The `self` constitution

Invariants of the self-learning system. Both agents obey this file; neither may
edit it. On conflict, this file overrides agent instructions.

- **C1 Procedural only.** Store only procedures: "when X happens, do Y, because
  last time Z". Facts and preferences ("the user/project is/has/prefers …") belong
  to the host tool's native memory — they are dropped, never stored here.
- **C2 Falsifiable.** Every learned skill names trigger (its description),
  procedure, and expected effect.
- **C3 Rule of two.** No promotion on a single occurrence, ever. (Maintenance
  exception: an unambiguous user correction of an existing learned skill's behavior
  is baked into that skill immediately.)
- **C4 Bounds.** Learned skills: ≤ 25 user-scope, ≤ 15 per project · description
  ≤ 25 words · body ≤ 100 lines · installed block ≤ 25 lines · open observations
  ≤ 50 · ≤ 1 transcript per learner run (≤ 3, via ≤ 3 reader subagents, in batch
  mode) · ≤ 3 observations per run · run log ≤ 200 lines. At a cap: stop and log
  `blocked`, don't squeeze.
- **C5 Null action is success.** Most runs should change nothing.
- **C6 Every run that passes the throttle logs exactly one line.** Throttled exits
  leave no trace by design; otherwise, no log line = run didn't happen.
- **C7 Removal leaves a graveyard line.** Silent deletion is forbidden.
- **C8 Marker discipline.** In files not owned by the system (`CLAUDE.md`,
  `AGENTS.md`), edit only between `<!-- self:start -->` and `<!-- self:end -->`.
- **C9 Instruction edits** (improver only): ≤ 1 per run; commit states evidence,
  change, expected metric effect; a later run verifies or reverts. This file is
  never edited by any agent.
- **C10 Single source per tool.** Agent instructions and skill bodies live only in
  each tool's native locations; the improver keeps tool variants semantically
  consistent.
- **C11 No locks.** Concurrency safety = end-of-run writes + idempotent retries +
  improver dedup. Never add locking machinery.
- **C12 LLM-first.** Agents use only standard read/write/edit/git tools on
  markdown. No helper scripts, parsers, or daemons may be created.
- **C13 In doubt → don't save.** The cost of a missed lesson is a repeat
  discovery; the cost of a bad skill is corrupted behavior in every matching
  future session.
- **C14 Ownership.** The system creates, edits, and retires only artifacts it
  authored (= registry-listed). User-authored skills, commands, and notes — and
  anything the user explicitly asked a live agent to save — are out of scope:
  never registered, never modified, never retired. They serve exactly one purpose
  here: dedup context, so the system never authors a competitor to something the
  user already has.
- **C15 Naming.** Skill names are 1–3 words (1–2 preferred), kebab-case, named
  like real human skills (`research`, `ci-gate`, `release-flow`) — the name is the
  identity; the description carries the trigger. Never sentence-like names.
- **C16 Project repos are never committed.** Project-scoped skill files are
  written and left for the user's own git flow (which doubles as review). Only
  `~/.self` is auto-committed.
