//! Redaction applied before any telemetry leaves the process.
//!
//! The local provider still benefits from this: it keeps credentials and
//! private keys out of prompts, logs and error traces.

use std::sync::LazyLock;

use regex::Regex;

static PRIVATE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----")
        .expect("valid private key regex")
});
static LABELED_SECRET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(password|passwd|token|secret|api[_-]?key|authorization)\b\s*[:=]\s*[^\s,;]+",
    )
    .expect("valid labeled secret regex")
});
static BEARER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)bearer\s+[A-Za-z0-9._~+/\-]+=*").expect("valid bearer regex")
});
static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}").expect("valid email regex")
});
static LONG_HEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[0-9a-fA-F]{32,}\b").expect("valid hex regex"));
static LONG_BASE64: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Za-z0-9+/]{40,}={0,2}\b").expect("valid base64 regex"));

/// Redacts common secret shapes and optionally truncates the result.
pub fn redact(text: &str, max_chars: usize) -> String {
    let mut value = PRIVATE_KEY.replace_all(text, "[private-key]").into_owned();
    value = BEARER.replace_all(&value, "Bearer [redacted]").into_owned();
    value = LABELED_SECRET
        .replace_all(&value, "$1=[redacted]")
        .into_owned();
    value = EMAIL.replace_all(&value, "[email]").into_owned();
    value = LONG_HEX.replace_all(&value, "[hash]").into_owned();
    value = LONG_BASE64.replace_all(&value, "[secret]").into_owned();
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_labeled_secrets() {
        let result = redact("password=SuperSecret token: abc123", 500);
        assert!(!result.contains("SuperSecret"));
        assert!(!result.contains("abc123"));
        assert!(result.contains("[redacted]"));
    }

    #[test]
    fn redacts_emails_and_bearer_tokens() {
        let result = redact(
            "mail user@example.com Authorization: Bearer abc.def.ghi",
            500,
        );
        assert!(!result.contains("user@example.com"));
        assert!(!result.contains("abc.def.ghi"));
        assert!(result.contains("[email]"));
        assert!(result.contains("[redacted]"));
    }

    #[test]
    fn redacts_private_keys_and_hashes() {
        let key = "-----BEGIN PRIVATE KEY-----\nMIIE\n-----END PRIVATE KEY-----";
        let hash = "0123456789abcdef0123456789abcdef";
        let result = redact(&format!("{key} {hash}"), 500);
        assert!(!result.contains("MIIE"));
        assert!(!result.contains(hash));
        assert!(result.contains("[private-key]"));
        assert!(result.contains("[hash]"));
    }

    #[test]
    fn truncates_long_text() {
        let result = redact(&"texto con espacios ".repeat(20), 10);
        assert_eq!(result.chars().count(), 10);
    }
}
