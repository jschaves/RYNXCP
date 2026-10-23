use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{CoreError, CoreResult};

/// Cryptographically secure random bytes, hex encoded.
pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    let mut rng = rand::rngs::OsRng;
    rng.fill_bytes(&mut buf);
    hex::encode(buf)
}

/// Generate a strong password containing at least four character classes.
pub fn random_password(len: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_";
    let len = len.clamp(12, 128);
    let mut rng = rand::rngs::OsRng;
    let mut chars: Vec<char> = (0..len)
        .map(|_| {
            let index = (rng.next_u32() as usize) % ALPHABET.len();
            ALPHABET[index] as char
        })
        .collect();
    if chars.len() >= 4 {
        chars[0] = 'A';
        chars[1] = 'a';
        chars[2] = '2';
        chars[3] = '!';
    }
    chars.into_iter().collect()
}

pub fn sha256_hex(data: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(data.as_ref()))
}

/// Constant-time comparison for secrets.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

pub fn now_unix() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

/// Generate a strong token and persist it with restrictive permissions.
pub fn load_or_create_token(path: &std::path::Path) -> CoreResult<String> {
    if let Ok(existing) = std::fs::read_to_string(path) {
        let trimmed = existing.trim();
        if trimmed.len() >= 32 {
            return Ok(trimmed.to_string());
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let token = random_hex(32);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &token)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(token)
}

pub fn read_token(path: &std::path::Path) -> CoreResult<String> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        CoreError::Config(format!("cannot read daemon token {}: {e}", path.display()))
    })?;
    let token = raw.trim().to_string();
    if token.len() < 32 {
        return Err(CoreError::Config("daemon token is too short".into()));
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_passwords_are_strong() {
        let password = random_password(24);
        assert_eq!(password.len(), 24);
        assert!(password.chars().any(|c| c.is_uppercase()));
        assert!(password.chars().any(|c| c.is_lowercase()));
        assert!(password.chars().any(|c| c.is_ascii_digit()));
        assert!(password.chars().any(|c| !c.is_alphanumeric()));
        assert_ne!(password, random_password(24));
    }

    #[test]
    fn tokens_are_unique_and_long() {
        let a = random_hex(32);
        let b = random_hex(32);
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }

    #[test]
    fn constant_time_compare() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
        assert!(!constant_time_eq("", ""));
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
