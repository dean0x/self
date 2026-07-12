/// Integration tests for the `self` CLI.
///
/// Each test:
/// - Creates a unique temp dir (never touches the real HOME).
/// - Sets HOME to that dir for every spawned `Command`.
/// - Creates a fake `~/.claude/` with a pre-existing CLAUDE.md line and a
///   `settings.json` that already has a deny rule.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

// ── helpers ──────────────────────────────────────────────────────────────────

/// The path to the compiled binary, injected by cargo at test time.
fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_self"))
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Create a unique, isolated test home directory.
fn make_home() -> PathBuf {
    let id = std::process::id();
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = env::temp_dir().join(format!("self-test-{id}-{n}"));
    fs::create_dir_all(&dir).expect("create test home dir");
    dir
}

/// Populate a fake `~/.claude/` with:
/// - `CLAUDE.md` containing one line of existing content.
/// - `settings.json` with an existing deny rule and one pre-existing allow rule.
fn seed_fake_claude(home: &Path) {
    let claude = home.join(".claude");
    fs::create_dir_all(&claude).unwrap();

    fs::write(
        claude.join("CLAUDE.md"),
        "# My existing CLAUDE.md content\n",
    )
    .unwrap();

    fs::write(
        claude.join("settings.json"),
        r#"{
  "permissions": {
    "allow": ["Bash(echo hello)"],
    "deny": ["Bash(rm -rf /)"]
  }
}
"#,
    )
    .unwrap();
}

/// Apply hermetic-but-portable environment isolation to a spawned command:
/// wipe the inherited environment, then restore only the OS essentials the
/// child needs to locate and run `git` on every platform.
///
/// The harness previously hardcoded `PATH=/usr/bin:/bin`, a Unix-only path.
/// On Windows the spawned `self` process could then not locate `git.exe`
/// (installed under `C:\Program Files\Git\cmd`, never on a Unix PATH), so its
/// internal `Command::new("git")` failed with "program not found". Propagating
/// the parent's real PATH keeps the child hermetic — callers still override
/// HOME and the git identity — while letting `git` resolve on Linux, macOS,
/// and Windows alike. (`bin()` is an absolute path, so isolation never relied
/// on PATH to select the binary under test.)
fn isolate_env(cmd: &mut Command) -> &mut Command {
    cmd.env_clear();
    if let Some(path) = env::var_os("PATH") {
        cmd.env("PATH", path);
    }
    // Git for Windows relies on SystemRoot for DLL loading and crypto init.
    #[cfg(windows)]
    if let Some(root) = env::var_os("SystemRoot") {
        cmd.env("SystemRoot", root);
    }
    cmd
}

/// Run `self <args>` with the given HOME, returning (stdout, stderr, exit_code).
fn run_cmd(home: &Path, args: &[&str]) -> (String, String, i32) {
    let mut cmd = Command::new(bin());
    cmd.args(args);
    let output = isolate_env(&mut cmd)
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@test")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@test")
        .output()
        .expect("spawn binary");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

/// Count commits in ~/.self.
fn git_log_count(home: &Path) -> usize {
    let output = Command::new("git")
        .args([
            "-C",
            home.join(".self").to_str().unwrap(),
            "log",
            "--oneline",
        ])
        .env("HOME", home)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .count()
}

/// Read a file relative to home.
fn read_file(home: &Path, rel: &str) -> String {
    fs::read_to_string(home.join(rel)).unwrap_or_default()
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Fresh `self init` seeds all five corpus files, sets up .git, and git log
/// shows exactly one commit.
#[test]
fn fresh_init_seeds_everything_and_one_commit() {
    let home = make_home();
    seed_fake_claude(&home);

    let (out, err, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0, "init failed\nstdout: {out}\nstderr: {err}");

    // Corpus files exist.
    for name in &[
        ".self/constitution.md",
        ".self/REGISTRY.md",
        ".self/observations.md",
        ".self/retired.md",
        ".self/log/runs.md",
    ] {
        assert!(
            home.join(name).exists(),
            "{name} not found after init\nstdout: {out}"
        );
    }

    // Agent definitions.
    assert!(home.join(".claude/agents/self-learning.md").exists());
    assert!(home.join(".claude/agents/self-improvement.md").exists());

    // No skills are seeded — the corpus ships empty.
    assert!(
        !home.join(".claude/skills").exists(),
        "skills dir must not be created by a clean init"
    );

    // CLAUDE.md has exactly one marker pair.
    let claude_md = read_file(&home, ".claude/CLAUDE.md");
    let start_count = claude_md.matches("<!-- self:start").count();
    let end_count = claude_md.matches("<!-- self:end -->").count();
    assert_eq!(start_count, 1, "start marker count: {start_count}");
    assert_eq!(end_count, 1, "end marker count: {end_count}");

    // The pre-existing line is preserved.
    assert!(
        claude_md.contains("# My existing CLAUDE.md content"),
        "pre-existing content missing from CLAUDE.md"
    );

    // settings.json: pre-existing deny rule preserved, all nine rules present.
    let settings_raw = read_file(&home, ".claude/settings.json");
    assert!(
        settings_raw.contains("Bash(rm -rf /)"),
        "deny rule was removed"
    );
    let settings: serde_json::Value = serde_json::from_str(&settings_raw).unwrap();
    let allow = settings["permissions"]["allow"].as_array().unwrap();
    assert!(
        allow.len() >= 9,
        "expected ≥ 9 allow rules, got {}",
        allow.len()
    );

    // REGISTRY.md ships with no skill entries (empty corpus contract).
    let registry = read_file(&home, ".self/REGISTRY.md");
    assert!(
        !registry.contains("- S-"),
        "registry must list no skills after a fresh init; got:\n{registry}"
    );
    // observations.md ships with no entries.
    let obs = read_file(&home, ".self/observations.md");
    assert!(
        !obs.contains("- obs-"),
        "observations must be empty after a fresh init; got:\n{obs}"
    );

    // Git log shows exactly one commit.
    assert_eq!(git_log_count(&home), 1, "expected 1 commit after init");

    // Output mentions codex skipped line.
    assert!(
        out.contains("codex: skipped") || out.contains("codex:"),
        "no codex line in output"
    );
}

/// A second `self init` must be idempotent: no new git commit, CLAUDE.md bytes
/// unchanged, and a corpus mutation made between the two runs must survive.
#[test]
fn second_init_is_idempotent() {
    let home = make_home();
    seed_fake_claude(&home);

    // First run.
    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);

    // Mutate a corpus file between the two runs.
    let obs_path = home.join(".self/observations.md");
    let original_obs = fs::read_to_string(&obs_path).unwrap();
    let mutated = format!(
        "{original_obs}\n- obs-9999 | 2026-01-01 | open | trigger: test | added manually | src: test\n"
    );
    fs::write(&obs_path, &mutated).unwrap();

    // Snapshot CLAUDE.md bytes.
    let claude_md_before = fs::read(home.join(".claude/CLAUDE.md")).unwrap();

    // Second run.
    let (out, err, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0, "second init failed\nstdout: {out}\nstderr: {err}");

    // No new git commit (still 1).
    assert_eq!(
        git_log_count(&home),
        1,
        "unexpected new commit on second init"
    );

    // CLAUDE.md bytes unchanged.
    let claude_md_after = fs::read(home.join(".claude/CLAUDE.md")).unwrap();
    assert_eq!(
        claude_md_before, claude_md_after,
        "CLAUDE.md changed on second init"
    );

    // The corpus mutation survives.
    let obs_after = fs::read_to_string(&obs_path).unwrap();
    assert_eq!(obs_after, mutated, "corpus mutation was overwritten");
}

/// `self uninstall` removes the marker block and agent files but leaves ~/.self.
#[test]
fn uninstall_removes_block_and_agents_leaves_self() {
    let home = make_home();
    seed_fake_claude(&home);

    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);

    let (out, err, code) = run_cmd(&home, &["uninstall"]);
    assert_eq!(code, 0, "uninstall failed\nstdout: {out}\nstderr: {err}");

    // Marker block removed.
    let claude_md = read_file(&home, ".claude/CLAUDE.md");
    assert!(
        !claude_md.contains("<!-- self:start"),
        "marker block still present after uninstall"
    );
    assert!(
        !claude_md.contains("<!-- self:end -->"),
        "end marker still present after uninstall"
    );

    // Pre-existing content preserved.
    assert!(
        claude_md.contains("# My existing CLAUDE.md content"),
        "pre-existing content removed by uninstall"
    );

    // Agent files deleted.
    assert!(!home.join(".claude/agents/self-learning.md").exists());
    assert!(!home.join(".claude/agents/self-improvement.md").exists());

    // ~/.self untouched.
    assert!(home.join(".self").exists(), "~/.self was removed");
    assert!(home.join(".self/REGISTRY.md").exists());

    // Output mentions ~/.self location.
    assert!(
        out.contains(".self"),
        "uninstall output doesn't mention ~/.self"
    );
}

/// `self doctor` exits 0 on a clean install (empty corpus), then exits 1 when
/// a registry entry points at a skill file that does not exist (drift detection).
#[test]
fn doctor_clean_then_finds_dangling_skill() {
    let home = make_home();
    seed_fake_claude(&home);

    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);

    // Doctor on a clean install with an empty corpus exits 0.
    let (out, err, code) = run_cmd(&home, &["doctor"]);
    assert_eq!(
        code, 0,
        "doctor not clean after init\nstdout: {out}\nstderr: {err}"
    );
    assert!(out.contains("clean"), "expected 'clean' in doctor output");

    // Inject a synthetic registry entry whose skill file does NOT exist —
    // this simulates the registry/file drift that doctor is designed to detect.
    let synthetic_skill_path = home.join(".claude/skills/test-skill/SKILL.md");
    let registry_path = home.join(".self/REGISTRY.md");
    let registry_line = format!(
        "- S-0099 | test-skill | user | {} | created: 2026-01-01 | src: obs-9999 | fired: 0 applied: 0 contradicted: 0 invoked: 0 refined: 0 | flags: -\n",
        synthetic_skill_path.display()
    );
    let mut registry_content = fs::read_to_string(&registry_path).unwrap();
    registry_content.push_str(&registry_line);
    fs::write(&registry_path, &registry_content).unwrap();

    // Doctor should now exit 1 — the registry entry is dangling.
    let (out, _, code) = run_cmd(&home, &["doctor"]);
    assert_eq!(code, 1, "doctor should exit 1 with dangling registry entry");
    assert!(
        out.contains("dangling"),
        "expected dangling finding in doctor output: {out}"
    );
}

/// `self doctor` exercises check_scope_vs_path and check_frontmatter:
///  - a skill with valid frontmatter at the right scope path → stays clean
///  - a skill with missing 'name:' → FIND reported
///  - a skill registered as 'user' scope but stored outside ~/.claude/skills/ → scope mismatch
#[test]
fn doctor_checks_frontmatter_and_scope() {
    let home = make_home();
    seed_fake_claude(&home);

    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);

    let registry_path = home.join(".self/REGISTRY.md");

    // ── Phase 1: valid skill → doctor remains clean ──────────────────────────
    let valid_skill_path = home.join(".claude/skills/tidy-imports/SKILL.md");
    fs::create_dir_all(valid_skill_path.parent().unwrap()).unwrap();
    fs::write(
        &valid_skill_path,
        "---\nname: tidy-imports\ndescription: removes unused import statements\n---\n\nbody\n",
    )
    .unwrap();
    let mut registry = fs::read_to_string(&registry_path).unwrap();
    registry.push_str(&format!(
        "- S-0042 | tidy-imports | user | {} | created: 2026-01-01 | src: obs-0001 | fired: 0 applied: 0 contradicted: 0 invoked: 0 refined: 0 | flags: -\n",
        valid_skill_path.display()
    ));
    fs::write(&registry_path, &registry).unwrap();

    let (out, _, code) = run_cmd(&home, &["doctor"]);
    assert_eq!(
        code, 0,
        "doctor should be clean with a valid skill entry: {out}"
    );

    // ── Phase 2: skill missing 'name:' → FIND for MissingName ───────────────
    let bad_fm_path = home.join(".claude/skills/no-name/SKILL.md");
    fs::create_dir_all(bad_fm_path.parent().unwrap()).unwrap();
    fs::write(
        &bad_fm_path,
        "---\ndescription: has description but no name field\n---\n\nbody\n",
    )
    .unwrap();
    let mut registry = fs::read_to_string(&registry_path).unwrap();
    registry.push_str(&format!(
        "- S-0043 | no-name | user | {} | created: 2026-01-01 | src: obs-0002 | fired: 0 applied: 0 contradicted: 0 invoked: 0 refined: 0 | flags: -\n",
        bad_fm_path.display()
    ));
    fs::write(&registry_path, &registry).unwrap();

    let (out, _, code) = run_cmd(&home, &["doctor"]);
    assert_eq!(
        code, 1,
        "doctor should find missing 'name:' in frontmatter: {out}"
    );
    assert!(
        out.contains("missing 'name:'"),
        "expected missing-name finding in doctor output: {out}"
    );

    // Remove the bad entry so phase 3 is independent.
    let registry = fs::read_to_string(&registry_path).unwrap();
    let registry: String = registry
        .lines()
        .filter(|l| !l.contains("S-0043"))
        .map(|l| format!("{l}\n"))
        .collect();
    fs::write(&registry_path, &registry).unwrap();
    fs::remove_file(&bad_fm_path).unwrap();

    // ── Phase 3: 'user' scope but path outside ~/.claude/skills/ → mismatch ─
    let misplaced_path = home.join("some-other-dir/odd-skill/SKILL.md");
    fs::create_dir_all(misplaced_path.parent().unwrap()).unwrap();
    fs::write(
        &misplaced_path,
        "---\nname: odd-skill\ndescription: correctly formed but stored in wrong location\n---\n\nbody\n",
    )
    .unwrap();
    let mut registry = fs::read_to_string(&registry_path).unwrap();
    registry.push_str(&format!(
        "- S-0044 | odd-skill | user | {} | created: 2026-01-01 | src: obs-0003 | fired: 0 applied: 0 contradicted: 0 invoked: 0 refined: 0 | flags: -\n",
        misplaced_path.display()
    ));
    fs::write(&registry_path, &registry).unwrap();

    let (out, _, code) = run_cmd(&home, &["doctor"]);
    assert_eq!(code, 1, "doctor should find scope mismatch: {out}");
    assert!(
        out.contains("scope mismatch"),
        "expected scope-mismatch finding in doctor output: {out}"
    );
}

/// `self init --reset` produces the two extra git commits (pre-reset snapshot
/// and factory reset) and restores factory content.
#[test]
fn reset_produces_two_extra_commits_and_restores_factory() {
    let home = make_home();
    seed_fake_claude(&home);

    // Initial init (1 commit).
    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);
    assert_eq!(git_log_count(&home), 1);

    // Mutate a corpus file so we can verify the reset overwrites it.
    let const_path = home.join(".self/constitution.md");
    fs::write(&const_path, "MUTATED\n").unwrap();

    // Mutate an agent def so we can verify it's reset too.
    let learner_path = home.join(".claude/agents/self-learning.md");
    fs::write(&learner_path, "MUTATED AGENT\n").unwrap();

    // --reset (2 more commits: pre-reset snapshot + factory reset).
    let (out, err, code) = run_cmd(&home, &["init", "--reset"]);
    assert_eq!(code, 0, "init --reset failed\nstdout: {out}\nstderr: {err}");

    // Three commits total: original + pre-reset snapshot + factory reset.
    assert_eq!(git_log_count(&home), 3, "expected 3 commits after --reset");

    // constitution.md restored to factory.
    let constitution = read_file(&home, ".self/constitution.md");
    assert!(
        !constitution.contains("MUTATED"),
        "constitution.md not restored by --reset"
    );
    assert!(
        constitution.contains("C1"),
        "constitution.md doesn't look like factory content"
    );

    // Agent definition restored.
    let learner = read_file(&home, ".claude/agents/self-learning.md");
    assert!(
        !learner.contains("MUTATED AGENT"),
        "self-learning.md not restored by --reset"
    );
    assert!(
        learner.contains("SelfLearning"),
        "self-learning.md doesn't look like factory content"
    );

    // CLAUDE.md block is still valid (exactly one pair).
    let claude_md = read_file(&home, ".claude/CLAUDE.md");
    assert_eq!(claude_md.matches("<!-- self:start").count(), 1);
    assert_eq!(claude_md.matches("<!-- self:end -->").count(), 1);
}

/// `self init --reset` fails cleanly if ~/.self is not a git repo.
#[test]
fn reset_without_git_repo_fails() {
    let home = make_home();
    let (out, err, code) = run_cmd(&home, &["init", "--reset"]);
    assert_ne!(code, 0, "expected failure\nstdout: {out}\nstderr: {err}");
    let combined = format!("{out}{err}");
    assert!(
        combined.contains("git") || combined.contains("repository") || combined.contains("error"),
        "error message should mention git or repository: {combined}"
    );
}

/// Unknown commands exit with code 2.
#[test]
fn unknown_command_exits_2() {
    let home = make_home();
    let (_, _, code) = run_cmd(&home, &["frobnicate"]);
    assert_eq!(code, 2);
}

/// `self status` renders skill counter table and top-3 ranking when the registry
/// is non-empty.
#[test]
fn status_after_init() {
    let home = make_home();
    seed_fake_claude(&home);

    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);

    // Inject a synthetic skill so the per-skill table loop and top-3 ranking run.
    let skill_path = home.join(".claude/skills/tidy-imports/SKILL.md");
    fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
    fs::write(
        &skill_path,
        "---\nname: tidy-imports\ndescription: removes unused import statements\n---\n\nbody\n",
    )
    .unwrap();
    let registry_path = home.join(".self/REGISTRY.md");
    let registry_line = format!(
        "- S-0042 | tidy-imports | user | {} | created: 2026-01-01 | src: obs-0001 | fired: 3 applied: 2 contradicted: 0 invoked: 1 refined: 0 | flags: -\n",
        skill_path.display()
    );
    let mut registry = fs::read_to_string(&registry_path).unwrap();
    registry.push_str(&registry_line);
    fs::write(&registry_path, registry).unwrap();

    let (out, err, code) = run_cmd(&home, &["status"]);
    assert_eq!(code, 0, "status failed\nstdout: {out}\nstderr: {err}");
    // Static header always present.
    assert!(
        out.contains("user"),
        "expected 'user' scope in status output: {out}"
    );
    // Per-skill counter row is rendered (the table loop body ran).
    assert!(
        out.contains("tidy-imports"),
        "expected skill slug in status counter table: {out}"
    );
    // Top-3 ranking section shows the skill (the entries.iter().take(3) path ran).
    assert!(
        out.contains("applied=") && out.contains("invoked="),
        "expected top-skills ranking in status output: {out}"
    );
}

/// Running without HOME set results in a non-zero exit and an error message.
#[test]
fn missing_home_exits_with_error() {
    // Spawn without HOME (or USERPROFILE) at all — env_clear removes both.
    let mut cmd = Command::new(bin());
    cmd.arg("init");
    let output = isolate_env(&mut cmd).output().expect("spawn binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("HOME") || stderr.contains("home"),
        "error should mention HOME: {stderr}"
    );
}

/// HOME set to an empty string is treated as unset: non-zero exit + error message.
#[test]
fn empty_home_exits_with_error() {
    let mut cmd = Command::new(bin());
    cmd.arg("init");
    let output = isolate_env(&mut cmd)
        .env("HOME", "")
        .output()
        .expect("spawn binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("HOME") || stderr.contains("home"),
        "error should mention HOME: {stderr}"
    );
}

/// When corpus files already exist in ~/.self but no .git is present (e.g. after
/// restoring from a backup that omitted .git), `self init` must still create exactly
/// one git commit so `git log` is valid.
#[test]
fn init_with_corpus_but_no_git_makes_one_commit() {
    let home = make_home();
    seed_fake_claude(&home);

    // Pre-create the corpus files without a .git directory.
    let self_dir = home.join(".self");
    fs::create_dir_all(self_dir.join("log")).unwrap();
    for name in &[
        "constitution.md",
        "REGISTRY.md",
        "observations.md",
        "retired.md",
        "log/runs.md",
    ] {
        fs::write(self_dir.join(name), "placeholder\n").unwrap();
    }

    let (out, err, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0, "init failed\nstdout: {out}\nstderr: {err}");
    assert_eq!(
        git_log_count(&home),
        1,
        "expected 1 commit when corpus pre-existed without .git"
    );
}

/// After `self init` + `self uninstall`, a second `self init` re-installs cleanly.
#[test]
fn reinit_after_uninstall() {
    let home = make_home();
    seed_fake_claude(&home);

    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);

    let (_, _, code) = run_cmd(&home, &["uninstall"]);
    assert_eq!(code, 0);

    // After uninstall, re-init.
    let (out, err, code) = run_cmd(&home, &["init"]);
    assert_eq!(
        code, 0,
        "re-init after uninstall failed\nstdout: {out}\nstderr: {err}"
    );

    // CLAUDE.md should again have exactly one marker pair.
    let claude_md = read_file(&home, ".claude/CLAUDE.md");
    assert_eq!(claude_md.matches("<!-- self:start").count(), 1);
    assert_eq!(claude_md.matches("<!-- self:end -->").count(), 1);

    // Agent files restored.
    assert!(home.join(".claude/agents/self-learning.md").exists());
    assert!(home.join(".claude/agents/self-improvement.md").exists());
}

/// The settings.json pre-existing deny rule and allow rule are preserved after init.
#[test]
fn settings_preserves_existing_rules_and_deny() {
    let home = make_home();
    seed_fake_claude(&home);

    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);

    let raw = read_file(&home, ".claude/settings.json");
    let val: serde_json::Value = serde_json::from_str(&raw).unwrap();

    // Pre-existing allow rule preserved.
    let allow = val["permissions"]["allow"].as_array().unwrap();
    assert!(
        allow.iter().any(|v| v.as_str() == Some("Bash(echo hello)")),
        "pre-existing allow rule removed"
    );

    // Pre-existing deny rule preserved.
    let deny = val["permissions"]["deny"].as_array().unwrap();
    assert!(
        deny.iter().any(|v| v.as_str() == Some("Bash(rm -rf /)")),
        "pre-existing deny rule removed"
    );

    // All nine required rules now present.
    let required = [
        "Read(~/.claude/projects/**)",
        "Read(~/.self/**)",
        "Write(~/.self/**)",
        "Edit(~/.self/**)",
        "Write(~/.claude/skills/**)",
        "Edit(~/.claude/skills/**)",
        "Write(**/.claude/skills/**)",
        "Edit(**/.claude/skills/**)",
        "Bash(git -C ~/.self *)",
    ];
    for rule in &required {
        assert!(
            allow.iter().any(|v| v.as_str() == Some(rule)),
            "required rule missing: {rule}"
        );
    }
}
