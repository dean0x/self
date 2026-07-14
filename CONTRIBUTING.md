# Contributing to self

## Prerequisites

- **Rust 1.85+** (edition 2024) — `rustup update stable` if needed
- **Node ≥ 18** — only required to exercise the npm shim in `npm/`; Rust work does not need it
- **git** — `self init` creates a git repo at `~/.self`; git must be on your `PATH`

## Build

```bash
cargo build --release
# binary lands at target/release/self
```

## Quality gates (must all pass before pushing)

These run verbatim in CI:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --locked
cargo deny check
```

Zero-warnings policy: clippy warnings are errors (`-D warnings`).

## Repo layout

| Path | Contents |
|------|----------|
| `src/` | CLI source (`self init`, `status`, `doctor`, `uninstall`) |
| `templates/` | Install artifacts embedded at build time (`preamble.md`, `constitution.md`, agents, seed corpus) |
| `npm/` | npm shim packages (`npm/self/` + five `npm/platforms/*/` binaries) |
| `scripts/` | `set-version.mjs` — single version-bump script for Cargo.toml + all package.json |
| `tests/` | Integration tests |

## Commit message convention

Lowercase `type: subject` — for example:

```
feat: add doctor --fix flag
fix: handle missing ~/.claude on Windows
docs: update install section in README
ci: pin actions to commit SHAs
chore: bump serde_json to 1.0.150
```

Types: `feat`, `fix`, `docs`, `ci`, `chore`, `refactor`, `test`, `perf`.

## Pull requests

- Add or update tests for any behaviour change
- All four quality gates must pass (`fmt`, `clippy -D warnings`, `test --locked`, `deny check`)
- No new compiler or clippy warnings
- Keep changes focused — one logical change per PR

Releases are maintainer-driven. See [RELEASING.md](RELEASING.md) for the release procedure.
