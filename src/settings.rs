use serde_json::{Value, json};

use crate::error::{Error, Result};

/// The nine permission rules that `self init` ensures are present in
/// `~/.claude/settings.json` under `permissions.allow`.
pub const REQUIRED_PERMISSIONS: &[&str] = &[
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

/// Parse `json_text` and merge any missing permission rules into
/// `permissions.allow`, preserving all existing keys and their order.
///
/// Returns `(updated_value, rules_added)` where `rules_added` is the list of
/// rules that were missing and have been appended.
///
/// Returns `Err` only if the JSON is invalid.
pub fn merge_permissions(json_text: &str) -> Result<(Value, Vec<String>)> {
    let mut root: Value = serde_json::from_str(json_text)
        .map_err(|e| Error::InvalidJson(format!("settings.json: {e}")))?;

    let added = add_missing_permissions(&mut root);
    Ok((root, added))
}

/// Build a minimal settings.json value with only the required permissions.
pub fn minimal_settings() -> Value {
    let rules: Vec<Value> = REQUIRED_PERMISSIONS.iter().map(|r| json!(r)).collect();
    json!({
        "permissions": {
            "allow": rules
        }
    })
}

/// Add any missing required permissions to `root["permissions"]["allow"]`.
/// Returns the list of added rules.
fn add_missing_permissions(root: &mut Value) -> Vec<String> {
    // Ensure root is an object.
    if !root.is_object() {
        *root = json!({});
    }

    // Ensure permissions is an object (coerce any other shape, e.g. [] or "strict").
    if !root["permissions"].is_object() {
        root["permissions"] = json!({});
    }

    let perms = root["permissions"]
        .as_object_mut()
        .expect("permissions is object"); // unreachable: coerced above

    // Ensure allow is an array (coerce any other shape, e.g. {} or "*").
    // Use .get() not perms["allow"] — Map's Index panics on missing keys.
    if !perms.get("allow").is_some_and(Value::is_array) {
        perms.insert("allow".to_owned(), json!([]));
    }

    let allow = perms["allow"].as_array_mut().expect("allow is array"); // unreachable: coerced above

    let mut added = Vec::new();
    for &rule in REQUIRED_PERMISSIONS {
        let already = allow.iter().any(|v| v.as_str() == Some(rule));
        if !already {
            allow.push(json!(rule));
            added.push(rule.to_owned());
        }
    }
    added
}

/// Serialize a `Value` to a pretty-printed JSON string with 2-space indent.
pub fn to_pretty_json(value: &Value) -> Result<String> {
    serde_json::to_string_pretty(value).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_adds_all_when_empty_object() {
        let (val, added) = merge_permissions("{}").unwrap();
        assert_eq!(added.len(), REQUIRED_PERMISSIONS.len());
        let allow = val["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), REQUIRED_PERMISSIONS.len());
    }

    #[test]
    fn merge_adds_only_missing() {
        let existing = r#"{
  "permissions": {
    "allow": ["Read(~/.claude/projects/**)"]
  }
}"#;
        let (val, added) = merge_permissions(existing).unwrap();
        // One already existed, rest should be added.
        assert_eq!(added.len(), REQUIRED_PERMISSIONS.len() - 1);
        assert!(!added.contains(&"Read(~/.claude/projects/**)".to_owned()));
        let allow = val["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), REQUIRED_PERMISSIONS.len());
        // First entry is the pre-existing one (order preserved).
        assert_eq!(allow[0].as_str().unwrap(), "Read(~/.claude/projects/**)");
    }

    #[test]
    fn merge_preserves_unknown_keys() {
        let existing = r#"{
  "someOtherKey": "value",
  "permissions": {
    "allow": [],
    "deny": ["Bash(rm -rf *)"]
  }
}"#;
        let (val, _) = merge_permissions(existing).unwrap();
        assert_eq!(val["someOtherKey"].as_str().unwrap(), "value");
        // deny rule preserved.
        let deny = val["permissions"]["deny"].as_array().unwrap();
        assert_eq!(deny[0].as_str().unwrap(), "Bash(rm -rf *)");
    }

    #[test]
    fn merge_preserves_key_order() {
        // With preserve_order feature, key insertion order is maintained.
        let existing = r#"{"z": 1, "a": 2, "permissions": {"allow": []}}"#;
        let (val, _) = merge_permissions(existing).unwrap();
        let serialized = serde_json::to_string(&val).unwrap();
        // "z" should appear before "a" in the output.
        let z_pos = serialized.find("\"z\"").unwrap();
        let a_pos = serialized.find("\"a\"").unwrap();
        assert!(z_pos < a_pos, "key order not preserved: {serialized}");
    }

    #[test]
    fn merge_no_op_when_all_present() {
        let mut json_str = String::from(r#"{"permissions":{"allow":["#);
        for (i, rule) in REQUIRED_PERMISSIONS.iter().enumerate() {
            if i > 0 {
                json_str.push(',');
            }
            json_str.push('"');
            json_str.push_str(rule);
            json_str.push('"');
        }
        json_str.push_str("]}}");

        let (_, added) = merge_permissions(&json_str).unwrap();
        assert!(added.is_empty());
    }

    #[test]
    fn merge_errors_on_invalid_json() {
        let result = merge_permissions("not json");
        assert!(result.is_err());
    }

    #[test]
    fn merge_handles_permissions_non_object() {
        // permissions is a JSON array, not an object — must not panic.
        let (val, added) = merge_permissions(r#"{"permissions": []}"#).unwrap();
        assert_eq!(added.len(), REQUIRED_PERMISSIONS.len());
        assert_eq!(
            val["permissions"]["allow"].as_array().unwrap().len(),
            REQUIRED_PERMISSIONS.len()
        );
    }

    #[test]
    fn merge_handles_permissions_string() {
        // permissions is a string — must not panic.
        let (_, added) = merge_permissions(r#"{"permissions": "strict"}"#).unwrap();
        assert_eq!(added.len(), REQUIRED_PERMISSIONS.len());
    }

    #[test]
    fn merge_handles_allow_non_array() {
        // allow is an object, not an array — must not panic.
        let (val, added) = merge_permissions(r#"{"permissions": {"allow": {}}}"#).unwrap();
        assert_eq!(added.len(), REQUIRED_PERMISSIONS.len());
        assert_eq!(
            val["permissions"]["allow"].as_array().unwrap().len(),
            REQUIRED_PERMISSIONS.len()
        );
    }

    #[test]
    fn merge_handles_allow_string() {
        // allow is a string (confirmed panic case) — must not panic.
        let (_, added) = merge_permissions(r#"{"permissions": {"allow": "*"}}"#).unwrap();
        assert_eq!(added.len(), REQUIRED_PERMISSIONS.len());
    }

    #[test]
    fn merge_handles_missing_permissions_key() {
        let existing = r#"{"other": "data"}"#;
        let (val, added) = merge_permissions(existing).unwrap();
        assert_eq!(added.len(), REQUIRED_PERMISSIONS.len());
        assert_eq!(val["other"].as_str().unwrap(), "data");
    }

    #[test]
    fn merge_handles_missing_allow_key() {
        let existing = r#"{"permissions": {"deny": ["Bash(rm *)"]}}"#;
        let (val, added) = merge_permissions(existing).unwrap();
        assert_eq!(added.len(), REQUIRED_PERMISSIONS.len());
        // deny is preserved.
        let deny = val["permissions"]["deny"].as_array().unwrap();
        assert_eq!(deny[0].as_str().unwrap(), "Bash(rm *)");
    }

    #[test]
    fn minimal_settings_has_all_rules() {
        let val = minimal_settings();
        let allow = val["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), REQUIRED_PERMISSIONS.len());
    }
}
