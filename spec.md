# `self` — a self-learning & self-improving layer for coding agents

**Status:** draft v0.3 · 2026-07-05
**Repo:** `~/Sandbox/self` (this spec, CLI source, factory templates)
**Installed instance:** `~/.self` (system state, owned by the agents)

---

## 1. What this is

A fully ambient continuous-learning layer for coding agents (Claude Code, Codex). At
the start of every interactive session, an injected preamble makes the main agent
launch two background subagents:

- **learner** — reads the most recent completed session transcript, decides
  (critically, defaulting to "no") whether it contains a durable *procedural* lesson,
  and audits whether previously learned skills were used, ignored, or violated.
- **improver** — reads only the system's own files, keeps the corpus structurally
  sound as it grows, and tunes the agents' instructions when run-log metrics show
  systematic failure.

What the system learns is materialized as **native skills** — the tools' own routing
mechanism. There is no second routing layer: a learned skill's `description` carries
its trigger, and the host tool decides when it fires, exactly as for any other skill.

**Automagic is a requirement, not a nicety.** The user never configures, directs, or
curates this system. If the user *explicitly* asks a coding agent to save, formalize,
or update something ("turn this into a skill", "remember to always X"), that is
ordinary coding-agent work done wherever and however the user says — it does not pass
through this system, is not registered by it, and is never touched by it (C14). This
system captures only what would otherwise be lost: lessons nobody asked to keep.

**Philosophy: LLM does everything; machinery does almost nothing.** The only binary is
a thin installer (`self` CLI). No daemons, no cron, no headless `claude -p` /
`codex exec` dispatch, no transcript-parsing scripts. All judgment, reading, writing,
maintenance, and even concurrency control are instructions executed by LLM agents over
plain markdown files. (Transcript formats are explicitly documented as unstable in
both tools — an LLM reader tolerates drift that would break parsers.)

### Non-goals

- Not a memory system. Native memory surfaces stay in charge of facts (§3).
- Not a librarian for user-authored skills: it never adopts, edits, or retires them.
- No runtime orchestration binaries, background services, or watchers.
- No guarantee any single session dispatches — the system is **eventually consistent**
  (§6); a missed dispatch is picked up by a later session.

---

## 2. Definitions

- **Observation** — a one-line candidate lesson mined from one session. Cheap,
  append-only, unproven.
- **Learned skill** — a promoted observation: a *falsifiable procedural hypothesis*
  packaged as a native skill. Name = identity (1–3 words, C15); `description` =
  trigger; body = procedure + provenance. Evidence counters live in the registry.
  Expensive — every learned skill's description sits in matching sessions' context —
  therefore capped (C4) and audited.
- **Corpus** — the system state under `~/.self/` plus the skill files it authored
  (tracked in `REGISTRY.md`, placed in native skill locations per scope, §4).
- **Installed block** — the marker-delimited preamble this system owns inside each
  tool's global instruction file.
- **Run** — one execution of learner or improver, recorded as one line in the run log.

Two governing principles:

- **Evidence gates promotion.** Nothing is promoted on a single occurrence (C3, rule
  of two); recurrence is observed, never predicted. One exception in *maintenance*:
  an unambiguous user correction, found in a transcript, of behavior a learned skill
  governs is strong evidence by itself and is baked into that skill immediately.
- **Ownership bounds action.** The system creates, edits, and retires only artifacts
  it authored (C14). Everything user-made is out of bounds — visible only as context
  so the system never builds a duplicate of something the user already has.

---

## 3. Boundary with native memory (the duplication tension, resolved)

The axis is **declarative vs procedural**:

> If it completes "the user/project **is/has/prefers** ___" → it is a fact →
> native memory's job. **Do not store it here.**
> If it completes "**when X happens, do Y**, because last time Z" → it is a
> procedure → it belongs here, and *only* here.

| Surface | Kind | Owner | `self` stance |
|---|---|---|---|
| Claude Code auto-memory (`~/.claude/projects/<slug>/memory/`) | facts, project state | Claude Code | never write; never duplicate |
| `CLAUDE.md` hierarchy / `AGENTS.md` layers | human-curated standing rules | user | write **only** inside `self` markers |
| Codex memories (`~/.codex/memories/`, opt-in, auto-generated) | preference recall | Codex | never write; if enabled, its extractions are not sources |
| devflow decisions ledger / dream agents | project decisions ("we chose X over Y") | devflow | out of scope as sources and as storage |
| User-authored skills/commands (any location) | procedures the user chose to keep | user | never touch; dedup context only (C14) |
| **Learned skills + `~/.self/`** | **procedures proven to recur, ambient-captured** | **this system** | the only thing we create |

Codex's own docs state durable rules belong in `AGENTS.md`/skills, not memories — this
system occupies exactly that sanctioned niche. A fact discovered mid-learning ("user
prefers squash merges") is *dropped*, not saved. When classification is unclear,
**don't save** (C1, C13).

---

## 4. Storage

### 4.1 System state: `~/.self/`

Git-initialized by `self init`. Every mutating agent run ends with one commit —
`git log` is the human review surface. Plain markdown throughout.

```
~/.self/
  constitution.md        # invariants (§9); agents obey it, neither may edit it
  REGISTRY.md            # catalog of learned skills: scope, path, provenance, counters (§8.3)
  observations.md        # append-only candidate ledger (§8.2)
  retired.md             # graveyard: one line per removed/expired item, with reason
  log/runs.md            # one line per completed agent run (§8.4)
```

**A fresh install is empty.** `self init` writes these five files with their headers
and nothing else: no skills, no observations, no registry entries. The system ships
no procedures and holds no opinion about how you work — everything the corpus ever
contains was mined by the learner from your own sessions and gated by the rule of two
(C3). There is no factory content to trust, tune, or delete; an empty corpus is the
correct steady state until the system has earned an entry.

Agent instructions do **not** live here. They live in each tool's native agent
location (§6) — one copy per tool, no shadow copies. The improver keeps tool variants
semantically consistent (C10).

### 4.2 Learned skill placement: scope decides location

The learner assigns scope at promotion time by asking: *would this procedure hold in
any repository, or only in the one the source session ran in?*

| Scope | Claude Code | Codex |
|---|---|---|
| **user** (this machine, all projects) | `~/.claude/skills/<slug>/SKILL.md` | `~/.agents/skills/<slug>/` (current docs) with `~/.codex/skills/` legacy fallback — version-detect at M3 |
| **project** (one repo) | `<repo>/.claude/skills/<slug>/SKILL.md` | `<repo>/.agents/skills/<slug>/` |

Scope placement is also the context-cost control: a project-scoped skill's description
loads only in that project's sessions.

Because nothing is seeded, `self init` creates no skill directory: the learner creates
the target directory itself when it writes its first promoted skill (the install only
grants the permissions for it, §10).

**Project repos are never committed by this system** (C16): a project-scoped skill is
written into the repo's skill directory and left uncommitted — it surfaces in the
user's normal git flow, which doubles as review. Only `~/.self` is auto-committed.

---

## 5. The installed block (preamble)

One marker-delimited block, identical in substance across tools, installed into:

- Claude Code: `~/.claude/CLAUDE.md` — loaded verbatim in every session of every
  project; re-read from disk after compaction.
- Codex: `~/.codex/AGENTS.md` — loaded first in every session (TUI and exec);
  concatenation stops at 32 KiB, and being first, the block survives truncation. Must
  coexist with other marker blocks already there (e.g. skim's).

**Budget: ≤ 25 lines.** The block is static — routing lives entirely in native skill
descriptions, so nothing here changes as the corpus grows, and nothing mirrors or
syncs. Canonical text: `templates/preamble.md` in the repo (embedded
into the CLI at build from M2 on):

```markdown
<!-- self:start v0.3 -->
## self — continuous learning layer

**Dispatch** — only if ALL are true: this is the first turn of the conversation
(no prior assistant turns), you are the main agent (not a subagent), and this is
an interactive user session (not an automated/headless run). Then: launch the
`SelfLearning` and `SelfImproving` subagents in the background NOW, do not wait
for or report their results, and proceed directly with the user's request.
Both agents self-throttle; launching them redundantly is cheap and safe.

**Boundary** — never write facts/preferences into ~/.self or its skills; facts
belong to your native memory. If the user explicitly asks you to save/formalize
something, do it wherever they say — that is your job, not the system's; do not
register or route it through ~/.self.
<!-- self:end -->
```

---

## 6. Dispatch: in-session, background, instruction-driven

Hard requirement: workers are launched **by the main session, inside the session** —
no `claude -p`, no `codex exec`, no hooks spawning processes.

| | Claude Code | Codex |
|---|---|---|
| Agent definition | `~/.claude/agents/self-learning.md`, `self-improvement.md` (YAML frontmatter: `name`, `description`, `tools`, `background: true`, `maxTurns`, model inherited — no `model:` override, per decision) | `~/.codex/agents/SelfLearning.toml`, `SelfImproving.toml` (`name`, `description`, `developer_instructions`; model inherited) |
| Launch mechanism | Agent/Task tool, background (default since v2.1.198); main conversation continues | native in-process subagent threads (`features.multi_agent`, default-on in current builds; `[agents] max_threads=6`) |
| Trigger | preamble instruction | preamble instruction (Codex spawns subagents only when the prompt asks — same model) |
| Lifetime | **killed if the user exits the session** | scoped to the session |

**Dispatch is model-followed, not config-guaranteed.** Neither tool has an auto-spawn
mechanism; the preamble is an instruction the model will *usually* follow. This is
accepted by design:

- A missed dispatch costs nothing — the backlog waits; the next session catches up.
- A duplicate or misfired dispatch costs almost nothing — every run starts with the
  throttle check (§7.0) and exits as a no-op.
- Dispatch health is *measurable*: the learner logs remaining backlog in each run
  line, so the improver can see coverage decaying and respond (§7.2), up to
  recommending the deterministic fallback (a SessionStart hook injecting the dispatch
  as context) — which exists, verified, but is not used until evidence demands it.

**Kill-tolerance invariant:** workers write all corpus mutations and their run-log
line at the *end* of a run (single transcript, single commit). A killed run leaves no
trace and is retried naturally next session. Worst case under concurrency: two
sessions process the same transcript → duplicate observation lines → improver dedups
(C11). This is the entire concurrency-control story — no locks, by design.

**Recursion guard:** subagents also see the global instruction files, so the dispatch
clause hard-fails for subagents ("you are the main agent") — otherwise the learner
would spawn learners. Belt-and-suspenders: the throttle check makes even a recursive
dispatch a no-op.

---

## 7. The two agents

### 7.0 Shared prologue (both agents, every run)

1. Read `~/.self/constitution.md`. It overrides these instructions on conflict.
2. Throttle: read `log/runs.md`. If a run of your type is newer than your cooldown
   (learner: **30 min**; improver: **24 h**), stop — write nothing, not even a log
   line. Throttled exits are the one run type that leaves no trace (C6 applies only
   to runs that pass the throttle). This is the expected outcome of most dispatches.
3. On finishing: append your run line to `log/runs.md`, then
   `git -C ~/.self add -A && git commit` with a one-line rationale — the run line
   rides in its own run's commit.
4. **Null action is success.** You are not judged by output volume; an empty-handed
   run that logs `no-op` is the system working correctly.

### 7.1 learner

**Inputs:** one session transcript + the corpus. **May write:** `observations.md`,
`REGISTRY.md`, `retired.md`, `log/runs.md`, and skill files *it owns* (registry-listed)
or is creating. **Must not touch:** user-authored skills (C14), agent instructions,
`constitution.md`.

Procedure (bounded: **one transcript per run** by default, newest first; batch mode
below):

1. **Select transcript.** Claude Code: `~/.claude/projects/*/*.jsonl`, excluding
   `subagents/` paths. Codex (when installed): newest
   `~/.codex/sessions/**/rollout-*.jsonl`. Eligible = file mtime idle ≥ 15 min
   (excludes live sessions, including the one that dispatched you), mtime within the
   last **7 days** (the lookback window — what makes run-log compaction safe), and
   not already in `log/runs.md` as processed. None eligible → log
   `no-op (no backlog)`.
2. **Audit learned skills first** (this ordering is deliberate — the feedback loop
   outranks new intake). For each registry entry whose trigger plausibly matched the
   session (read descriptions via one Grep over the skill paths): update its registry
   counters — `fired` +1; then `applied` +1 if followed, `invoked` +1 if explicitly
   run, or `contradicted` +1 if following it caused harm or the world changed.
   Fired-but-not-applied stays visible as the counter gap — a *routing* failure and
   the improver's signal, not a reason to edit. If the transcript shows the user
   correcting behavior a learned skill governs — spoken feedback or a silent redo —
   bake the correction into the skill body **only if unambiguous** (bump `refined`);
   ambiguous → ordinary observation referencing the skill. `contradicted ≥ 2` →
   retire immediately (§7.3, exit a).
3. **Mine at most 3 candidate observations.** Admission test — every clause must pass:
   - **Nameable trigger** — "when merging to main", not "be careful". Can't name it →
     not a lesson.
   - **Procedural** — passes the §3 boundary test. Facts are dropped, not relocated.
   - **Non-obvious** — a fresh session with existing CLAUDE.md/AGENTS.md/skills
     context would plausibly have gotten this wrong. If the model already knows it,
     skip.
   - **Not user-directed** — if the user explicitly asked for something to be saved or
     formalized in that session, the live agent already did it; not our event (C14).
   - **It cost something** — real time lost, an error made, or a user correction.
   - **Auditable** — a future transcript reader can tell whether it was applied and
     helped.
4. **Dedup before writing:** against open observations, the registry, `retired.md`,
   **and all existing skills/commands in the relevant scope locations** — the user's
   included: if the user already has a skill covering the lesson, drop it. A match
   with an open observation from a *different* session → second occurrence →
   **promote** (rule of two). A match with a `retired: stale` entry → the graveyard
   line counts as first occurrence → promote citing it. A match with
   `retired: contradicted` or `expired` → drop unless the retirement reason no longer
   holds.
5. **Promote = author a native skill:** pick the name (1–3 words, C15), write
   `description` as the trigger (≤ 25 words), body = procedure + `## Why` with source
   observation IDs; choose scope (§4.2) and write to that location; add the registry
   line; mark the observation `promoted`.
6. Housekeeping guard: if over any cap (C4), do not write; log `blocked (cap)` — the
   improver owns eviction.
7. Log the run line, including `backlog=<n idle unprocessed transcripts>`.

**Batch mode (backlog drain).** When `backlog` has exceeded 3 in two consecutive
learner runs, a Claude Code learner may process up to **3 transcripts in one run** by
fanning out **one reader subagent per transcript** (custom subagents may hold the
`Agent` tool; nesting is supported to a fixed depth of 5, and each reader gets a fresh
context — so large transcripts never blow the learner's own window). Readers only
*distill*: they return candidate lessons, trigger-match events, and observed
corrections as a compact digest; all judgment, dedup, and writes stay in the learner.
Codex learners stay sequential (`[agents] max_depth = 1` forbids nesting). Claude
Code's heavier orchestration primitives (workflow scripts, agent view, agent teams)
were considered and rejected: deterministic JS orchestration is exactly the machinery
C12 exists to avoid.

### 7.2 improver

**Inputs: system files only** — the corpus (including registry-listed skill files) and
both tools' agent instruction files. **Never reads session transcripts** (run-log
lines and counters are its only window onto the world — by construction, so its
judgments stay about the *system*, not about coding).

Jobs, in priority order (a run does the first thing that needs doing, at most — most
runs do none):

1. **Integrity:** `REGISTRY.md` ↔ skill files agree (paths exist, scopes match,
   nothing orphaned); frontmatter well-formed; every retirement has a graveyard line.
   Repair is always permitted.
2. **Caps & hygiene** (C4): over-cap → evict lowest-evidence learned skills (stale
   exit); prune observations `open` > 60 days (→ `retired.md` as `expired`); compact
   `log/runs.md` beyond 200 lines into its summary header (per-path history older
   than the learner's 7-day lookback window may be dropped safely).
3. **Consolidation:** merge near-duplicate learned skills (union of counters, both
   slugs in graveyard with `superseded-by`); split one whose audits show two distinct
   triggers; sharpen descriptions that never fire (fired=0 across many audited
   sessions); rename anything violating C15.
4. **Process tuning** — the self-improvement mandate, evidence-gated: only when
   `log/runs.md` shows a *systematic* pattern (promotion rate ~0 over 20+ runs:
   learner too strict; most skills fired-but-ignored: descriptions too vague; backlog
   growing monotonically: dispatch decaying → recommend the hook fallback). Then it
   may edit the learner's — or its own — instruction files, under C9: **at most one
   instruction edit per run**, commit message states the observed evidence, the
   change, and the **expected effect on which metric** — falsifiability applied to
   the improver itself. A later run must check whether the effect appeared and revert
   if not (git makes this one command).
5. **Structural evolution** (rare, corpus > ~15 learned skills): registry sections,
   per-scope views — structure follows scale, never precedes it.

### 7.3 Lifecycle

```
            session evidence            2nd independent occurrence
transcript ────────────────▶ observation ─────────────────────────▶ learned skill
                              (open)                              (native location
                                │ 60d unpromoted                     + registry)
                                ▼                                         │ exits:
                             retired.md ◀─────────────────────── (a) contradicted ≥ 2   [learner, immediate]
                          (one line, with reason;                (b) stale: fired+invoked = 0 across
                           stale entries can seed                    20 audited sessions or 90 days [improver]
                           re-promotion — §7.1.4)                 (c) superseded by merge [improver]
```

Retiring a learned skill deletes its skill file and registry line and writes the
graveyard line. Every exit writes that line — the system must remember what it
decided *not* to keep, or it will re-learn it forever. User-authored skills have no
lifecycle here at all (C14).

---

## 8. File formats

Human-readable, diff-friendly, no schema tooling — validation is `self doctor` + the
improver's integrity pass.

### 8.1 A learned skill (`SKILL.md` — purely native, no system frontmatter)

The example below is **hypothetical** — an illustration of the shape, not an artifact
you will find anywhere. No skill ships with the system (§4.1); every real one is
written by the learner, in your own corpus, from your own sessions.

```markdown
---
name: port-conflict
description: Use when a dev server fails to start with EADDRINUSE — reclaim the port, don't switch it.
---

# port-conflict

When a dev server won't bind because the port is already taken:
1. Identify the holder: `lsof -ti :<port>` (or `ss -lptn 'sport = :<port>'`).
2. If it's a stale process from an earlier session, kill it and retry the same port.
3. Don't route around it by changing the configured port — callbacks, proxies, and
   `.env` files hardcode the original.

## Why
Recurred in two independent sessions (obs-0007, obs-0019); switching the port broke
the OAuth callback both times and cost a debugging cycle.
```

Name: 1–3 words, like a real human skill (C15). Description: the trigger, ≤ 25 words —
most of the learner's craft goes into that line, because **a skill that doesn't fire
is dead weight regardless of quality**. Body ≤ 100 lines. All system metadata lives in
the registry, keeping the skill file indistinguishable from a hand-written one to the
host tool.

### 8.2 `observations.md` (append-only)

```markdown
- obs-0007 | 2026-07-04 | open | trigger: dev server fails with EADDRINUSE | changed the port instead of killing the stale process; broke the OAuth callback | src: ~/.claude/projects/-Users-dean-Sandbox-mdl/018f...jsonl
```

Status ∈ `open | promoted | expired`. A fresh install has no observation lines — only
the header and this format legend.

### 8.3 `REGISTRY.md` (catalog of learned skills — system-authored only)

```markdown
- S-0042 | port-conflict | user | ~/.claude/skills/port-conflict/SKILL.md | created: 2026-07-04 | src: obs-0007+obs-0019 | fired: 0 applied: 0 contradicted: 0 invoked: 0 refined: 0 | flags: -
```

Hypothetical, like §8.1 — a fresh install's `REGISTRY.md` lists **zero** skills. IDs
are assigned in promotion order, starting from the first skill the system ever learns,
and are never reused.

Scope ∈ `user | project:<repo-name>`. `flags` ∈ `- | overlap(<slug>) |
unsynced(<tool>)` — improver working notes. User-authored skills never appear here.

### 8.4 `log/runs.md` (append-only; header holds improver-written rollup summaries)

```markdown
- 2026-07-04T09:12Z | learner | tool=claude | processed=...jsonl | verdict=observed(1) audited(2) | backlog=3
- 2026-07-04T09:13Z | improver | verdict=no-op
```

Verdict vocabulary: `no-op | observed(n) | promoted(slug) | audited(n) |
refined(slug) | retired(slug,reason) | merged | tuned(file) | repaired | blocked(cap)`.
(Throttled exits write no line at all — §7.0.)

---

## 9. `constitution.md` (normative core, installed verbatim)

- **C1 Procedural only.** The §3 test decides. Facts are dropped, never stored.
- **C2 Falsifiable.** Every learned skill names trigger (description), procedure,
  and expected effect.
- **C3 Rule of two.** No promotion on a single occurrence, ever. (Maintenance
  exception: an unambiguous user correction of an existing learned skill's behavior
  is baked in immediately — §7.1.2.)
- **C4 Bounds.** Learned skills: ≤ 25 user-scope, ≤ 15 per project · description
  ≤ 25 words · body ≤ 100 lines · installed block ≤ 25 lines · open observations
  ≤ 50 · ≤ 1 transcript per learner run (≤ 3, via ≤ 3 reader subagents, in batch
  mode — §7.1) · ≤ 3 observations per run · run log ≤ 200 lines. At a cap: stop and
  log `blocked`, don't squeeze.
- **C5 Null action is success.** Most runs should change nothing.
- **C6 Every run that passes the throttle logs exactly one line.** Throttled exits
  leave no trace by design; otherwise, no log line = run didn't happen.
- **C7 Removal leaves a graveyard line.** Silent deletion is forbidden.
- **C8 Marker discipline.** In files not owned by the system (`CLAUDE.md`,
  `AGENTS.md`), edit only between `<!-- self:start -->` and `<!-- self:end -->`.
- **C9 Instruction edits** (improver only): ≤ 1 per run; commit states evidence,
  change, expected metric effect; a later run verifies or reverts. `constitution.md`
  is never edited by any agent.
- **C10 Single source per tool.** Agent instructions and skill bodies live only in
  each tool's native locations; the improver keeps tool variants semantically
  consistent.
- **C11 No locks.** Concurrency safety = end-of-run writes + idempotent retries +
  improver dedup. Never add locking machinery.
- **C12 LLM-first.** Agents use only standard read/write/edit/git tools on markdown.
  No helper scripts, parsers, or daemons may be created.
- **C13 In doubt → don't save.** The cost of a missed lesson is a repeat discovery;
  the cost of a bad skill is corrupted behavior in every matching future session.
- **C14 Ownership.** The system creates, edits, and retires only artifacts it
  authored (= registry-listed). User-authored skills, commands, and notes — and
  anything the user explicitly asked a live agent to save — are out of scope: never
  registered, never modified, never retired. They serve exactly one purpose here:
  dedup context, so the system never authors a competitor to something the user
  already has.
- **C15 Naming.** Skill names are 1–3 words (1–2 preferred), kebab-case, named like
  real human skills (`research`, `ci-gate`, `release-flow`) — the name is the
  identity; the description carries the trigger. Never sentence-like names.
- **C16 Project repos are never committed.** Project-scoped skill files are written
  and left for the user's own git flow (which doubles as review). Only `~/.self` is
  auto-committed.

---

## 10. The `self` CLI

A thin, deterministic **installer** — file plumbing only, zero runtime role, zero
judgment. Rust, single static binary, distributed via crates.io and npm (§10.1);
factory templates in `templates/` embedded at build via
`include_str!` (`.mds` authoring deferred until `mds` exists on the build machine). Runtime dependency count: zero (git assumed present).

| Command | Behavior |
|---|---|
| `self init` | Create + `git init` `~/.self`, seed factory files — the five corpus files, headers only, no skills and no observations (§4.1). Detect tools and install adapters (below). Idempotent: marker blocks replaced in place, never duplicated; existing corpus never overwritten (only `--reset` restores factory files, via a git commit first — which means `--reset` restores an *empty* corpus, discarding everything learned; the pre-reset commit is the only way back). |
| `self status` | Registry counts per scope, caps headroom, last learner/improver run, backlog trend, top learned skills by `applied`/`invoked`. |
| `self doctor` | Registry ↔ skill-file drift, marker-block integrity, frontmatter shape, permissions present, orphaned/dangling entries. Report only — repair belongs to the improver. |
| `self uninstall` | Remove marker blocks + agent definitions; leave `~/.self` and learned skills untouched (report where they live). |

Adapter actions per detected tool:

- **Claude Code** (`~/.claude` exists): block → `~/.claude/CLAUDE.md`; agent defs →
  `~/.claude/agents/{self-learning,self-improvement}.md` (`background: true`,
  `maxTurns: 50/40`, model inherited, tools: Read, Grep, Glob, Write, Edit, Bash —
  learner additionally gets `Agent` for batch-mode readers);
  merge into `~/.claude/settings.json` permissions:
  `Read(~/.claude/projects/**)`, `Read/Write/Edit(~/.self/**)`,
  `Write/Edit(~/.claude/skills/**)`, `Write/Edit(**/.claude/skills/**)`,
  `Bash(git -C ~/.self *)` — so background runs never hit a permission prompt.
- **Codex** (`~/.codex` **and** a `codex` binary — currently absent on this machine,
  so init reports "codex: skipped (not installed)"): block → `~/.codex/AGENTS.md`
  (alongside skim's markers); agent defs → `~/.codex/agents/*.toml`; skills dir
  version-detection (`~/.agents/skills` vs `~/.codex/skills`); verify
  `features.multi_agent` availability against the installed version before enabling
  dispatch wording.

### 10.1 Distribution

The binary reaches users over two registries plus raw downloads; only the crates.io
channel needs a Rust toolchain, because it compiles from source.

| Channel | Command / source | Toolchain | Platforms |
|---|---|---|---|
| **npm** | `npm install -g @dean0x/self` | none — prebuilt binary shipped | Windows, macOS, Linux (x64 + arm64) |
| **crates.io** | `cargo install self-cli` | Rust — builds from source | any Rust target |
| **GitHub Releases** | download the archive for your platform | none | the five native targets |

The npm package `@dean0x/self` carries no binary itself: it declares five
`optionalDependencies` — one per platform (`@dean0x/self-{linux,darwin}-{x64,arm64}`
and `@dean0x/self-windows-x64`) — and npm installs only the one whose `os`/`cpu`
matches the host. A tiny zero-dependency CommonJS shim resolves that package and execs
its binary. Because the crate embeds `templates/**` via `include_str!` (§10), every
channel ships the factory templates *inside* the artifact — including `cargo install`,
which compiles on the user's machine; nothing is fetched at install time.

**One version, one source of truth.** The version lives in `Cargo.toml`; the six
`package.json` files (main package + five platform packages) and their pinned
cross-references are *derived* from it. `scripts/set-version.mjs <x.y.z>` (Node,
zero dependencies) rewrites all of them in one shot — versions are never hand-edited.

**Tag-driven releases.** Pushing a `vX.Y.Z` tag triggers CI to build the five native
binaries (one runner per target — no cross-compilation), publish the crate to
crates.io and the six packages to npm (trusted publishing over OIDC — no long-lived
tokens), and attach the binaries to the GitHub release. The operator procedure —
prerequisites, the version bump, the tag push — lives in `RELEASING.md`.

---

## 11. Failure modes → defenses

| Failure | Defense |
|---|---|
| Write-only memory (stored, never used) | native description routing + audit counters + stale exit (b) |
| Context pollution / per-session tax | C4 caps + C15 tight descriptions + scope placement (project skills load only in their project) |
| Speculative generalization | C3 rule of two + nameable-trigger test |
| Re-learning removed items | graveyard consulted in dedup (§7.1.4) |
| Duplicating something the user already built | dedup context includes all skill/command locations, user-authored included (C14) |
| Touching what the user owns | C14 ownership: registry-listed artifacts only |
| Learner over-firing (saves everything) | C4 per-run caps + C5 + improver watches promotion rate |
| Instruction drift / self-lobotomy | C9: one edit per run, evidence-cited, verify-or-revert; constitution immutable; git history |
| Missed dispatch (model ignores preamble) | eventual consistency + backlog metric + hook fallback held in reserve |
| Killed mid-run (session exit) | end-of-run writes; no partial state; natural retry |
| Concurrent duplicate runs | idempotent selection + improver dedup (C11) |
| Transcript format drift | LLM reader, no parsers (C12) |
| Cross-tool inconsistency | C10 + `self doctor` drift checks |

---

## 12. Success metrics (readable off `self status` at any time)

After 4 weeks of normal use:

1. **Utility:** ≥ 1 audited application or invocation of a learned skill per week
   (`applied`/`invoked` rising) — the only metric that ultimately matters.
2. **Precision:** contradicted exits < 10% of promotions.
3. **Restraint:** ≥ 60% of logged learner runs end `no-op`; corpus comfortably
   under caps without evictions forced weekly.
4. **Liveness:** backlog not growing monotonically (dispatch is happening).
5. **Zero collisions:** nothing system-authored that fails the §3 boundary test or
   duplicates a user-authored artifact.

Metrics (1) and (2) have no subject until the learner promotes its first skill — the
corpus starts empty (§4.1), so utility is necessarily zero until then, and the clock on
(1) starts at that first promotion, not at install. This does not soften the test: if
*nothing* has been promoted by week 6, that is the same hypothesis failing one step
earlier (nothing in the user's work recurred cleanly enough to learn), and the
conclusion is unchanged.

If (1) is not met by week 6, the honest conclusion is that the hypothesis failed —
uninstall rather than tune (the graveyard of memory systems is full of tuned ones).

---

## 13. Milestones

- **M0 — this spec.**
- **M1 — manual pilot (no CLI):** install everything from `templates/` per the
  runbook in §13.1, Claude Code only. Run ~2 weeks, purely ambient, starting from an
  empty corpus. Validates dispatch rate and learner judgment — the two assumptions
  that *are* testable from nothing — before any Rust is written. Description-routing
  fire rate is the third risk, but it cannot be probed until the learner promotes a
  skill of its own (§14, open question 1); M1 may end without a verdict on it.
- **M2 — `self` CLI:** init/status/doctor/uninstall; factory defaults embedded from
  `templates/`; pilot state migrates cleanly.
- **M3 — Codex adapter:** when codex is reinstalled — AGENTS.md block, TOML agents,
  rollout-transcript reading in the learner, skills-dir version detection,
  verify `multi_agent`.
- **M4 — evidence-driven extensions,** only as improver findings demand: SessionStart
  hook fallback for dispatch; multi-machine sync via the `~/.self` git remote.

### 13.1 M1 runbook (executor-ready)

All judgment is already encoded in `templates/` — the steps below are mechanical.
An executor implements exactly one milestone per invocation; this is the next one.
Execute top to bottom; stop on any acceptance failure.

**Install** (idempotent — re-running replaces only what it installed):

1. `~/.self/`: create; copy `templates/seed/{REGISTRY,observations,retired}.md` in,
   `templates/seed/runs.md` → `log/runs.md`, `templates/constitution.md` →
   `constitution.md`; `git init` + initial commit. These are headers only — the
   install seeds no skills and no observations (§4.1), and creates no skill
   directory (§4.2).
2. Agents: `templates/agents/{self-learning,self-improvement}.md` → `~/.claude/agents/`.
3. Preamble: append `templates/preamble.md` to `~/.claude/CLAUDE.md` — only if no
   `<!-- self:start` marker exists there yet; never touch content outside markers.
4. Permissions: merge into `~/.claude/settings.json` → `permissions.allow`:
   `Read(~/.claude/projects/**)`, `Read(~/.self/**)`, `Write(~/.self/**)`,
   `Edit(~/.self/**)`, `Write(~/.claude/skills/**)`, `Edit(~/.claude/skills/**)`,
   `Write(**/.claude/skills/**)`, `Edit(**/.claude/skills/**)`,
   `Bash(git -C ~/.self *)`. Verify rule syntax against current docs; if permission
   prompts still appear during the pilot, add the expanded-path (`/Users/…`) forms.

**Acceptance — in-place (an executor verifies all of these in one session, showing
evidence in-conversation for each):**

- A1 Marker integrity: exactly one `self:start`/`self:end` pair in
  `~/.claude/CLAUDE.md`; a diff proves content outside the markers is unchanged.
- A2 Learner end-to-end: dispatch the `SelfLearning` subagent directly (Agent
  tool). It must select a real idle transcript, append exactly one run line to
  `~/.self/log/runs.md`, and commit — or log `no-op (no backlog)` if none is
  eligible. Show the run line and `git -C ~/.self log --oneline`.
- A3 Throttle: immediately dispatch `SelfLearning` again — it must leave no trace
  (no new line, no new commit; shown).
- A4 Improver end-to-end: dispatch `SelfImproving` once — exactly one run line
  (likely `no-op`) and a commit (shown).
- A5 Empty corpus, and nothing clobbered by the runs. Two halves:
  - *Install ships nothing:* `REGISTRY.md` lists no skills, `observations.md` and
    `retired.md` have no entries, and the install authored no skill file anywhere
    (`REGISTRY.md` being empty is the proof — any pre-existing skill under
    `~/.claude/skills/` is the user's and is out of scope by C14).
  - *Blast radius of A2–A4:* those runs modified nothing they did not author.
    `git -C ~/.self diff <sha-before-A2>..HEAD` contains only lines those runs wrote
    (their `log/runs.md` lines, plus any observation they mined), and
    `constitution.md` is byte-identical to `templates/constitution.md`.

  **Precondition, stated honestly:** a fresh corpus is empty, so in-place there are no
  pre-existing registry rows, observations, or skill files for a run to destroy — the
  second half is a weak check here and only becomes load-bearing once the pilot's own
  runs have populated the corpus. The full property is carried as A10.

**Acceptance — live pilot (first real sessions, over the ~2 weeks):**

- A6 Dispatch: a fresh *interactive* session in another project launches both
  subagents on turn one and proceeds without waiting. (Not testable headlessly —
  the preamble deliberately skips non-interactive runs.)
- A7 Restraint in the wild: learner passes over trivial sessions log `no-op`.
- A8 Routing: the first skill the learner promotes fires in a session matching its
  trigger (`fired` bumps on the next audit). Gated on a promotion happening at all —
  the corpus starts empty, so this cannot be checked on day one and may not be
  checkable within M1 (§14, open question 1).
- A9 The §12 metrics are computable from `runs.md` + `REGISTRY.md` alone.
- A10 Non-destruction, once the corpus is non-empty (the first promotion, or the first
  retained observation, makes this checkable): a later run leaves every corpus artifact
  it did not author byte-unchanged. On any run's commit, `git -C ~/.self show <sha>
  --stat` touches only `log/runs.md` plus artifacts that run authored or is licensed to
  edit (C14: its own registry counters, a skill it owns, an observation it mined).
  `constitution.md`, prior observations, and other skills' registry rows never appear in
  the diff. This is the A5 property with a real subject.

**Running M1 with the native `/goal` command** (v2.1.139+): the goal evaluator
judges only what the transcript shows, so the condition below names in-place
evidence only (A6–A10 verify themselves during normal use). Start a session in
this repo and paste:

```
/goal M1 of spec.md is installed and verified in-place: ~/.self exists, git-initialized and seeded from templates/ (constitution.md, REGISTRY.md, observations.md, retired.md, log/runs.md); the corpus is EMPTY as shipped — REGISTRY.md lists no skills and observations.md has no entries, shown; both templates/agents files are installed under ~/.claude/agents/; ~/.claude/CLAUDE.md contains exactly one self:start/self:end block and a diff shown in conversation proves content outside the markers is unchanged; the permission rules from spec 13.1 step 4 are present in ~/.claude/settings.json; a dispatched SelfLearning subagent appended exactly one run line to ~/.self/log/runs.md with a matching git commit, both shown; an immediate second learner dispatch produced no new line and no new commit, shown; a dispatched SelfImproving appended one run line with a commit, shown; and ~/.self/constitution.md is still byte-identical to templates/constitution.md after all three runs, shown. Or stop after 25 turns.
```

**Rollback:** remove the marker block and the two `~/.claude/agents/{self-learning,self-improvement}.md`
files; leave `~/.self` and learned skills in place (they are inert without
dispatch).

---

## 14. Open questions

1. **Description-routing reliability** — the system leans entirely on native skill
   invocation for consumption. This is the riskiest assumption in the design, and
   shipping an empty corpus makes it the *slowest* to test: there is nothing to route
   to until the learner promotes its first skill, which the rule of two (C3) puts at
   two independent occurrences of the same lesson. Routing is therefore **unmeasured
   for the opening stretch of the pilot**, and the question resolves only when a
   learned skill exists and its `fired` counter moves (A8) — possibly not within M1 at
   all. If `fired` stays near zero *once a skill exists* while its trigger demonstrably
   occurred, routing needs strengthening (sharper descriptions first; hook-injected
   reminders as a last resort). This delay is an accepted cost, not an oversight: an
   earlier verdict could only have been bought by shipping a hand-written skill, which
   would have tested that skill's description — not the learner's ability to write one.
2. **Codex dispatch semantics** — whether codex subagent threads truly run without
   blocking the parent turn on current builds; unverifiable until reinstalled.

---

## 15. Mechanics appendix (research findings this spec relies on)

**Claude Code** (docs: code.claude.com/docs — memory.md, hooks.md, sub-agents.md,
sessions.md, skills.md):
`~/.claude/CLAUDE.md` loads verbatim every session, all projects, re-injected after
compaction · custom agents in `~/.claude/agents/*.md`, background default since
v2.1.198, killed on session exit · no auto-spawn mechanism exists; dispatch is
model-followed; deterministic alternatives are @-mention / SessionStart-hook-injected
context (`additionalContext` survives compaction; hooks block startup) · transcripts:
`~/.claude/projects/<slug>/<uuid>.jsonl`, written incrementally, format explicitly
internal/unstable, subagent transcripts stored separately · subagent nesting: `Agent`
is valid in a custom agent's `tools:` frontmatter, nested spawns execute, fixed depth
limit 5 · heavier orchestration exists (workflow scripts: 16 concurrent / 1,000 agents
per run; agent view; experimental agent teams) — noted and rejected as machinery ·
auto-memory:
`~/.claude/projects/<slug>/memory/MEMORY.md`, first 200 lines/25 KB loaded · skills:
user `~/.claude/skills/`, project `./.claude/skills/`; descriptions always in context
(truncated at 1,536 chars), bodies load on demand.

**Codex** (docs: developers.openai.com/codex — agents-md, config-reference, hooks,
subagents, skills, memories; repo `codex-rs` protocol/rollout source; local ground
truth = v0.128-era residue, **binary currently not installed**):
`~/.codex/AGENTS.md` loads first, then root-down concatenation, 32 KiB cap
(`project_doc_max_bytes`) · native in-process subagents (`features.multi_agent`,
default-on in current builds; `[agents] max_threads=6, max_depth=1`,
`job_max_runtime_seconds=1800`), spawned only on explicit prompt instruction; custom
agents = TOML in `~/.codex/agents/`; experimental batch tool `spawn_agents_on_csv` ·
lifecycle hooks exist (`features.hooks`, `hooks.json`) incl. `SessionStart` +
`additionalContext` but are experimental and version-sensitive — held in reserve ·
skills: native, SKILL.md-compatible; directory drift `~/.codex/skills` (legacy) vs
`~/.agents/skills` (current docs) — support both · memories: opt-in, auto-generated
two-phase pipeline, local-only, empty here; OpenAI guidance: durable rules belong in
AGENTS.md/skills · transcripts: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
(`{"timestamp","type","payload"}` envelope) indexed by `~/.codex/state_*.sqlite`
(copy before querying; filename is schema-versioned); no session-end marker — use
idle heuristic · local `~/.codex/settings.json` (skim's hook registration) is an
undocumented filename, presumed inert.
