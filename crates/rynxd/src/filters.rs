//! Recent antivirus and antispam activity collected from the system journal.
//!
//! The panel only needs read access to the mail filter units. The executor keeps
//! the privileged work, so this module shells out to `journalctl` with an
//! explicit argv and falls back to an empty list when the journal is missing.

use serde_json::Value;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

use rynx_proto::FilterEvent;

const UNITS: [&str; 6] = [
    "clamav-daemon",
    "clamav-milter",
    "spamd",
    "spamassassin",
    "spamass-milter",
    "rynxcp-antivirus",
];

/// Collects the newest journal entries for the mail filters, newest first.
pub async fn recent_activity(limit: u32) -> Vec<FilterEvent> {
    let limit = limit.clamp(1, 500).to_string();
    let mut command = tokio::process::Command::new("journalctl");
    command.args(["-o", "json", "--no-pager", "-n", limit.as_str()]);
    for unit in UNITS {
        command.args(["-u", unit]);
    }
    command.env_clear();
    command.env(
        "PATH",
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    );
    let Ok(output) = command.output().await else {
        return Vec::new();
    };
    let mut events: Vec<FilterEvent> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_journal_line)
        .collect();
    events.sort_by(|left, right| right.at.cmp(&left.at));
    events
}

/// Deterministic sample used by simulation mode.
pub fn simulated_activity() -> Vec<FilterEvent> {
    let now = OffsetDateTime::now_utc();
    let entries = [
        (
            1,
            "antivirus",
            "rynxcp-antivirus",
            "info",
            "Scheduled scan completed: /home 12840 files, 0 infected",
        ),
        (
            2,
            "antispam",
            "spamass-milter",
            "warn",
            "Rejected message from 198.51.100.23 with score 16.4 (SPAM)",
        ),
        (
            3,
            "antivirus",
            "clamav-milter",
            "alert",
            "Blocked infected message: Eicar-Test-Signature FOUND",
        ),
        (
            5,
            "antispam",
            "spamd",
            "info",
            "Identified spam (score 7.9) for user ventas@example.com",
        ),
        (
            8,
            "antivirus",
            "clamav-daemon",
            "info",
            "Signature database updated to version 27345",
        ),
        (
            20,
            "antispam",
            "spamass-milter",
            "info",
            "Added X-Spam-Score header to outgoing message",
        ),
        (
            26,
            "antivirus",
            "rynxcp-antivirus",
            "info",
            "Scheduled scan completed: /var/vmail 2310 files, 0 infected",
        ),
    ];
    entries
        .into_iter()
        .map(|(hours, kind, service, severity, message)| FilterEvent {
            at: format_stamp(now - Duration::hours(hours)),
            kind: kind.to_string(),
            service: service.to_string(),
            severity: severity.to_string(),
            message: message.to_string(),
        })
        .collect()
}

fn parse_journal_line(line: &str) -> Option<FilterEvent> {
    let value: Value = serde_json::from_str(line).ok()?;
    let unit = value.get("_SYSTEMD_UNIT").and_then(Value::as_str)?;
    let kind = classify_unit(unit)?;
    let message = journal_message(&value)?;
    let message = message.trim();
    if message.is_empty() {
        return None;
    }
    Some(FilterEvent {
        at: journal_timestamp(&value).unwrap_or_else(|| "—".to_string()),
        kind: kind.to_string(),
        service: unit.to_string(),
        severity: severity_for(&value, message).to_string(),
        message: message.to_string(),
    })
}

fn classify_unit(unit: &str) -> Option<&'static str> {
    match unit {
        "clamav-daemon" | "clamav-milter" | "rynxcp-antivirus" => Some("antivirus"),
        "spamd" | "spamassassin" | "spamass-milter" => Some("antispam"),
        _ => None,
    }
}

fn journal_message(value: &Value) -> Option<String> {
    match value.get("MESSAGE")? {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let bytes: Vec<u8> = items
                .iter()
                .filter_map(|item| item.as_u64())
                .map(|byte| byte as u8)
                .collect();
            if bytes.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(&bytes).into_owned())
            }
        }
        _ => None,
    }
}

fn journal_timestamp(value: &Value) -> Option<String> {
    let micros = value
        .get("__REALTIME_TIMESTAMP")?
        .as_str()?
        .parse::<i128>()
        .ok()?;
    let at = OffsetDateTime::from_unix_timestamp_nanos(micros.checked_mul(1000)?).ok()?;
    Some(format_stamp(at))
}

fn severity_for(value: &Value, message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("found") || lower.contains("infected") || lower.contains("virus") {
        return "alert";
    }
    if lower.contains("reject") || lower.contains("blocked") || lower.contains("spam") {
        return "warn";
    }
    match value.get("PRIORITY").and_then(Value::as_str) {
        Some("0" | "1" | "2" | "3") => "error",
        Some("4") => "warn",
        _ => "info",
    }
}

fn format_stamp(at: OffsetDateTime) -> String {
    at.format(&Rfc3339)
        .unwrap_or_else(|_| at.unix_timestamp().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_units() {
        assert_eq!(classify_unit("clamav-daemon"), Some("antivirus"));
        assert_eq!(classify_unit("rynxcp-antivirus"), Some("antivirus"));
        assert_eq!(classify_unit("spamd"), Some("antispam"));
        assert_eq!(classify_unit("spamass-milter"), Some("antispam"));
        assert_eq!(classify_unit("nginx"), None);
    }

    #[test]
    fn parses_journal_json_entries() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1761825600000000","PRIORITY":"4","_SYSTEMD_UNIT":"clamav-milter","MESSAGE":"Blocked infected message: Eicar-Test-Signature FOUND"}"#;
        let event = parse_journal_line(line).expect("event");
        assert_eq!(event.kind, "antivirus");
        assert_eq!(event.service, "clamav-milter");
        assert_eq!(event.severity, "alert");
        assert!(event.at.starts_with("2025-"));
    }

    #[test]
    fn ignores_unrelated_units() {
        let line = r#"{"_SYSTEMD_UNIT":"nginx","MESSAGE":"reload","__REALTIME_TIMESTAMP":"1761825600000000"}"#;
        assert!(parse_journal_line(line).is_none());
    }

    #[test]
    fn simulation_has_both_kinds_and_is_ordered() {
        let events = simulated_activity();
        assert!(events.iter().any(|event| event.kind == "antivirus"));
        assert!(events.iter().any(|event| event.kind == "antispam"));
        assert!(events.windows(2).all(|pair| pair[0].at >= pair[1].at));
    }
}
