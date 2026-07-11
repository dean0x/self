/// One parsed run-log entry from `log/runs.md`.
#[derive(Debug, Clone)]
pub struct RunEntry {
    /// UTC timestamp string as written.
    pub timestamp: String,
    /// "learner" or "improver".
    pub agent: String,
    /// Full verdict string.
    pub verdict: String,
    /// Backlog count, if present (learner lines only).
    pub backlog: Option<u64>,
}

/// Parse runs.md content tolerantly.
///
/// Lines that start with `- ` and contain at least a timestamp and agent type
/// are parsed; others are skipped.
///
/// Returns `(entries, total_lines, unparseable_candidates)`.
/// `total_lines` counts all non-blank, non-header lines for the ≤ 200-line cap check.
pub fn parse(content: &str) -> (Vec<RunEntry>, usize, usize) {
    let mut entries = Vec::new();
    let mut total_lines = 0; // count of candidate run-log lines (starting with "- ")
    let mut unparseable = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip blank and comment/header lines.
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("<!--")
            || !trimmed.starts_with("- ")
        {
            continue;
        }
        total_lines += 1;

        match parse_entry(trimmed) {
            Some(e) => entries.push(e),
            None => unparseable += 1,
        }
    }

    (entries, total_lines, unparseable)
}

fn parse_entry(line: &str) -> Option<RunEntry> {
    let line = line.strip_prefix("- ")?;
    let mut parts = line.splitn(3, " | ");
    let timestamp = parts.next()?.trim().to_owned();
    let agent = parts.next()?.trim().to_owned();
    let rest = parts.next().unwrap_or("").trim().to_owned();

    // Agent must be "learner" or "improver" (or starts with one of those).
    let agent_lower = agent.to_lowercase();
    if !agent_lower.starts_with("learner") && !agent_lower.starts_with("improver") {
        return None;
    }

    // Extract verdict from the rest.
    let verdict = extract_field(&rest, "verdict=").unwrap_or_default();

    // Extract backlog from the rest (learner lines only).
    let backlog = extract_counter_field(&rest, "backlog=");

    Some(RunEntry {
        timestamp,
        agent,
        verdict: verdict.to_owned(),
        backlog,
    })
}

fn extract_field<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let start = s.find(key)?;
    let after = &s[start + key.len()..];
    // Value ends at the next " | " separator or end-of-string.
    let end = after.find(" | ").unwrap_or(after.len());
    Some(after[..end].trim())
}

fn extract_counter_field(s: &str, key: &str) -> Option<u64> {
    let start = s.find(key)?;
    let after = s[start + key.len()..].trim_start();
    let end = after
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after.len());
    after[..end].parse().ok()
}

/// Compute the backlog trend from the last up-to-5 learner entries.
///
/// Returns `(values, trend_label)` where `trend_label` ∈ "rising" | "flat" | "falling".
pub fn backlog_trend(entries: &[RunEntry]) -> (Vec<u64>, &'static str) {
    let learner_backlogs: Vec<u64> = entries
        .iter()
        .filter(|e| e.agent.to_lowercase().starts_with("learner"))
        .filter_map(|e| e.backlog)
        .rev() // oldest-first within the last-5
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev() // back to newest-last order so .windows() is chronological
        .collect();

    let trend = compute_trend(&learner_backlogs);
    (learner_backlogs, trend)
}

fn compute_trend(values: &[u64]) -> &'static str {
    if values.len() < 2 {
        return "flat";
    }
    let mut rising = 0usize;
    let mut falling = 0usize;
    for w in values.windows(2) {
        if w[1] > w[0] {
            rising += 1;
        } else if w[1] < w[0] {
            falling += 1;
        }
    }
    if rising > falling {
        "rising"
    } else if falling > rising {
        "falling"
    } else {
        "flat"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CONTENT: &str = "# self run log (append-only; compaction summary lives in this header)\n\n<!-- improver compaction summary: none yet -->\n\nFormat: `- <UTC ISO time> | <learner|improver> | ... | verdict=<...>`\n- 2026-07-04T09:12Z | learner | tool=claude | processed=foo.jsonl | verdict=observed(1) audited(2) | backlog=3\n- 2026-07-04T09:13Z | improver | verdict=no-op\n";

    #[test]
    fn parse_sample() {
        let (entries, total, bad) = parse(SAMPLE_CONTENT);
        assert_eq!(bad, 0);
        assert_eq!(entries.len(), 2);
        assert_eq!(total, 2);
    }

    #[test]
    fn learner_backlog_extracted() {
        let (entries, _, _) = parse(SAMPLE_CONTENT);
        let learner = entries.iter().find(|e| e.agent == "learner").unwrap();
        assert_eq!(learner.backlog, Some(3));
        assert_eq!(learner.verdict, "observed(1) audited(2)");
    }

    #[test]
    fn improver_no_backlog() {
        let (entries, _, _) = parse(SAMPLE_CONTENT);
        let improver = entries.iter().find(|e| e.agent == "improver").unwrap();
        assert_eq!(improver.backlog, None);
        assert_eq!(improver.verdict, "no-op");
    }

    #[test]
    fn parse_skips_malformed() {
        let content = "- 2026-07-01T00:00Z | unknown-agent | verdict=x\n";
        let (entries, _, bad) = parse(content);
        assert_eq!(entries.len(), 0);
        assert_eq!(bad, 1);
    }

    #[test]
    fn backlog_trend_rising() {
        let entries = vec![
            run("learner", Some(1)),
            run("learner", Some(2)),
            run("learner", Some(4)),
        ];
        let (_, trend) = backlog_trend(&entries);
        assert_eq!(trend, "rising");
    }

    #[test]
    fn backlog_trend_falling() {
        let entries = vec![
            run("learner", Some(5)),
            run("learner", Some(3)),
            run("learner", Some(1)),
        ];
        let (_, trend) = backlog_trend(&entries);
        assert_eq!(trend, "falling");
    }

    #[test]
    fn backlog_trend_flat() {
        let entries = vec![
            run("learner", Some(2)),
            run("learner", Some(2)),
            run("learner", Some(2)),
        ];
        let (_, trend) = backlog_trend(&entries);
        assert_eq!(trend, "flat");
    }

    #[test]
    fn backlog_trend_single_value() {
        let entries = vec![run("learner", Some(5))];
        let (_, trend) = backlog_trend(&entries);
        assert_eq!(trend, "flat");
    }

    #[test]
    fn backlog_trend_uses_only_last_5() {
        // 6 entries: first rising, last 5 falling — should report "falling".
        let entries = vec![
            run("learner", Some(1)),
            run("learner", Some(2)), // <-- ignored (6th from end)
            run("learner", Some(10)),
            run("learner", Some(8)),
            run("learner", Some(6)),
            run("learner", Some(4)),
            run("learner", Some(2)),
        ];
        let (values, trend) = backlog_trend(&entries);
        assert_eq!(values.len(), 5);
        assert_eq!(trend, "falling");
    }

    fn run(agent: &str, backlog: Option<u64>) -> RunEntry {
        RunEntry {
            timestamp: "2026-01-01T00:00Z".to_owned(),
            agent: agent.to_owned(),
            verdict: "no-op".to_owned(),
            backlog,
        }
    }
}
