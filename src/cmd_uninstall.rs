use std::fs;

use crate::error::{Error, Result};
use crate::markers;
use crate::paths;
use crate::registry;

pub fn run() -> Result<()> {
    let home = paths::home()?;
    let self_dir = paths::self_dir(&home);
    let claude_dir = paths::claude_dir(&home);

    // ── 1. Remove marker block from CLAUDE.md ──
    let claude_md_path = claude_dir.join("CLAUDE.md");
    if claude_md_path.exists() {
        let content = fs::read_to_string(&claude_md_path)?;
        match markers::scan(&content) {
            markers::MarkerState::None => {
                println!("CLAUDE.md: no self marker block found (nothing to remove)");
            }
            markers::MarkerState::One { .. } => {
                let state = markers::scan(&content);
                let new_content = markers::remove_block(&content, &state);
                fs::write(&claude_md_path, new_content)?;
                println!("removed block: ~/.claude/CLAUDE.md");
            }
            markers::MarkerState::Malformed(msg) => {
                eprintln!("error: CLAUDE.md has malformed markers — {msg}");
                eprintln!("       refusing to modify CLAUDE.md; fix the markers manually.");
                return Err(Error::MalformedMarkers(msg));
            }
        }
    } else {
        println!("CLAUDE.md: not found (nothing to remove)");
    }

    // ── 2. Delete agent definition files ──
    let agents_dir = claude_dir.join("agents");
    for agent in &["SelfLearning.md", "SelfImproving.md"] {
        let p = agents_dir.join(agent);
        if p.exists() {
            fs::remove_file(&p)?;
            println!("deleted: ~/.claude/agents/{agent}");
        } else {
            println!("not found (already absent): ~/.claude/agents/{agent}");
        }
    }

    // ── 3. Report remaining artifacts ──
    println!();
    println!("The following were NOT removed (use your own judgment):");
    println!("  ~/.self data dir: {}", self_dir.display());

    // List registry-listed skill paths.
    let registry_path = self_dir.join("REGISTRY.md");
    if registry_path.exists() {
        let content = fs::read_to_string(&registry_path)?;
        let (entries, _) = registry::parse(&content);
        if entries.is_empty() {
            println!("  (no learned skills in registry)");
        } else {
            println!("  Learned skills:");
            for e in &entries {
                let path = e.skill_path(&home);
                println!("    {} ({}) — {}", e.slug, e.id, path.display());
            }
        }
    } else {
        println!("  (REGISTRY.md not found — cannot list skill paths)");
    }

    println!();
    println!("settings.json permission rules and ~/.self history remain intact.");

    Ok(())
}
