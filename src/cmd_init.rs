use std::fs;
use std::path::Path;
use std::process::Command;

use crate::corpus;
use crate::error::{Error, Result};
use crate::markers;
use crate::paths;
use crate::settings;
use crate::templates;

pub fn run(reset: bool) -> Result<()> {
    let home = paths::home()?;
    let today = corpus::today_string();

    let self_dir = paths::self_dir(&home);
    let log_dir = self_dir.join("log");
    let git_dir = self_dir.join(".git");

    // --reset requires ~/.self to be an existing git repo.
    if reset && !git_dir.exists() {
        return Err(Error::Other(
            "--reset requires ~/.self to be a git repository; run `self init` first".to_owned(),
        ));
    }

    // Ensure ~/.self and ~/.self/log/ exist.
    if !self_dir.exists() {
        fs::create_dir_all(&log_dir)?;
        println!("created: ~/.self/");
        println!("created: ~/.self/log/");
    } else if !log_dir.exists() {
        fs::create_dir_all(&log_dir)?;
        println!("created: ~/.self/log/");
    }

    // --reset step 1: commit current state.
    if reset {
        git_run(&self_dir, &["add", "-A"])?;
        git_run(
            &self_dir,
            &["commit", "--allow-empty", "-m", "pre-reset snapshot"],
        )?;
        println!("committed: pre-reset snapshot");
    }

    // Seed (or overwrite for --reset) corpus files.
    let registry_content = corpus::seed_registry(templates::REGISTRY, &today);
    let corpus_files: &[(&str, &str)] = &[
        ("constitution.md", templates::CONSTITUTION),
        ("REGISTRY.md", &registry_content),
        ("observations.md", templates::OBSERVATIONS),
        ("retired.md", templates::RETIRED),
        ("log/runs.md", templates::RUNS),
    ];

    let mut anything_changed = false;

    for (name, content) in corpus_files {
        let path = self_dir.join(name);
        if reset {
            fs::write(&path, content)?;
            println!("replaced: ~/.self/{name}");
            anything_changed = true;
        } else if !path.exists() {
            fs::write(&path, content)?;
            println!("seeded: ~/.self/{name}");
            anything_changed = true;
        } else {
            println!("kept (exists): ~/.self/{name}");
        }
    }

    // Git init if needed.
    let mut git_initialized = false;
    if !git_dir.exists() {
        git_init(&self_dir)?;
        git_initialized = true;
        println!("created: ~/.self/.git (git init)");
    }

    // Commit corpus changes (also commit when git was just initialized so the
    // repo always has at least one commit, even if all corpus files pre-existed).
    if (anything_changed || git_initialized) && !reset {
        git_commit_all(&self_dir, "self init: seed corpus (M2 CLI)")?;
        println!("committed: self init: seed corpus (M2 CLI)");
    }
    if reset {
        git_commit_all(&self_dir, "factory reset")?;
        println!("committed: factory reset");
    }

    // Claude Code adapter.
    let claude_dir = paths::claude_dir(&home);
    if claude_dir.exists() {
        apply_claude_adapter(&home, &claude_dir, reset)?;
    } else {
        println!("skipped: Claude Code (~/.claude not found)");
    }

    // Codex adapter.
    apply_codex_adapter(&home);

    Ok(())
}

fn apply_claude_adapter(home: &Path, claude_dir: &Path, reset: bool) -> Result<()> {
    // ── a. CLAUDE.md marker block ──
    let claude_md_path = claude_dir.join("CLAUDE.md");
    let block = templates::PREAMBLE;

    if !claude_md_path.exists() {
        fs::write(&claude_md_path, block)?;
        println!("created: ~/.claude/CLAUDE.md");
    } else {
        let existing = fs::read_to_string(&claude_md_path)?;
        match markers::scan(&existing) {
            markers::MarkerState::None => {
                let new_content = markers::append_block(&existing, block);
                fs::write(&claude_md_path, new_content)?;
                println!("appended block: ~/.claude/CLAUDE.md");
            }
            markers::MarkerState::One { .. } => {
                let state = markers::scan(&existing);
                let new_content = markers::replace_block(&existing, block, &state);
                if new_content != existing {
                    fs::write(&claude_md_path, new_content)?;
                    println!("replaced block: ~/.claude/CLAUDE.md");
                } else {
                    println!("kept (exists, block unchanged): ~/.claude/CLAUDE.md");
                }
            }
            markers::MarkerState::Malformed(msg) => {
                eprintln!("error: CLAUDE.md has malformed markers — {msg}");
                eprintln!("       refusing to modify CLAUDE.md; fix the markers manually.");
                return Err(Error::MalformedMarkers(msg));
            }
        }
    }

    // ── b. Agent definitions ──
    let agents_dir = claude_dir.join("agents");
    if !agents_dir.exists() {
        fs::create_dir_all(&agents_dir)?;
    }
    write_agent_file(
        &agents_dir.join("self-learner.md"),
        templates::SELF_LEARNER,
        reset,
    )?;
    write_agent_file(
        &agents_dir.join("self-improver.md"),
        templates::SELF_IMPROVER,
        reset,
    )?;

    // ── c. Seed ci-gate skill ──
    let skills_dir = claude_dir.join("skills").join("ci-gate");
    if !skills_dir.exists() {
        fs::create_dir_all(&skills_dir)?;
    }
    let skill_path = skills_dir.join("SKILL.md");
    if reset || !skill_path.exists() {
        fs::write(&skill_path, templates::CI_GATE_SKILL)?;
        let action = if reset { "replaced" } else { "seeded" };
        println!("{action}: ~/.claude/skills/ci-gate/SKILL.md");
    } else {
        println!("kept (exists): ~/.claude/skills/ci-gate/SKILL.md");
    }

    // ── d. settings.json permissions merge ──
    let settings_path = claude_dir.join("settings.json");
    apply_settings(&settings_path, home)?;

    Ok(())
}

fn write_agent_file(path: &Path, factory: &str, reset: bool) -> Result<()> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if reset {
        fs::write(path, factory)?;
        println!("replaced: ~/.claude/agents/{name}");
    } else if !path.exists() {
        fs::write(path, factory)?;
        println!("seeded: ~/.claude/agents/{name}");
    } else {
        let existing = fs::read_to_string(path)?;
        if existing != factory {
            println!(
                "kept as-is (possibly improver-tuned): ~/.claude/agents/{name}; use --reset to restore factory"
            );
        } else {
            println!("kept (exists, unchanged): ~/.claude/agents/{name}");
        }
    }
    Ok(())
}

fn apply_settings(settings_path: &Path, home: &Path) -> Result<()> {
    let _ = home; // reserved for future tilde expansion of paths in JSON
    if !settings_path.exists() {
        let val = settings::minimal_settings();
        let json = settings::to_pretty_json(&val)?;
        fs::write(settings_path, json + "\n")?;
        println!("created: ~/.claude/settings.json (minimal with required permissions)");
        return Ok(());
    }

    let raw = fs::read_to_string(settings_path)?;
    match settings::merge_permissions(&raw) {
        Err(e) => {
            eprintln!("error: ~/.claude/settings.json is invalid JSON — {e}");
            eprintln!("       refusing to modify settings.json; fix it manually.");
            return Err(e);
        }
        Ok((val, added)) => {
            if added.is_empty() {
                println!("kept (all permissions present): ~/.claude/settings.json");
            } else {
                let json = settings::to_pretty_json(&val)?;
                fs::write(settings_path, json + "\n")?;
                println!(
                    "updated: ~/.claude/settings.json (added {} permission rule(s))",
                    added.len()
                );
            }
        }
    }
    Ok(())
}

fn apply_codex_adapter(home: &Path) {
    let codex_dir = paths::codex_dir(home);
    if !codex_dir.exists() {
        println!("codex: skipped (not installed)");
        return;
    }
    // Check for a `codex` binary on PATH.
    let codex_present = Command::new("sh")
        .args(["-c", "command -v codex"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if codex_present {
        println!("codex: detected but adapter is M3 — skipped");
    } else {
        println!("codex: skipped (not installed)");
    }
}

// ── git helpers ──

fn git_run(work_dir: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .arg("-c")
        .arg("user.name=self")
        .arg("-c")
        .arg("user.email=self@local")
        .arg("-C")
        .arg(work_dir)
        .args(args)
        .status()?;
    if !status.success() {
        return Err(Error::Other(format!(
            "git {} failed (exit {:?})",
            args.join(" "),
            status.code()
        )));
    }
    Ok(())
}

fn git_init(dir: &Path) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("init")
        .status()?;
    if !status.success() {
        return Err(Error::Other("git init failed".to_owned()));
    }
    Ok(())
}

fn git_commit_all(work_dir: &Path, message: &str) -> Result<()> {
    git_run(work_dir, &["add", "-A"])?;
    git_run(work_dir, &["commit", "--allow-empty", "-m", message])?;
    Ok(())
}
