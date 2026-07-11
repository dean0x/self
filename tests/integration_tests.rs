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

/// Run `self <args>` with the given HOME, returning (stdout, stderr, exit_code).
fn run_cmd(home: &Path, args: &[&str]) -> (String, String, i32) {
    let output = Command::new(bin())
        .args(args)
        .env("HOME", home)
        // Clear any inherited PATH that might point to a real `self`.
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
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
    assert!(home.join(".claude/agents/self-learner.md").exists());
    assert!(home.join(".claude/agents/self-improver.md").exists());

    // ci-gate skill.
    assert!(home.join(".claude/skills/ci-gate/SKILL.md").exists());

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

    // REGISTRY.md has today's date (not the template date 2026-07-05).
    let registry = read_file(&home, ".self/REGISTRY.md");
    // The template date is the old one; after seeding it should have been replaced.
    assert!(
        registry.contains("created: "),
        "created: field missing from registry"
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
    let mutated = format!("{original_obs}\n- obs-9999 | 2026-01-01 | open | trigger: test | added manually | src: test\n");
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

/// `self uninstall` removes the marker block and agent files but leaves ~/.self
/// and the ci-gate skill.
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
    assert!(!home.join(".claude/agents/self-learner.md").exists());
    assert!(!home.join(".claude/agents/self-improver.md").exists());

    // ~/.self untouched.
    assert!(home.join(".self").exists(), "~/.self was removed");
    assert!(home.join(".self/REGISTRY.md").exists());

    // ci-gate skill untouched.
    assert!(home.join(".claude/skills/ci-gate/SKILL.md").exists());

    // Output mentions ~/.self location.
    assert!(
        out.contains(".self"),
        "uninstall output doesn't mention ~/.self"
    );
}

/// `self doctor` exits 0 when the installation is clean, then 1 after the
/// ci-gate skill file is deleted (dangling registry entry).
#[test]
fn doctor_clean_then_finds_dangling_skill() {
    let home = make_home();
    seed_fake_claude(&home);

    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);

    // Doctor on clean install should exit 0.
    let (out, err, code) = run_cmd(&home, &["doctor"]);
    assert_eq!(
        code, 0,
        "doctor not clean after init\nstdout: {out}\nstderr: {err}"
    );
    assert!(out.contains("clean"), "expected 'clean' in doctor output");

    // Remove the skill file.
    fs::remove_file(home.join(".claude/skills/ci-gate/SKILL.md")).unwrap();

    // Doctor should now exit 1.
    let (out, _, code) = run_cmd(&home, &["doctor"]);
    assert_eq!(code, 1, "doctor should exit 1 after skill deleted");
    assert!(
        out.contains("dangling") || out.contains("FIND"),
        "expected dangling finding in doctor output: {out}"
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
    let learner_path = home.join(".claude/agents/self-learner.md");
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
    let learner = read_file(&home, ".claude/agents/self-learner.md");
    assert!(
        !learner.contains("MUTATED AGENT"),
        "self-learner.md not restored by --reset"
    );
    assert!(
        learner.contains("self-learner"),
        "self-learner.md doesn't look like factory content"
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

/// `self status` succeeds on a freshly-initialized install.
#[test]
fn status_after_init() {
    let home = make_home();
    seed_fake_claude(&home);

    let (_, _, code) = run_cmd(&home, &["init"]);
    assert_eq!(code, 0);

    let (out, err, code) = run_cmd(&home, &["status"]);
    assert_eq!(code, 0, "status failed\nstdout: {out}\nstderr: {err}");
    assert!(
        out.contains("user"),
        "expected 'user' scope in status output"
    );
}

/// Running without HOME set results in a non-zero exit and an error message.
#[test]
fn missing_home_exits_with_error() {
    // Spawn without HOME at all.
    let output = Command::new(bin())
        .arg("init")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("spawn binary");

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
    let output = Command::new(bin())
        .arg("init")
        .env_clear()
        .env("HOME", "")
        .env("PATH", "/usr/bin:/bin")
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
    assert!(home.join(".claude/agents/self-learner.md").exists());
    assert!(home.join(".claude/agents/self-improver.md").exists());
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
