use std::cmp::Reverse;
use std::fs;

use crate::error::{Error, Result};
use crate::paths;
use crate::registry;
use crate::runlog;
use crate::settings::REQUIRED_PERMISSIONS;

/// Caps from C4.
const USER_CAP: usize = 25;
const PROJECT_CAP: usize = 15;

pub fn run() -> Result<()> {
    let home = paths::home()?;
    let self_dir = paths::self_dir(&home);

    if !self_dir.exists() {
        return Err(Error::NoSelfDir);
    }

    // Parse REGISTRY.md.
    let registry_path = self_dir.join("REGISTRY.md");
    let registry_content = if registry_path.exists() {
        fs::read_to_string(&registry_path)?
    } else {
        String::new()
    };
    let (mut entries, reg_bad) = registry::parse(&registry_content);

    // Parse log/runs.md.
    let runs_path = self_dir.join("log").join("runs.md");
    let runs_content = if runs_path.exists() {
        fs::read_to_string(&runs_path)?
    } else {
        String::new()
    };
    let (run_entries, total_run_lines, runs_bad) = runlog::parse(&runs_content);

    // ── Skill counts per scope ──
    let user_count = entries.iter().filter(|e| e.scope == "user").count();
    let project_scopes: std::collections::HashMap<String, usize> = {
        let mut map = std::collections::HashMap::new();
        for e in entries.iter().filter(|e| e.scope.starts_with("project:")) {
            *map.entry(e.scope.clone()).or_insert(0) += 1;
        }
        map
    };

    println!("=== self status ===");
    println!();
    println!("Learned skills:");
    println!(
        "  user scope:    {user_count:>3} / {USER_CAP}  ({} headroom)",
        USER_CAP.saturating_sub(user_count)
    );
    for (scope, count) in &project_scopes {
        let repo = scope.trim_start_matches("project:");
        println!(
            "  project:{repo:<16} {count:>3} / {PROJECT_CAP}  ({} headroom)",
            PROJECT_CAP.saturating_sub(*count)
        );
    }
    println!();

    // ── Per-skill counters ──
    if entries.is_empty() {
        println!("  (no learned skills registered)");
    } else {
        println!(
            "  {:<10} {:<20} {:<6} {:<8} {:<12} {:<8} {:<8}",
            "ID", "Slug", "Fired", "Applied", "Contradicted", "Invoked", "Refined"
        );
        println!("  {}", "-".repeat(80));
        for e in &entries {
            println!(
                "  {:<10} {:<20} {:<6} {:<8} {:<12} {:<8} {:<8}",
                e.id, e.slug, e.fired, e.applied, e.contradicted, e.invoked, e.refined
            );
        }
    }
    println!();

    if reg_bad > 0 {
        println!("  ({reg_bad} unparseable registry line(s) skipped)");
    }

    // ── Last learner and improver run lines ──
    println!("Recent runs:");
    let last_learner = run_entries
        .iter()
        .rev()
        .find(|e| e.agent.to_lowercase().starts_with("learner"));
    let last_improver = run_entries
        .iter()
        .rev()
        .find(|e| e.agent.to_lowercase().starts_with("improver"));

    if let Some(l) = last_learner {
        println!("  last learner:  {} — {}", l.timestamp, l.verdict);
    } else {
        println!("  last learner:  (none)");
    }
    if let Some(i) = last_improver {
        println!("  last improver: {} — {}", i.timestamp, i.verdict);
    } else {
        println!("  last improver: (none)");
    }
    println!();

    // ── Backlog trend ──
    let (backlog_values, trend) = runlog::backlog_trend(&run_entries);
    print!(
        "Backlog trend (last {} learner runs): ",
        backlog_values.len()
    );
    if backlog_values.is_empty() {
        println!("(no data)");
    } else {
        let vals: Vec<String> = backlog_values.iter().map(|v| v.to_string()).collect();
        println!("{} — {trend}", vals.join(", "));
    }
    println!();

    // ── Top 3 skills by applied+invoked ──
    entries.sort_by_key(|e| Reverse(e.evidence()));
    println!("Top skills by applied+invoked:");
    let top = entries.iter().take(3);
    let mut any = false;
    for e in top {
        println!(
            "  {} ({}) — applied={} invoked={}",
            e.slug, e.id, e.applied, e.invoked
        );
        any = true;
    }
    if !any {
        println!("  (none)");
    }
    println!();

    if runs_bad > 0 {
        println!("({runs_bad} unparseable run-log line(s) skipped)");
    }
    if total_run_lines > 0 {
        println!("Run log: {total_run_lines} line(s) (cap: 200)");
    }

    // ── Settings check (informational) ──
    let claude_dir = paths::claude_dir(&home);
    let settings_path = claude_dir.join("settings.json");
    if settings_path.exists() {
        match fs::read_to_string(&settings_path) {
            Ok(raw) => match crate::settings::merge_permissions(&raw) {
                Ok((_, missing)) => {
                    if missing.is_empty() {
                        println!(
                            "Permissions: all {} rules present",
                            REQUIRED_PERMISSIONS.len()
                        );
                    } else {
                        println!(
                            "Permissions: {} / {} rules present ({} missing)",
                            REQUIRED_PERMISSIONS.len() - missing.len(),
                            REQUIRED_PERMISSIONS.len(),
                            missing.len()
                        );
                    }
                }
                Err(_) => println!("Permissions: settings.json is invalid JSON"),
            },
            Err(_) => println!("Permissions: could not read settings.json"),
        }
    }

    Ok(())
}
