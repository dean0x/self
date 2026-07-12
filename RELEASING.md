# Releasing `self`

This document is the honest operator's guide to shipping a release. It covers the
one-time bootstrap (which is **not** automated and cannot be), the everyday release
procedure (which is), and exactly where the boundary between the two sits.

The pipeline publishes three things from a single `vX.Y.Z` git tag:

| Target        | Package(s)                                                        | Auth                         |
| ------------- | ---------------------------------------------------------------- | ---------------------------- |
| GitHub        | Release with per-target archives + `SHA256SUMS`                  | built-in `GITHUB_TOKEN`      |
| npm           | `@dean0x/self` + 5 platform packages                             | OIDC trusted publishing      |
| crates.io     | `self-cli`                                                        | OIDC trusted publishing      |

Workflows: [`.github/workflows/ci.yml`](.github/workflows/ci.yml) (every push / PR) and
[`.github/workflows/release.yml`](.github/workflows/release.yml) (every `v*` tag).

---

## TL;DR — cutting a release

Once the one-time setup below is done, every release is:

```bash
# 1. Bump the single source of truth (Cargo.toml + all six package.json + pins).
node scripts/set-version.mjs 0.2.0

# 2. Re-sync Cargo.lock to the new crate version (see "Cargo.lock" note below).
cargo build --locked || cargo build      # writes the updated Cargo.lock
git add -A

# 3. Commit, tag, push. The tag is what triggers the pipeline.
git commit -m "release: v0.2.0"
git tag v0.2.0
git push origin main
git push origin v0.2.0
```

That is the entire happy path. Everything after `git push origin v0.2.0` is automated.

---

## The release pipeline, job by job

`release.yml` triggers on `push` of any tag matching `v*` and runs these jobs in order:

```
check ──▶ build (5 native runners) ──┬──▶ github-release
                                     ├──▶ npm-publish
                                     └──▶ crates-publish
```

1. **check** (`contents: read`) — asserts `tag == Cargo.toml version == npm/self/package.json version`
   and **fails loudly** on any mismatch, then runs the full gate suite once
   (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --locked`, `cargo build --release --locked`). Nothing is published if this fails.
2. **build** (`contents: read`) — five native runner/target pairs, no cross-compilation:
   `x86_64-unknown-linux-gnu` (ubuntu-latest), `aarch64-unknown-linux-gnu` (ubuntu-24.04-arm),
   `x86_64-apple-darwin` (macos-13), `aarch64-apple-darwin` (macos-latest),
   `x86_64-pc-windows-msvc` (windows-latest). Each produces
   `self-v<ver>-<target>.tar.gz` (`.zip` on Windows) + a `.sha256`, and uploads the raw
   binary separately for the npm job.
3. **github-release** (`contents: write` — the **only** job with write) — downloads all
   archives, regenerates a combined `SHA256SUMS`, and creates the GitHub Release with
   `gh release create --generate-notes`.
4. **npm-publish** (`contents: read`, `id-token: write`) — places each downloaded binary
   into `npm/platforms/<p>/bin/` (restoring the executable bit that `upload-artifact`
   strips), then publishes the **five platform packages first, main last**. Ordering is
   load-bearing: if `@dean0x/self` landed before its platform packages, an install racing
   the publish would 404 on the optional dependencies.
5. **crates-publish** (`contents: read`, `id-token: write`) — exchanges the OIDC token for
   a crates.io token via `rust-lang/crates-io-auth-action@v1` and runs `cargo publish --locked`.

Least privilege: the workflow is `permissions: {}` at the top and each job opts into only
what it needs. `contents: write` exists on exactly one job.

### Cargo.lock must be in sync with the version

The pipeline uses `--locked` everywhere (reproducibility + supply-chain integrity). Bumping
the crate version in `Cargo.toml` makes the committed `Cargo.lock` stale (its `self-cli`
entry still names the old version), and a stale lock makes `cargo build --locked` — and
therefore the **check** gate — fail before anything is published. That failure is a feature
(it catches an inconsistent commit), but you must avoid tripping it: run `cargo build` after
`set-version.mjs` and commit the refreshed `Cargo.lock` in the same commit as the bump.

---

## One-time setup (NOT automated — and cannot be)

> **The core constraint, verified against primary docs (July 2026):** neither npm nor
> crates.io lets OIDC trusted publishing create a package that has never existed. The
> **first** publish of every package must be done by hand with a token; only then can you
> attach a trusted publisher and let CI take over. See [Verified facts](#verified-facts)
> for citations.

### A. crates.io — `self-cli`

1. **First publish, manually, with a token.** Create a crates.io API token
   (crates.io → Account Settings → API Tokens), then from a clean checkout at the release
   version:
   ```bash
   cargo login <your-crates-io-token>     # or: export CARGO_REGISTRY_TOKEN=<token>
   cargo publish --locked
   ```
   > Sanity check first: `cargo package --list` MUST show `templates/**` — the binary
   > embeds those via `include_str!`, so if they are excluded from the `.crate`,
   > `cargo publish`'s verification build fails and, worse, `cargo install self-cli` would
   > fail for end users. (Cargo.toml packaging metadata is owned outside this doc; this is
   > the check that proves it is correct.)
2. **Configure the trusted publisher.** crates.io → your crate → Settings → Trusted
   Publishing → add a GitHub publisher:
   - Repository owner/name: `dean0x/self`
   - Workflow filename: `release.yml`
   - (Optional but recommended) restrict to an environment if you add one.
3. Done. From the next tag, `crates-publish` authenticates via OIDC with no stored secret.

**Token fallback (only if you deliberately opt out of trusted publishing):** delete the
`rust-lang/crates-io-auth-action` step, add a `CARGO_REGISTRY_TOKEN` repository secret, and
set it as the env on `cargo publish`. Trusted publishing is verified working and is the
default in `release.yml`, so the fallback is documented for completeness, not needed.

### B. npm — all six packages

npm trusted publishing **cannot publish the initial version** and the package must exist on
npmjs.com before a trusted publisher can be configured. So each of the six packages needs a
manual first publish. There are two clean ways to do it:

**Option 1 (recommended) — ship `0.1.0` by hand once, then automate `0.2.0`+.**
The `build` and `github-release` jobs do **not** need OIDC, so push the `v0.1.0` tag and let
them run. `npm-publish` and `crates-publish` will fail on this first tag (no trusted
publisher yet) — that is expected. Then, locally:

```bash
# Get the five compiled binaries from the just-created GitHub Release (or the run's
# "binary-*" artifacts) and place them exactly where the CI job would:
#   npm/platforms/linux-x64/bin/self
#   npm/platforms/linux-arm64/bin/self
#   npm/platforms/darwin-x64/bin/self
#   npm/platforms/darwin-arm64/bin/self
#   npm/platforms/windows-x64/bin/self.exe
chmod +x npm/platforms/*/bin/self          # not the .exe

npm login                                   # a human account with publish rights
# Platform packages FIRST, main LAST (same ordering the pipeline enforces):
for p in linux-x64 linux-arm64 darwin-x64 darwin-arm64 windows-x64; do
  npm publish --access public "npm/platforms/$p"
done
npm publish --access public npm/self
```

Then do step B.2 below for all six and let CI handle every future release.

**Option 2 — bootstrap with throwaway versions.** Publish a minimal placeholder version
(e.g. `0.0.0`) of each package by hand just to register the names, configure trusted
publishing, then run the real `0.1.0` release through CI. Cleaner automation story, but
leaves a dead `0.0.0` version on each package forever. Option 1 is preferred.

**B.2 — configure the trusted publisher for each of the six packages.** For every package
(`@dean0x/self`, `@dean0x/self-linux-x64`, `-linux-arm64`, `-darwin-x64`, `-darwin-arm64`,
`-windows-x64`): npmjs.com → package → Settings → "Trusted Publisher" → **GitHub Actions**,
and set:
- Organization/user: `dean0x`
- Repository: `self`
- Workflow filename: `release.yml`

Yes, this is six separate configurations. There is no bulk option today.

> **`--access public` is mandatory and non-negotiable.** Scoped packages default to
> `restricted`; every `package.json` already carries `publishConfig.access: "public"` and
> the publish commands pass `--access public` as belt-and-suspenders. Without it the publish
> either fails or silently creates a private package.

### C. GitHub repository settings

- **Actions permissions:** Settings → Actions → General → Workflow permissions. The default
  read-only token is correct — the workflows request `contents: write` per-job where needed;
  do **not** grant org-wide write.
- **OIDC** requires no repo toggle beyond the per-job `id-token: write` already in the
  workflow.
- If you add a GitHub **Environment** (e.g. `release`) with required reviewers for a manual
  approval gate, also add that environment's name to the npm and crates.io trusted-publisher
  configs, and add `environment: release` to the `npm-publish` / `crates-publish` jobs.
  (Not configured by default.)

---

## What is explicitly NOT automated

- **The first publish of every package** (npm ×6, crates.io ×1) — one-time, manual, above.
- **Trusted-publisher configuration** — one-time, in the npm and crates.io web UIs.
- **The version bump, commit, tag, and push** — you run `set-version.mjs`, sync `Cargo.lock`,
  commit, tag, and push. The pipeline only reacts to the pushed tag.
- **Cargo.lock re-sync after a bump** — run `cargo build` and commit the result (see note above).
- **Curated changelogs** — release notes are auto-generated by `gh --generate-notes`
  (commit/PR-derived). Edit the release afterward if you want hand-written notes.
- **Re-releasing a version that already published** — npm and crates.io both reject
  re-publishing an existing version. If a release half-fails after some packages published,
  you cannot overwrite them: bump to the next patch version and release again. (npm-publish
  and crates-publish are independent jobs, so a crates.io failure does not roll back npm, and
  vice versa.)

---

## Verified facts

Checked against primary sources on 2026-07-12.

- **npm trusted publishing cannot create a new package; the first version must be published
  manually (login/token), and the package must exist before a trusted publisher can be
  configured.** Requires npm CLI ≥ 11.5.1 and Node ≥ 22.14.0; provenance is automatic on the
  OIDC path (no `--provenance` flag). Sources:
  npm docs, <https://docs.npmjs.com/trusted-publishers/>;
  open feature request confirming initial-version publish is unsupported,
  npm/cli#8544 "Allow publishing initial version with OIDC",
  <https://github.com/npm/cli/issues/8544>;
  GA announcement, <https://github.blog/changelog/2025-07-31-npm-trusted-publishing-with-oidc-is-generally-available/>.
- **crates.io trusted publishing** uses the official action **`rust-lang/crates-io-auth-action@v1`**,
  which exchanges the GitHub OIDC token for a short-lived (~30 min) crates.io token exposed as
  `steps.<id>.outputs.token` and consumed by `cargo publish` via `CARGO_REGISTRY_TOKEN`; it
  requires `permissions: id-token: write`. **The first release must be published manually**
  before a trusted publisher can be configured. Sources:
  <https://crates.io/docs/trusted-publishing>;
  <https://github.com/rust-lang/crates-io-auth-action>;
  RFC 3691, <https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html>.

---

## Hardening notes (optional)

- **Pin actions to commit SHAs.** The workflows pin to major-version / branch tags
  (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`,
  `rust-lang/crates-io-auth-action@v1`, …) for readability. For maximum supply-chain
  integrity, repin each to a full commit SHA and enable Dependabot for GitHub Actions to
  keep them current.
- **Add a `release` environment with required reviewers** to gate `npm-publish` /
  `crates-publish` behind a human approval (see section C above).
- **The version-check greps are anchored** (`^version[[:space:]]*=`) and the npm version is
  read with `node -p` (exact JSON parse), so neither can be fooled by a substring match such
  as `rust-version`.
