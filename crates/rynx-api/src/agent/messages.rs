//! Localized text for deterministic findings and agent alert emails.
//!
//! Rule findings are rendered in the panel owner's locale. Spanish and English
//! are complete; other locales fall back to English so the agent never sends
//! Spanish text to an English-speaking administrator by accident.

use serde_json::Value;

pub struct FindingText {
    pub title: String,
    pub summary: String,
    pub recommendation: String,
}

pub fn email_subject(locale: &str, count: usize) -> String {
    if locale == "es" {
        format!("RYNXCP: {count} alerta(s) del agente")
    } else {
        format!("RYNXCP: {count} agent alert(s)")
    }
}

pub fn email_labels(locale: &str) -> (&'static str, &'static str) {
    if locale == "es" {
        ("Recomendación", "Severidad")
    } else {
        ("Recommendation", "Severity")
    }
}

pub fn english(code: &str, params: &Value) -> Option<FindingText> {
    let (title, summary, recommendation) = match code {
        "auth-failures" => (
            "Elevated failed login attempts",
            "{failures} failed authentication attempts in the last 24 hours.",
            "Review Security, block suspicious IPs and check fail2ban.",
        ),
        "auth-blocked" => (
            "Authentication lockouts",
            "{blocked} login attempts were blocked in the last 24 hours.",
            "Check whether the lockouts come from brute force or legitimate users.",
        ),
        "service-failed" => (
            "Service down: {name}",
            "Unit {unit} is failed and may affect hosting.",
            "Check the service status and journal; restart it from Programs if appropriate.",
        ),
        "mail-filter-missing" => (
            "Incomplete mail filters",
            "Missing antivirus/antispam components: {missing}.",
            "Reinstall the mail stack with the installer or from Programs.",
        ),
        "mail-filter-inactive" => (
            "Mail filter stopped: {name}",
            "{name} is installed but its unit reports '{status}'.",
            "Restart the component from Antivirus and antispam or Programs.",
        ),
        "antivirus-detection" => (
            "Malware detected in {service}",
            "{message}",
            "Isolate the file, review the affected mailbox or account and run a full scan.",
        ),
        "antispam-volume" => (
            "High recent spam volume",
            "{spam} messages were tagged or rejected by SpamAssassin in the recent journal.",
            "Review SpamAssassin rules, mail volume and whether a spam campaign is in progress.",
        ),
        "security-updates" => (
            "Security updates pending",
            "{count} updates marked as security are pending.",
            "Apply the updates from Programs during a maintenance window.",
        ),
        "disk-usage" => (
            "High disk usage",
            "Disk usage is at {percent}% and may fill up.",
            "Review backups, logs and accounts with the highest usage before the disk fills.",
        ),
        "memory-usage" => (
            "High memory usage",
            "Memory usage is at {percent}%.",
            "Review processes, containers and application limits before the OOM killer acts.",
        ),
        "load-average" => (
            "High system load",
            "Load 1m per CPU: {per_cpu}.",
            "Identify CPU-heavy processes and review managed applications.",
        ),
        "certificate-expired" => (
            "Certificate expired: {domain}",
            "The certificate for {domain} has expired (expires {not_after}).",
            "Renew the certificate from Certificates or republish the site.",
        ),
        "certificate-expiring" => (
            "Certificate expiring: {domain}",
            "The certificate for {domain} is about to expire (expires {not_after}).",
            "Renew the certificate from Certificates or republish the site.",
        ),
        "firewall-blocks" => (
            "Many blocked IPs",
            "{blocked} IP addresses are currently blocked.",
            "Check for a distributed attack and review the fail2ban rules.",
        ),
        _ => return None,
    };
    Some(FindingText {
        title: fill(title, params),
        summary: fill(summary, params),
        recommendation: fill(recommendation, params),
    })
}

fn fill(template: &str, params: &Value) -> String {
    let mut output = template.to_string();
    if let Some(map) = params.as_object() {
        for (key, value) in map {
            let rendered = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            output = output.replace(&format!("{{{key}}}"), &rendered);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_placeholders() {
        let text = english("auth-failures", &json!({ "failures": 42, "unique_ips": 3 }))
            .expect("template");
        assert!(text.summary.contains("42"));
        assert!(text.title.contains("failed"));
    }

    #[test]
    fn unknown_codes_have_no_template() {
        assert!(english("unknown", &json!({})).is_none());
    }

    #[test]
    fn labels_and_subject_follow_locale() {
        assert!(email_subject("es", 2).contains("alerta"));
        assert!(email_subject("fr", 2).contains("alert"));
        assert_eq!(email_labels("es").0, "Recomendación");
        assert_eq!(email_labels("fr").0, "Recommendation");
    }
}
