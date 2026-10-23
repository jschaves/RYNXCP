//! Configurable user rules: password strength and resource limits.
//!
//! Password rules are enforced by the API when panel users are created or
//! updated. Resource limits are enforced by the daemon before it accepts a new
//! website or application, independently of what the API allows.

use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy)]
pub struct PasswordPolicy {
    pub min_len: usize,
    pub min_classes: u8,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_len: 12,
            min_classes: 3,
        }
    }
}

const COMMON_PASSWORDS: &[&str] = &[
    "password",
    "password123",
    "password1234",
    "123456789012",
    "1234567890123",
    "qwertyuiop",
    "administrator",
    "letmein123",
    "welcome123",
    "changeme123",
    "iloveyou123",
    "adminadmin",
    "rootroot",
];

impl PasswordPolicy {
    pub fn new(min_len: usize, min_classes: u8) -> Self {
        Self {
            min_len: min_len.max(8),
            min_classes: min_classes.clamp(1, 4),
        }
    }

    /// Reject weak passwords, including passwords derived from the username or
    /// email. The error message is safe to return to an authenticated admin.
    pub fn validate(&self, password: &str, username: &str, email: &str) -> CoreResult<()> {
        if password.len() < self.min_len {
            return Err(CoreError::Validation(format!(
                "password must be at least {} characters",
                self.min_len
            )));
        }
        if password.len() > 256 {
            return Err(CoreError::Validation(
                "password must be at most 256 characters".into(),
            ));
        }
        if password.trim().is_empty() {
            return Err(CoreError::Validation("password cannot be blank".into()));
        }

        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_symbol = password.chars().any(|c| !c.is_alphanumeric());
        let classes = [has_lower, has_upper, has_digit, has_symbol]
            .into_iter()
            .filter(|present| *present)
            .count() as u8;
        if classes < self.min_classes {
            return Err(CoreError::Validation(format!(
                "password must combine at least {} character classes (lowercase, uppercase, digit, symbol)",
                self.min_classes
            )));
        }

        let lowered = password.to_lowercase();
        let username_lower = username.to_lowercase();
        if username_lower.len() >= 3 && lowered.contains(&username_lower) {
            return Err(CoreError::Validation(
                "password must not contain the username".into(),
            ));
        }
        let email_local = email.split('@').next().unwrap_or_default().to_lowercase();
        if email_local.len() >= 3 && lowered.contains(&email_local) {
            return Err(CoreError::Validation(
                "password must not contain the email mailbox".into(),
            ));
        }
        if COMMON_PASSWORDS.contains(&lowered.as_str()) {
            return Err(CoreError::Validation("password is too common".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_weak_passwords() {
        let policy = PasswordPolicy::default();
        assert!(policy
            .validate("short", "admin", "admin@example.com")
            .is_err());
        assert!(policy
            .validate("alllowercasepassword", "admin", "admin@example.com")
            .is_err());
        assert!(policy
            .validate("Password1234", "admin", "admin@example.com")
            .is_err());
        assert!(policy
            .validate("Sup3r-Secret-Pass", "admin", "admin@example.com")
            .is_ok());
    }

    #[test]
    fn rejects_identity_based_passwords() {
        let policy = PasswordPolicy::default();
        assert!(policy
            .validate("Admin-2026-Secure", "admin", "admin@example.com")
            .is_err());
        assert!(policy
            .validate("Alice-2026-Secure", "bob", "alice@example.com")
            .is_err());
    }

    #[test]
    fn policy_is_configurable() {
        let policy = PasswordPolicy::new(20, 4);
        assert!(policy.validate("Ab1!short", "admin", "x@y.com").is_err());
        assert!(policy
            .validate("Abcdefghijk1!mnopqrs", "admin", "x@y.com")
            .is_ok());
    }
}
