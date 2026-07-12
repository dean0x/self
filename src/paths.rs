use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Resolve the user's home directory.
///
/// Prefers `HOME` (present on Unix, and set by Git Bash / WSL on Windows) and
/// falls back to `USERPROFILE`, the native home variable in Windows shells
/// (cmd.exe, PowerShell) where `HOME` is typically unset. Empty values are
/// treated as unset. Returns [`Error::HomeNotSet`] when neither is usable.
pub fn home() -> Result<PathBuf> {
    for var in ["HOME", "USERPROFILE"] {
        if let Some(val) = std::env::var_os(var) {
            if !val.is_empty() {
                return Ok(PathBuf::from(val));
            }
        }
    }
    Err(Error::HomeNotSet)
}

/// Expand a leading `~` against the given home path.
pub fn expand_tilde(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        home.to_path_buf()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(path)
    }
}

/// `~/.self`
pub fn self_dir(home: &Path) -> PathBuf {
    home.join(".self")
}

/// `~/.claude`
pub fn claude_dir(home: &Path) -> PathBuf {
    home.join(".claude")
}

/// `~/.codex`
pub fn codex_dir(home: &Path) -> PathBuf {
    home.join(".codex")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn expand_tilde_alone() {
        let home = PathBuf::from("/home/user");
        assert_eq!(expand_tilde("~", &home), PathBuf::from("/home/user"));
    }

    #[test]
    fn expand_tilde_with_path() {
        let home = PathBuf::from("/home/user");
        assert_eq!(
            expand_tilde("~/.self", &home),
            PathBuf::from("/home/user/.self")
        );
    }

    #[test]
    fn expand_tilde_no_tilde() {
        let home = PathBuf::from("/home/user");
        assert_eq!(
            expand_tilde("/absolute/path", &home),
            PathBuf::from("/absolute/path")
        );
    }

    #[test]
    fn expand_tilde_relative() {
        let home = PathBuf::from("/home/user");
        assert_eq!(
            expand_tilde("relative/path", &home),
            PathBuf::from("relative/path")
        );
    }
}
