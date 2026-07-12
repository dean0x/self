use std::fs;
use std::path::Path;

use crate::error::{Error, Result};
use crate::markers;
use crate::paths;
use crate::registry;
use crate::runlog;
use crate::settings::REQUIRED_PERMISSIONS;

/// Caps from C4.
const MAX_OPEN_OBSERVATIONS: usize = 50;
const MAX_RUN_LINES: usize = 200;
const MAX_BLOCK_LINES: usize = 25;

pub fn run() -> Result<()> {
    let home = paths::home()?;
    let self_dir = paths::self_dir(&home);
    let claude_dir = paths::claude_dir(&home);

    if !self_dir.exists() {
        return Err(Error::NoSelfDir);
    }

    let mut findings: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // ── 1. Registry ↔ skill files ──
    let registry_path = self_dir.join("REGISTRY.md");
    let registry_content = if registry_path.exists() {
        fs::read_to_string(&registry_path)?
    } else {
        findings.push("REGISTRY.md missing from ~/.self/".to_owned());
        String::new()
    };

    let (entries, reg_bad) = registry::parse(&registry_content);

    if reg_bad > 0 {
        warnings.push(format!("{reg_bad} unparseable registry line(s)"));
    }

    for entry in &entries {
        let skill_path = entry.skill_path(&home);
        if !skill_path.exists() {
            findings.push(format!(
                "dangling registry entry {}: skill file not found at {}",
                entry.id,
                skill_path.display()
            ));
            continue;
        }

        // Check path location agrees with scope.
        let scope_ok = check_scope_vs_path(&entry.scope, &skill_path, &home);
        if !scope_ok {
            findings.push(format!(
                "scope mismatch for {}: scope='{}' but path={}",
                entry.id,
                entry.scope,
                skill_path.display()
            ));
        }

        // Check YAML frontmatter.
        match fs::read_to_string(&skill_path) {
            Err(e) => {
                findings.push(format!("could not read skill file for {}: {e}", entry.id));
            }
            Ok(content) => {
                let fm_result = check_frontmatter(&content);
                match fm_result {
                    FrontmatterCheck::Ok { description_words } => {
                        if description_words > 25 {
                            warnings.push(format!(
                                "description for {} is {} words (C4: ≤ 25 words)",
                                entry.slug, description_words
                            ));
                        }
                    }
                    FrontmatterCheck::MissingName => {
                        findings.push(format!(
                            "skill file for {} missing 'name:' in YAML frontmatter",
                            entry.id
                        ));
                    }
                    FrontmatterCheck::MissingDescription => {
                        findings.push(format!(
                            "skill file for {} missing 'description:' in YAML frontmatter",
                            entry.id
                        ));
                    }
                    FrontmatterCheck::NoFrontmatter => {
                        findings.push(format!(
                            "skill file for {} has no YAML frontmatter",
                            entry.id
                        ));
                    }
                }
            }
        }
    }

    // ── 2. CLAUDE.md marker block ──
    let claude_md_path = claude_dir.join("CLAUDE.md");
    if claude_md_path.exists() {
        match fs::read_to_string(&claude_md_path) {
            Err(e) => findings.push(format!("could not read CLAUDE.md: {e}")),
            Ok(content) => match markers::scan(&content) {
                markers::MarkerState::None => {
                    findings.push("CLAUDE.md has no self marker block".to_owned());
                }
                markers::MarkerState::One {
                    region_start,
                    region_end,
                } => {
                    let block = &content[region_start..region_end];
                    let line_count = block.lines().count();
                    if line_count > MAX_BLOCK_LINES {
                        findings.push(format!(
                                "CLAUDE.md marker block is {line_count} lines (C4: ≤ {MAX_BLOCK_LINES})"
                            ));
                    }
                }
                markers::MarkerState::Malformed(msg) => {
                    findings.push(format!("CLAUDE.md has malformed markers: {msg}"));
                }
            },
        }
    } else {
        findings.push("~/.claude/CLAUDE.md does not exist".to_owned());
    }

    // ── 3. Agent definition files ──
    let agents_dir = claude_dir.join("agents");
    for agent in &["SelfLearning.md", "SelfImproving.md"] {
        let p = agents_dir.join(agent);
        if !p.exists() {
            findings.push(format!(
                "agent definition missing: ~/.claude/agents/{agent}"
            ));
        }
    }

    // ── 4. settings.json permissions ──
    let settings_path = claude_dir.join("settings.json");
    if settings_path.exists() {
        match fs::read_to_string(&settings_path) {
            Err(e) => findings.push(format!("could not read settings.json: {e}")),
            Ok(raw) => match crate::settings::merge_permissions(&raw) {
                Err(e) => findings.push(format!("settings.json invalid JSON: {e}")),
                Ok((_, missing)) => {
                    for rule in &missing {
                        findings.push(format!("missing permission rule in settings.json: {rule}"));
                    }
                }
            },
        }
    } else {
        for rule in REQUIRED_PERMISSIONS {
            findings.push(format!("settings.json absent; missing rule: {rule}"));
        }
    }

    // ── 5. Open observations count ──
    let obs_path = self_dir.join("observations.md");
    if obs_path.exists() {
        match fs::read_to_string(&obs_path) {
            Err(e) => findings.push(format!("could not read observations.md: {e}")),
            Ok(content) => {
                let open_count = content.lines().filter(|l| l.contains("| open |")).count();
                if open_count > MAX_OPEN_OBSERVATIONS {
                    findings.push(format!(
                        "open observations: {open_count} (C4: ≤ {MAX_OPEN_OBSERVATIONS})"
                    ));
                }
            }
        }
    }

    // ── 6. runs.md line count ──
    let runs_path = self_dir.join("log").join("runs.md");
    if runs_path.exists() {
        match fs::read_to_string(&runs_path) {
            Err(e) => findings.push(format!("could not read log/runs.md: {e}")),
            Ok(content) => {
                let (_, total_lines, _) = runlog::parse(&content);
                if total_lines > MAX_RUN_LINES {
                    findings.push(format!(
                        "log/runs.md has {total_lines} entries (C4: ≤ {MAX_RUN_LINES})"
                    ));
                }
            }
        }
    }

    // ── Print report ──
    println!("=== self doctor ===");
    println!();
    if warnings.is_empty() && findings.is_empty() {
        println!("clean — no findings");
    } else {
        for w in &warnings {
            println!("WARN  {w}");
        }
        for f in &findings {
            println!("FIND  {f}");
        }
    }

    if !findings.is_empty() {
        return Err(Error::Other(format!(
            "doctor found {} finding(s)",
            findings.len()
        )));
    }

    Ok(())
}

/// Verify that the skill file's path is consistent with its declared scope.
fn check_scope_vs_path(scope: &str, skill_path: &Path, home: &Path) -> bool {
    if scope == "user" {
        // Should be under ~/.claude/skills/ or ~/.agents/skills/ (user-scope locations).
        let user_claude = home.join(".claude").join("skills");
        let user_agents = home.join(".agents").join("skills");
        skill_path.starts_with(&user_claude) || skill_path.starts_with(&user_agents)
    } else if scope.starts_with("project:") {
        // Should be under <repo>/.claude/skills/ or <repo>/.agents/skills/.
        // We can't know the exact repo path, but it should NOT be under the home user dirs.
        let user_claude = home.join(".claude").join("skills");
        let user_agents = home.join(".agents").join("skills");
        !skill_path.starts_with(&user_claude) && !skill_path.starts_with(&user_agents)
    } else {
        // Unknown scope — pass (don't generate a false positive).
        true
    }
}

#[derive(Debug)]
enum FrontmatterCheck {
    Ok { description_words: usize },
    MissingName,
    MissingDescription,
    NoFrontmatter,
}

/// Check that a skill file has YAML frontmatter with `name:` and `description:`.
fn check_frontmatter(content: &str) -> FrontmatterCheck {
    if !content.starts_with("---") {
        return FrontmatterCheck::NoFrontmatter;
    }
    // Find the closing `---`.
    let after_open = &content[3..];
    let close = after_open.find("\n---").map(|i| i + 3);
    let frontmatter = match close {
        Some(end) => &content[3..end],
        None => return FrontmatterCheck::NoFrontmatter,
    };

    let has_name = frontmatter
        .lines()
        .any(|l| l.trim_start().starts_with("name:"));
    let desc_line = frontmatter
        .lines()
        .find(|l| l.trim_start().starts_with("description:"));

    if !has_name {
        return FrontmatterCheck::MissingName;
    }
    let Some(desc) = desc_line else {
        return FrontmatterCheck::MissingDescription;
    };
    // Count words in the description value.
    let value = desc
        .trim_start()
        .strip_prefix("description:")
        .unwrap_or("")
        .trim();
    let word_count = value.split_whitespace().count();
    FrontmatterCheck::Ok {
        description_words: word_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── check_frontmatter ────────────────────────────────────────────────────

    #[test]
    fn frontmatter_ok_minimal() {
        let content = "---\nname: tidy-imports\ndescription: removes unused imports\n---\n\nbody\n";
        match check_frontmatter(content) {
            FrontmatterCheck::Ok { description_words } => assert_eq!(description_words, 3),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn frontmatter_missing_name() {
        let content = "---\ndescription: removes unused imports\n---\n\nbody\n";
        assert!(matches!(
            check_frontmatter(content),
            FrontmatterCheck::MissingName
        ));
    }

    #[test]
    fn frontmatter_missing_description() {
        let content = "---\nname: tidy-imports\n---\n\nbody\n";
        assert!(matches!(
            check_frontmatter(content),
            FrontmatterCheck::MissingDescription
        ));
    }

    #[test]
    fn frontmatter_no_frontmatter() {
        let content = "no frontmatter here\nname: tidy-imports\n";
        assert!(matches!(
            check_frontmatter(content),
            FrontmatterCheck::NoFrontmatter
        ));
    }

    #[test]
    fn frontmatter_long_description_exceeds_cap() {
        // 26 words — should parse as Ok with a high count so the caller can warn.
        let words = "one two three four five six seven eight nine ten \
                     eleven twelve thirteen fourteen fifteen sixteen seventeen \
                     eighteen nineteen twenty twenty-one twenty-two twenty-three \
                     twenty-four twenty-five twenty-six";
        let content = format!("---\nname: verbose\ndescription: {words}\n---\n");
        match check_frontmatter(&content) {
            FrontmatterCheck::Ok { description_words } => {
                assert!(
                    description_words > 25,
                    "expected > 25 words, got {description_words}"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    // ── check_scope_vs_path ──────────────────────────────────────────────────

    #[test]
    fn scope_user_path_under_claude_skills_is_ok() {
        let home = PathBuf::from("/home/user");
        let path = home.join(".claude/skills/tidy-imports/SKILL.md");
        assert!(check_scope_vs_path("user", &path, &home));
    }

    #[test]
    fn scope_user_path_under_agents_skills_is_ok() {
        let home = PathBuf::from("/home/user");
        let path = home.join(".agents/skills/tidy-imports/SKILL.md");
        assert!(check_scope_vs_path("user", &path, &home));
    }

    #[test]
    fn scope_user_path_outside_user_dirs_is_mismatch() {
        let home = PathBuf::from("/home/user");
        let path = PathBuf::from("/some/project/.claude/skills/tidy-imports/SKILL.md");
        assert!(!check_scope_vs_path("user", &path, &home));
    }

    #[test]
    fn scope_project_path_not_in_user_dirs_is_ok() {
        let home = PathBuf::from("/home/user");
        let path = PathBuf::from("/some/project/.claude/skills/tidy-imports/SKILL.md");
        assert!(check_scope_vs_path("project:some-project", &path, &home));
    }

    #[test]
    fn scope_project_path_in_user_claude_dir_is_mismatch() {
        let home = PathBuf::from("/home/user");
        let path = home.join(".claude/skills/tidy-imports/SKILL.md");
        assert!(!check_scope_vs_path("project:some-project", &path, &home));
    }

    #[test]
    fn scope_unknown_passes_without_false_positive() {
        let home = PathBuf::from("/home/user");
        let path = PathBuf::from("/anywhere/SKILL.md");
        assert!(check_scope_vs_path("unknown-scope", &path, &home));
    }
}
