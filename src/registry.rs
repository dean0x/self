use std::path::{Path, PathBuf};

/// One parsed entry from REGISTRY.md.
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub id: String,
    pub slug: String,
    pub scope: String,
    /// Raw path as written in the file (may contain `~`).
    pub raw_path: String,
    pub fired: u64,
    pub applied: u64,
    pub contradicted: u64,
    pub invoked: u64,
    pub refined: u64,
}

impl RegistryEntry {
    /// Resolve the skill file path, expanding `~` against `home`.
    pub fn skill_path(&self, home: &Path) -> PathBuf {
        crate::paths::expand_tilde(&self.raw_path, home)
    }

    /// Total evidence score (used for sorting in status).
    pub fn evidence(&self) -> u64 {
        self.applied + self.invoked
    }
}

/// Parse REGISTRY.md content tolerantly.
///
/// Lines that do not match the expected format are counted but not included in
/// the returned entries. The caller receives a count of unparseable candidate
/// lines (lines that look like registry entries but failed to parse).
pub fn parse(content: &str) -> (Vec<RegistryEntry>, usize) {
    let mut entries = Vec::new();
    let mut unparseable = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        // Candidate: starts with "- S-" followed by digits.
        if !trimmed.starts_with("- S-") {
            continue;
        }
        match parse_entry(trimmed) {
            Some(e) => entries.push(e),
            None => unparseable += 1,
        }
    }

    (entries, unparseable)
}

/// Try to parse a single registry entry line.
///
/// Format:
/// `- S-NNNN | <slug> | <scope> | <path> | created: <date> | src: <...> | fired: N applied: N contradicted: N invoked: N refined: N | flags: <...>`
fn parse_entry(line: &str) -> Option<RegistryEntry> {
    // Strip leading "- "
    let line = line.strip_prefix("- ")?;

    let parts: Vec<&str> = line.splitn(8, " | ").collect();
    if parts.len() < 7 {
        return None;
    }

    let id = parts[0].trim().to_owned();
    if !id.starts_with('S') {
        return None;
    }

    let slug = parts[1].trim().to_owned();
    let scope = parts[2].trim().to_owned();
    let raw_path = parts[3].trim().to_owned();
    // parts[4] = "created: <date>"
    // parts[5] = "src: <...>"
    let counters_str = parts[6].trim();
    // parts[7] = "flags: <...>" (optional, may be absent or last field of splitn)

    let (fired, applied, contradicted, invoked, refined) = parse_counters(counters_str)?;

    Some(RegistryEntry {
        id,
        slug,
        scope,
        raw_path,
        fired,
        applied,
        contradicted,
        invoked,
        refined,
    })
}

/// Parse `fired: N applied: N contradicted: N invoked: N refined: N`.
fn parse_counters(s: &str) -> Option<(u64, u64, u64, u64, u64)> {
    let fired = extract_counter(s, "fired:")?;
    let applied = extract_counter(s, "applied:")?;
    let contradicted = extract_counter(s, "contradicted:")?;
    let invoked = extract_counter(s, "invoked:")?;
    let refined = extract_counter(s, "refined:")?;
    Some((fired, applied, contradicted, invoked, refined))
}

fn extract_counter(s: &str, key: &str) -> Option<u64> {
    let start = s.find(key)?;
    let after = s[start + key.len()..].trim_start();
    // Read digits until non-digit.
    let end = after
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after.len());
    after[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_LINE: &str = "- S-0042 | tidy-imports | user | ~/.claude/skills/tidy-imports/SKILL.md | created: 2026-01-15 | src: obs-0007+obs-0019 | fired: 3 applied: 2 contradicted: 0 invoked: 1 refined: 0 | flags: -";

    #[test]
    fn parse_single_entry() {
        let (entries, bad) = parse(SAMPLE_LINE);
        assert_eq!(bad, 0);
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.id, "S-0042");
        assert_eq!(e.slug, "tidy-imports");
        assert_eq!(e.scope, "user");
        assert_eq!(e.raw_path, "~/.claude/skills/tidy-imports/SKILL.md");
        assert_eq!(e.fired, 3);
        assert_eq!(e.applied, 2);
        assert_eq!(e.contradicted, 0);
        assert_eq!(e.invoked, 1);
        assert_eq!(e.refined, 0);
    }

    #[test]
    fn parse_skips_non_entry_lines() {
        let content = "# Registry\n\nSome description line.\n".to_owned() + SAMPLE_LINE + "\n";
        let (entries, bad) = parse(&content);
        assert_eq!(bad, 0);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn parse_counts_malformed_candidates() {
        // A line that starts like an entry but is malformed.
        let content = "- S-9999 | only-two-fields\n";
        let (entries, bad) = parse(content);
        assert_eq!(entries.len(), 0);
        assert_eq!(bad, 1);
    }

    #[test]
    fn parse_multiple_entries() {
        let line2 = "- S-0002 | my-skill | project:myrepo | /repo/.claude/skills/my-skill/SKILL.md | created: 2026-07-06 | src: obs-0003+obs-0004 | fired: 0 applied: 0 contradicted: 0 invoked: 0 refined: 0 | flags: -";
        let content = format!("{SAMPLE_LINE}\n{line2}\n");
        let (entries, bad) = parse(&content);
        assert_eq!(bad, 0);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].id, "S-0002");
        assert_eq!(entries[1].scope, "project:myrepo");
    }

    #[test]
    fn tilde_expansion_in_path() {
        let (entries, _) = parse(SAMPLE_LINE);
        let home = std::path::PathBuf::from("/home/testuser");
        let path = entries[0].skill_path(&home);
        assert_eq!(
            path,
            std::path::PathBuf::from("/home/testuser/.claude/skills/tidy-imports/SKILL.md")
        );
    }

    #[test]
    fn evidence_is_applied_plus_invoked() {
        let (entries, _) = parse(SAMPLE_LINE);
        assert_eq!(entries[0].evidence(), 3); // applied=2 + invoked=1
    }
}
