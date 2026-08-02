//! Automatic redaction of sensitive field values.
//!
//! Fields whose key looks sensitive (for example `password`, `api_key`, or
//! `authorization`) are masked before they are written, in **both** pretty and
//! JSON output. This makes it much harder to accidentally leak secrets into
//! logs. Detection is heuristic and based on the key name — see
//! [`is_sensitive_key`].

use std::collections::HashMap;

use crate::json::JsonValue;

/// Case-insensitive substrings that mark a field key as sensitive.
const SENSITIVE_SUBSTRINGS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "authorization",
    "api_key",
    "apikey",
    "api-key",
    "private_key",
    "credential",
    "credit_card",
    "card_number",
    "cvv",
    "ssn",
];

/// Returns `true` if `key` contains any known sensitive substring
/// (case-insensitive). Note that `token` also matches `access_token`,
/// `refresh_token`, and similar.
pub fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_SUBSTRINGS
        .iter()
        .any(|needle| lower.contains(needle))
}

/// Masks a value, revealing as little as possible while staying recognizable.
///
/// The amount revealed scales with length (counted in Unicode scalar values, so
/// this is safe for non-ASCII input):
///
/// - empty → `****`
/// - 1–2 chars → all asterisks
/// - 3–6 chars → first character, then asterisks
/// - 7+ chars → first and last character, asterisks between
///
/// ```
/// use loxide::redact_value;
///
/// assert_eq!(redact_value(""), "****");
/// assert_eq!(redact_value("ab"), "**");
/// assert_eq!(redact_value("abcd"), "a***");
/// assert_eq!(redact_value("hunter2"), "h*****2");
/// ```
pub fn redact_value(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    match chars.len() {
        0 => "****".to_string(),
        n @ 1..=2 => "*".repeat(n),
        n @ 3..=6 => format!("{}{}", chars[0], "*".repeat(n - 1)),
        n => format!("{}{}{}", chars[0], "*".repeat(n - 2), chars[n - 1]),
    }
}

/// Redacts a single field value if its key is sensitive, otherwise returns it
/// unchanged. Non-string sensitive values are rendered to their textual form
/// first so that, e.g., a numeric secret is still masked.
pub(crate) fn redact_field(key: &str, value: JsonValue) -> JsonValue {
    if !is_sensitive_key(key) {
        return value;
    }
    let plain = match &value {
        JsonValue::String(s) => s.clone(),
        other => other.to_string(),
    };
    JsonValue::String(redact_value(&plain))
}

/// Returns a copy of `map` with sensitive values redacted. Convenient for
/// scrubbing a map of strings (such as HTTP headers or form data) before it is
/// attached to a log record as a single field.
///
/// ```
/// use std::collections::HashMap;
/// use loxide::redact_map;
///
/// let mut headers = HashMap::new();
/// headers.insert("authorization".to_string(), "Bearer abcdef".to_string());
/// headers.insert("accept".to_string(), "application/json".to_string());
///
/// let safe = redact_map(&headers);
/// assert_eq!(safe["accept"], "application/json");
/// assert_ne!(safe["authorization"], "Bearer abcdef");
/// ```
pub fn redact_map(map: &HashMap<String, String>) -> HashMap<String, String> {
    map.iter()
        .map(|(key, value)| {
            let value = if is_sensitive_key(key) {
                redact_value(value)
            } else {
                value.clone()
            };
            (key.clone(), value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;

    #[test]
    fn detects_sensitive_keys() {
        for key in [
            "password",
            "Password",
            "user_password",
            "secret",
            "access_token",
            "refresh_token",
            "Authorization",
            "api_key",
            "X-Api-Key",
            "credit_card",
            "ssn",
        ] {
            assert!(is_sensitive_key(key), "expected {key} to be sensitive");
        }
    }

    #[test]
    fn ignores_ordinary_keys() {
        for key in ["username", "email", "host", "port", ""] {
            assert!(!is_sensitive_key(key), "expected {key} to be safe");
        }
    }

    #[test]
    fn masks_by_length() {
        assert_eq!(redact_value(""), "****");
        assert_eq!(redact_value("a"), "*");
        assert_eq!(redact_value("ab"), "**");
        assert_eq!(redact_value("abc"), "a**");
        assert_eq!(redact_value("abcdef"), "a*****");
        assert_eq!(redact_value("abcdefg"), "a*****g");
        assert_eq!(redact_value("abcdefghij"), "a********j");
    }

    #[test]
    fn masking_is_utf8_safe() {
        // Multi-byte characters must not cause a panic or byte-boundary slice.
        // "héllo" is 5 chars → first char plus asterisks.
        assert_eq!(redact_value("héllo"), "h****");
        // "naïveté" is 7 chars → first and last char, asterisks between.
        assert_eq!(redact_value("naïveté"), "n*****é");
    }

    #[test]
    fn redact_field_only_touches_sensitive_keys() {
        assert_eq!(redact_field("username", json!("john")), json!("john"));
        let masked = redact_field("password", json!("secret123"));
        assert_eq!(masked, json!("s*******3"));
    }

    #[test]
    fn redact_map_masks_selectively() {
        let mut map = HashMap::new();
        map.insert("username".to_string(), "john".to_string());
        map.insert("password".to_string(), "secret123".to_string());
        let result = redact_map(&map);
        assert_eq!(result["username"], "john");
        assert_eq!(result["password"], "s*******3");
    }
}
