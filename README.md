# self

[![CI](https://github.com/dean0x/self/actions/workflows/ci.yml/badge.svg)](https://github.com/dean0x/self/actions/workflows/ci.yml)

A self-learning & self-improving layer for coding agents (Claude Code, Codex).
Fully ambient: two background subagents — a **learner** and an **improver** —
dispatched at session start, learning procedures as native skills. Everything of
substance is in **[spec.md](spec.md)**; read that first.

## Install

> **Not yet published.** v0.1.0 has not been tagged or released to any registry.
> The commands below will be valid once the first release ships (see [RELEASING.md](RELEASING.md)).

Three channels — pick one, then run `self init`:

- **npm** (recommended — Windows, macOS, Linux; no Rust toolchain needed):

  ```
  npm install -g @dean0x/self
  ```

- **cargo** (builds from source; needs a Rust toolchain):

  ```
  cargo install self-cli
  ```

- **Prebuilt binaries** — grab the archive for your platform from the
  [GitHub Releases](https://github.com/dean0x/self/releases) page.

## Layout

- `spec.md` — the system spec. §13.1 is the executor-ready M1 runbook.
- `Cargo.toml`, `src/`, `tests/` — the `self` CLI (M2): thin installer with
  `init` / `status` / `doctor` / `uninstall`.
- `templates/` — canonical install artifacts (plain markdown, embedded into the
  CLI at build):
  - `preamble.md` — marker block installed into `~/.claude/CLAUDE.md` (M3: `~/.codex/AGENTS.md`)
  - `constitution.md` — invariants, installed to `~/.self/constitution.md`
  - `agents/` — background agent definitions (`SelfLearning`, `SelfImproving`)
  - `seed/` — the initial `~/.self` corpus skeleton (headers only; the system ships no pre-seeded skills)

The installed instance lives at `~/.self` (git-tracked, agent-owned) — never
inside this repo.

## Development

Build from source:

```
cargo build --release
```

The binary is `target/release/self`. Before pushing, the gates are
`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
and `cargo test`.

## Status

M0 spec ✅ · M1 manual pilot ✅ installed 2026-07-11 (live ~2-week window running) ·
M2 `self` CLI ✅ · M3 Codex adapter (gated: codex not installed) · M4
evidence-driven extensions (gated on improver run-log evidence).
