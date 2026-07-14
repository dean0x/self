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

Workflows: [`.github/workflows/ci.yml`](.github/workflows/ci.yml) (every push / PR),
[`.github/workflows/release.yml`](.github/workflows/release.yml) (every `v*` tag), and
[`.github/workflows/publish.yml`](.github/workflows/publish.yml) (`workflow_dispatch` — the
one-time CI bootstrap for the first publish of a package).

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
2. **build** (`contents: read`) — five runner/target pairs:
   `x86_64-unknown-linux-gnu` (ubuntu-latest), `aarch64-unknown-linux-gnu` (ubuntu-24.04-arm),
   `x86_64-apple-darwin` (macos-latest, cross-compiled from arm64), `aarch64-apple-darwin` (macos-latest),
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

## One-time setup (bootstrap the first publish)

> **The core constraint, verified against primary docs (July 2026):** neither npm nor
> crates.io lets OIDC trusted publishing create a package that has never existed. The
> **first** publish of every package must be done with a token; only then can you attach a
> trusted publisher and let CI take over keylessly. See [Verified facts](#verified-facts)
> for citations.

That token publish no longer has to happen on anyone's laptop.
[`.github/workflows/publish.yml`](.github/workflows/publish.yml) is a `workflow_dispatch`
job that runs the entire first publish **in CI** from token secrets — idempotent,
each archive verified against its `.sha256`, platform packages before the main package. So
the only manual work is creating two tokens and clicking "Run workflow".

### Recommended: bootstrap from CI with token secrets

1. **Create the tokens.**
   - npm: a **granular access token** (npmjs.com → Access Tokens → Generate New Token →
     Granular) with **Read and write** permission for the `@dean0x/self*` packages. A classic
     "Automation" token also works.
   - crates.io: an API token (crates.io → Account Settings → API Tokens) with publish scope.
2. **Add them as repository secrets** named exactly `NPM_TOKEN` and `CARGO_REGISTRY_TOKEN`
   (GitHub → Settings → Secrets and variables → Actions → New repository secret, or from a
   trusted shell):
   ```bash
   gh secret set NPM_TOKEN            # paste the npm token
   gh secret set CARGO_REGISTRY_TOKEN # paste the crates.io token
   ```
3. **Dispatch the bootstrap publish** against the release tag whose GitHub Release already
   exists. (Push the tag first and let `release.yml`'s `build` + `github-release` jobs create
   the Release and its archives; `npm-publish` / `crates-publish` in `release.yml` will fail on
   that first tag — expected, no trusted publisher yet.)
   ```bash
   gh workflow run publish.yml -f tag=v0.1.0
   ```
   `publish.yml` then downloads the five release archives, **verifies each against its
   `.sha256`** (hard stop on mismatch), places the binaries into `npm/platforms/<p>/bin/`,
   publishes the **five platform packages first, `@dean0x/self` last**, and publishes
   `self-cli` to crates.io. With the secrets set it uses them (npm with `--provenance`); with
   them unset it falls back to OIDC. It is **idempotent** — every package is checked
   (`npm view` / the crates.io API) and skipped if already published, so a partial failure is
   recovered by simply dispatching the workflow again.
4. **Configure the trusted publisher for all 7 packages** (six npm + `self-cli`) — see
   **A. crates.io** and **B. npm** below. From the next tag push, `release.yml` publishes every
   package keylessly; `publish.yml` is only needed again to bootstrap a brand-new package name.
5. **Optionally delete both tokens.** Once trusted publishers are live, no workflow reads
   `NPM_TOKEN` or `CARGO_REGISTRY_TOKEN`; delete the two secrets and revoke the tokens if you
   want zero long-lived credentials.

### A. crates.io — `self-cli`

**Configure the trusted publisher** (after the crate exists — i.e. after step 3):
crates.io → your crate → Settings → Trusted Publishing → add a GitHub publisher:
- Repository owner/name: `dean0x/self`
- Workflow filename: `release.yml`
- (Optional but recommended) restrict to an environment if you add one.

From the next tag, `crates-publish` in `release.yml` authenticates via OIDC with no stored
secret.

> Sanity check the crate contents before the first publish: `cargo package --list` MUST show
> `templates/**` — the binary embeds those via `include_str!`, so if they are excluded from
> the `.crate`, `cargo publish`'s verification build fails and, worse, `cargo install self-cli`
> would fail for end users. (Cargo.toml packaging metadata is owned outside this doc; this is
> the check that proves it is correct.)

**Local fallback (only if you cannot or prefer not to dispatch `publish.yml`):** publish the
first version by hand from a clean checkout at the release version:
```bash
cargo login <your-crates-io-token>     # or: export CARGO_REGISTRY_TOKEN=<token>
cargo publish --locked
```

### B. npm — all six packages

**Configure the trusted publisher for each of the six packages** (after they exist — i.e. after
step 3). For every package (`@dean0x/self`, `@dean0x/self-linux-x64`, `-linux-arm64`,
`-darwin-x64`, `-darwin-arm64`, `-windows-x64`): npmjs.com → package → Settings → "Trusted
Publisher" → **GitHub Actions**, and set:
- Organization/user: `dean0x`
- Repository: `self`
- Workflow filename: `release.yml`

Yes, this is six separate configurations. There is no bulk option today.

> **`--access public` is mandatory and non-negotiable.** Scoped packages default to
> `restricted`; every `package.json` already carries `publishConfig.access: "public"` and
> the publish commands pass `--access public` as belt-and-suspenders. Without it the publish
> either fails or silently creates a private package.

**Local fallback (only if you cannot or prefer not to dispatch `publish.yml`):** place the five
compiled binaries from the GitHub Release exactly where CI would and publish by hand, platform
packages FIRST, main LAST:
```bash
#   npm/platforms/linux-x64/bin/self
#   npm/platforms/linux-arm64/bin/self
#   npm/platforms/darwin-x64/bin/self
#   npm/platforms/darwin-arm64/bin/self
#   npm/platforms/windows-x64/bin/self.exe
chmod +x npm/platforms/*/bin/self          # not the .exe

npm login                                   # a human account with publish rights
for p in linux-x64 linux-arm64 darwin-x64 darwin-arm64 windows-x64; do
  npm publish --access public "npm/platforms/$p"
done
npm publish --access public npm/self
```

### C. GitHub repository settings

- **Secrets:** `NPM_TOKEN` and `CARGO_REGISTRY_TOKEN` are read only by `publish.yml`, and only
  while bootstrapping; `release.yml` never reads them. Delete them once trusted publishers are
  configured (recommended step 5 above).
- **Actions permissions:** Settings → Actions → General → Workflow permissions. The default
  read-only token is correct — the workflows request `contents: write` / `id-token: write`
  per-job where needed; do **not** grant org-wide write.
- **OIDC** requires no repo toggle beyond the per-job `id-token: write` already in the
  workflows.
- If you add a GitHub **Environment** (e.g. `release`) with required reviewers for a manual
  approval gate, also add that environment's name to the npm and crates.io trusted-publisher
  configs, and add `environment: release` to the publishing jobs. (Not configured by default.)

---

## What is explicitly NOT automated

- **Creating the two bootstrap tokens and adding them as secrets** — one-time, manual, above.
  The first publish itself now runs **in CI**: `gh workflow run publish.yml -f tag=vX.Y.Z`
  (idempotent; a local manual publish remains as a fallback). See the one-time setup section.
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

## Dry-run (build without publishing)

Run `gh workflow run release.yml` (no tag) to execute a full dry-run:

- The `check` job runs all quality gates but skips the tag-version assertion.
- The `build` matrix compiles and packages all five targets, uploading the archives as
  workflow artifacts.
- The three publish-side jobs (`github-release`, `npm-publish`, `crates-publish`) are
  skipped entirely.

This is useful for verifying that the build matrix works before cutting a release tag.

## Build-provenance attestations

Release archives and `SHA256SUMS` are attested with GitHub build-provenance
attestations generated by `actions/attest-build-provenance`. Verify any release
asset with:

```bash
gh attestation verify <file> -R dean0x/self
```

---

## Hardening notes (optional)

- **Add a `release` environment with required reviewers** to gate `npm-publish` /
  `crates-publish` behind a human approval (see section C above).
- **The version-check greps are anchored** (`^version[[:space:]]*=`) and the npm version is
  read with `node -p` (exact JSON parse), so neither can be fooled by a substring match such
  as `rust-version`.
