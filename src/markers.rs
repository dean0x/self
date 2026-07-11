/// Result of scanning a file for `<!-- self:start … --> … <!-- self:end -->` markers.
#[derive(Debug, PartialEq)]
pub enum MarkerState {
    /// No markers present.
    None,
    /// Exactly one well-formed pair.
    One {
        /// Byte offset of the first byte of the start-marker line.
        region_start: usize,
        /// Byte offset one past the last byte of the end-marker line
        /// (including its newline if present).
        region_end: usize,
    },
    /// Malformed: mismatched, multiple, or out-of-order markers.
    Malformed(String),
}

/// Scan `content` and return the marker state.
pub fn scan(content: &str) -> MarkerState {
    let mut starts: Vec<(usize, usize)> = Vec::new(); // (line_index, byte_start)
    let mut ends: Vec<(usize, usize)> = Vec::new(); // (line_index, byte_end_exclusive)

    let mut byte_pos: usize = 0;
    for (line_idx, line) in content.split_inclusive('\n').enumerate() {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.contains("<!-- self:start") {
            starts.push((line_idx, byte_pos));
        }
        if trimmed.contains("<!-- self:end -->") {
            ends.push((line_idx, byte_pos + line.len()));
        }
        byte_pos += line.len();
    }

    // Handle a trailing line that has no '\n'.
    // split_inclusive already handles it correctly; byte_pos is now == content.len().

    match (starts.len(), ends.len()) {
        (0, 0) => MarkerState::None,
        (1, 1) => {
            let (si, s_byte) = starts[0];
            let (ei, e_byte) = ends[0];
            if si < ei {
                MarkerState::One {
                    region_start: s_byte,
                    region_end: e_byte,
                }
            } else {
                MarkerState::Malformed(
                    "<!-- self:end --> appears before <!-- self:start -->".to_owned(),
                )
            }
        }
        (s, e) => MarkerState::Malformed(format!(
            "found {s} start marker(s) and {e} end marker(s); expected exactly 1 of each"
        )),
    }
}

/// Replace the marker block in `content` with `new_block`.
///
/// `new_block` must include the start and end marker lines.
/// Everything outside the markers is byte-preserved.
pub fn replace_block(content: &str, new_block: &str, state: &MarkerState) -> String {
    match state {
        MarkerState::One {
            region_start,
            region_end,
        } => {
            let prefix = &content[..*region_start];
            let suffix = &content[*region_end..];
            format!("{prefix}{new_block}{suffix}")
        }
        _ => panic!("replace_block called with non-One MarkerState"),
    }
}

/// Append `block` to `content` with exactly one blank line separating them.
///
/// `block` must include the start and end marker lines.
pub fn append_block(content: &str, block: &str) -> String {
    // Determine the separator needed so there is exactly one blank line.
    let sep = if content.is_empty() || content.ends_with("\n\n") {
        ""
    } else if content.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    format!("{content}{sep}{block}")
}

/// Remove the marker block from `content`, returning content with the block stripped.
///
/// The content outside the markers is byte-preserved.
pub fn remove_block(content: &str, state: &MarkerState) -> String {
    match state {
        MarkerState::One {
            region_start,
            region_end,
        } => {
            let prefix = &content[..*region_start];
            let suffix = &content[*region_end..];
            format!("{prefix}{suffix}")
        }
        MarkerState::None => content.to_owned(),
        MarkerState::Malformed(_) => panic!("remove_block called with Malformed MarkerState"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "<!-- self:start v0.3 -->\n## self\ncontent here\n<!-- self:end -->\n";

    #[test]
    fn scan_empty() {
        assert_eq!(scan(""), MarkerState::None);
    }

    #[test]
    fn scan_no_markers() {
        assert_eq!(scan("just some text\nno markers here\n"), MarkerState::None);
    }

    #[test]
    fn scan_one_pair() {
        let content = BLOCK;
        match scan(content) {
            MarkerState::One {
                region_start,
                region_end,
            } => {
                assert_eq!(region_start, 0);
                assert_eq!(region_end, content.len());
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn scan_one_pair_with_prefix_and_suffix() {
        let prefix = "existing content\n";
        let suffix = "trailing content\n";
        let content = format!("{prefix}{BLOCK}{suffix}");
        match scan(&content) {
            MarkerState::One {
                region_start,
                region_end,
            } => {
                assert_eq!(region_start, prefix.len());
                assert_eq!(region_end, prefix.len() + BLOCK.len());
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn scan_start_without_end() {
        let content = "before\n<!-- self:start v0.3 -->\ncontent\nafter\n";
        assert!(matches!(scan(content), MarkerState::Malformed(_)));
    }

    #[test]
    fn scan_end_without_start() {
        let content = "before\ncontent\n<!-- self:end -->\nafter\n";
        assert!(matches!(scan(content), MarkerState::Malformed(_)));
    }

    #[test]
    fn scan_two_pairs() {
        let content = format!("{BLOCK}{BLOCK}");
        assert!(matches!(scan(&content), MarkerState::Malformed(_)));
    }

    #[test]
    fn scan_end_before_start() {
        let content = "<!-- self:end -->\nsome content\n<!-- self:start v0.3 -->\nstuff\n";
        assert!(matches!(scan(content), MarkerState::Malformed(_)));
    }

    #[test]
    fn replace_block_preserves_outside_bytes() {
        let prefix = "line one\nline two\n";
        let suffix = "line three\nline four\n";
        let old_block = "<!-- self:start v0.3 -->\nold content\n<!-- self:end -->\n";
        let new_block = "<!-- self:start v0.3 -->\nnew content\n<!-- self:end -->\n";
        let content = format!("{prefix}{old_block}{suffix}");
        let state = scan(&content);
        let result = replace_block(&content, new_block, &state);
        assert_eq!(result, format!("{prefix}{new_block}{suffix}"));
        // Prefix bytes preserved exactly.
        assert!(result.starts_with(prefix));
        // Suffix bytes preserved exactly.
        assert!(result.ends_with(suffix));
    }

    #[test]
    fn replace_block_idempotent() {
        let content = format!("preamble\n{BLOCK}");
        let state = scan(&content);
        let result = replace_block(&content, BLOCK, &state);
        assert_eq!(result, content);
    }

    #[test]
    fn append_block_empty_file() {
        let result = append_block("", BLOCK);
        assert_eq!(result, BLOCK);
    }

    #[test]
    fn append_block_single_newline() {
        let existing = "existing content\n";
        let result = append_block(existing, BLOCK);
        assert_eq!(result, format!("{existing}\n{BLOCK}"));
    }

    #[test]
    fn append_block_double_newline() {
        let existing = "existing content\n\n";
        let result = append_block(existing, BLOCK);
        assert_eq!(result, format!("{existing}{BLOCK}"));
    }

    #[test]
    fn append_block_no_trailing_newline() {
        let existing = "existing content";
        let result = append_block(existing, BLOCK);
        assert_eq!(result, format!("{existing}\n\n{BLOCK}"));
    }

    #[test]
    fn remove_block_from_middle() {
        let prefix = "before\n";
        let suffix = "after\n";
        let content = format!("{prefix}{BLOCK}{suffix}");
        let state = scan(&content);
        let result = remove_block(&content, &state);
        assert_eq!(result, format!("{prefix}{suffix}"));
    }

    #[test]
    fn remove_block_none_is_noop() {
        let content = "no markers here\n";
        let result = remove_block(content, &MarkerState::None);
        assert_eq!(result, content);
    }

    #[test]
    fn outside_content_byte_preserved_after_replace() {
        // Verify non-ASCII bytes outside markers are preserved exactly.
        let prefix = "héllo\n";
        let suffix = "wörld\n";
        let old = "<!-- self:start v0.3 -->\nold\n<!-- self:end -->\n";
        let new = "<!-- self:start v0.3 -->\nnew\n<!-- self:end -->\n";
        let content = format!("{prefix}{old}{suffix}");
        let state = scan(&content);
        let result = replace_block(&content, new, &state);
        let result_prefix = &result[..prefix.len()];
        let result_suffix = &result[result.len() - suffix.len()..];
        assert_eq!(result_prefix.as_bytes(), prefix.as_bytes());
        assert_eq!(result_suffix.as_bytes(), suffix.as_bytes());
    }
}
