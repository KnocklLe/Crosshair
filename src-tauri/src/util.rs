//! Small shared helpers.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current time in milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// New random identifier.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Returns `base`, or `base (2)`, `base (3)`… so that it does not collide with
/// any name in `taken` (case-insensitive). The result respects `max_chars`.
pub fn unique_name<'a>(base: &str, taken: impl IntoIterator<Item = &'a str> + Clone, max_chars: usize) -> String {
    let exists = |candidate: &str| {
        taken
            .clone()
            .into_iter()
            .any(|t| t.eq_ignore_ascii_case(candidate))
    };
    if !exists(base) {
        return base.to_string();
    }
    for n in 2..10_000 {
        let suffix = format!(" ({n})");
        let keep = max_chars.saturating_sub(suffix.chars().count());
        let truncated: String = base.chars().take(keep).collect();
        let candidate = format!("{}{suffix}", truncated.trim_end());
        if !exists(&candidate) {
            return candidate;
        }
    }
    base.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_names() {
        let taken = ["Default", "Default (2)", "Pro"];
        assert_eq!(unique_name("New", taken, 64), "New");
        assert_eq!(unique_name("Default", taken, 64), "Default (3)");
        assert_eq!(unique_name("pro", taken, 64), "pro (2)");
        let long = "x".repeat(64);
        let taken_long = [long.as_str()];
        let out = unique_name(&long, taken_long, 64);
        assert_eq!(out.chars().count(), 64);
        assert!(out.ends_with(" (2)"));
    }
}
