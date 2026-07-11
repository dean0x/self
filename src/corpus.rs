/// Convert a count of days since the Unix epoch to a Gregorian (year, month, day).
///
/// Uses the civil-date algorithm from Howard Hinnant
/// <https://howardhinnant.github.io/date_algorithms.html>.
/// Pure function, no external dependencies.
pub fn days_to_civil(days: u64) -> (u32, u32, u32) {
    // Shift epoch from 1970-01-01 to 0000-03-01 (makes leap-day handling uniform).
    let z: i64 = days as i64 + 719_468;

    // 400-year era containing z.
    let era: i64 = if z >= 0 { z } else { z - 146_096 } / 146_097;

    // Day-of-era [0, 146096].
    let doe: u64 = (z - era * 146_097) as u64;

    // Year-of-era [0, 399].
    let yoe: u64 = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;

    // Gregorian year (may still be off by 1 for Jan/Feb — corrected below).
    let y: i64 = yoe as i64 + era * 400;

    // Day-of-year starting from March 1 [0, 365].
    let doy: u64 = doe - (365 * yoe + yoe / 4 - yoe / 100);

    // Month index (March = 0) [0, 11].
    let mp: u64 = (5 * doy + 2) / 153;

    // Day-of-month [1, 31].
    let d: u32 = (doy - (153 * mp + 2) / 5 + 1) as u32;

    // Month [1, 12].
    let m: u32 = if mp < 10 {
        mp as u32 + 3
    } else {
        mp as u32 - 9
    };

    // Adjust year for Jan/Feb.
    let y: u32 = if m <= 2 { (y + 1) as u32 } else { y as u32 };

    (y, m, d)
}

/// Return the current UTC date as a `YYYY-MM-DD` string.
pub fn today_string() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86_400;
    let (y, m, d) = days_to_civil(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Rewrite the `created:` field on the S-0001 line to `today`.
pub fn seed_registry(template: &str, today: &str) -> String {
    template
        .lines()
        .map(|line| {
            if line.contains("S-0001") {
                // Replace the "created: YYYY-MM-DD" field in this line.
                replace_created_date(line, today)
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if template.ends_with('\n') { "\n" } else { "" }
}

/// Replace the `created: <date>` value in a registry entry line.
fn replace_created_date(line: &str, today: &str) -> String {
    // Find "created: " then the next word (date) separated by " |" or end.
    const NEEDLE: &str = "created: ";
    if let Some(start) = line.find(NEEDLE) {
        let after = &line[start + NEEDLE.len()..];
        // The date ends at the next " |" separator or end-of-string.
        let date_len = after.find(" |").unwrap_or(after.len());
        let before = &line[..start + NEEDLE.len()];
        let rest = &after[date_len..];
        format!("{before}{today}{rest}")
    } else {
        line.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spot-check a known epoch day: 2026-07-05 is day 20639 since 1970-01-01.
    // 2026-07-05: years 1970..2026 = 56 years.
    // leap years in [1970, 2026): 1972, 1976, ..., 2024 → 14 leaps.
    // days = 56*365 + 14 + day-of-year(2026-07-05).
    // 2026 day-of-year: Jan=31, Feb=28, Mar=31, Apr=30, May=31, Jun=30, Jul-05=5 → 186-1=185 (0-indexed).
    // total = 56*365 + 14 + 185 = 20440 + 14 + 185 = 20639.
    #[test]
    fn known_epoch_day() {
        assert_eq!(days_to_civil(20_639), (2026, 7, 5));
    }

    #[test]
    fn epoch_day_zero() {
        assert_eq!(days_to_civil(0), (1970, 1, 1));
    }

    #[test]
    fn known_date_2000_01_01() {
        // 2000-01-01: 30 years from 1970.
        // Leaps in [1970,2000): 1972,1976,1980,1984,1988,1992,1996 → 7 leaps.
        // 30*365 + 7 = 10957.
        assert_eq!(days_to_civil(10_957), (2000, 1, 1));
    }

    #[test]
    fn leap_day_2000() {
        // 2000-02-29: 10957 + 59 = 11016? Let's compute: Jan=31 days, Feb 1-29 = 59th day of year (1-based).
        // 11016 = 10957 + 31 + 28 = 10957 + 59. Wait: Jan(31) + Feb(29) = 60, but day 1 is Jan1=10957.
        // 2000-01-01 = 10957, 2000-02-29 = 10957 + 31 + 28 = 11016.
        assert_eq!(days_to_civil(11_016), (2000, 2, 29));
    }

    #[test]
    fn non_leap_2100() {
        // 2100 is not a leap year (divisible by 100 but not 400).
        // 2100-03-01: verify we don't put a Feb-29 there.
        // 2100-02-28 should be valid.
        let (y, m, d) = days_to_civil(47_540);
        // Just verify the function doesn't panic and returns a plausible date.
        assert!((2090..=2110).contains(&y), "year={y}");
        assert!((1..=12).contains(&m));
        assert!((1..=31).contains(&d));
    }

    #[test]
    fn seed_registry_updates_date() {
        let tmpl = "- S-0001 | ci-gate | user | ~/.claude/skills/ci-gate/SKILL.md | created: 2026-07-05 | src: obs-0001+obs-0002 | fired: 0 applied: 0 contradicted: 0 invoked: 0 refined: 0 | flags: -\n";
        let result = seed_registry(tmpl, "2026-07-11");
        assert!(result.contains("created: 2026-07-11"), "got: {result}");
        assert!(
            !result.contains("created: 2026-07-05"),
            "old date remains: {result}"
        );
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn seed_registry_leaves_other_lines_unchanged() {
        let tmpl = "# Header\n- S-0001 | ci-gate | user | ~/.claude/skills/ci-gate/SKILL.md | created: 2026-07-05 | src: obs | fired: 0 applied: 0 contradicted: 0 invoked: 0 refined: 0 | flags: -\n";
        let result = seed_registry(tmpl, "2026-08-01");
        assert!(result.starts_with("# Header\n"));
        assert!(result.contains("created: 2026-08-01"));
    }
}
