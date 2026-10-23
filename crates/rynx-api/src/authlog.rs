//! Append-only authentication log, formatted for fail2ban.
//!
//! Lines never contain secrets. User-controlled fields are collapsed to a safe
//! character set to prevent log injection.

use std::io::Write;
use std::path::Path;

use rynx_core::crypto::now_rfc3339;

fn clean(value: &str, max: usize) -> String {
    let mut out: String = value
        .chars()
        .take(max)
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._@:-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        out.push('-');
    }
    out
}

pub fn log(
    path: &Path,
    event: &str,
    username: &str,
    ip: &str,
    user_agent: &str,
    actor: Option<&str>,
) {
    let actor = actor
        .map(|value| clean(value, 48))
        .unwrap_or_else(|| "-".into());
    let line = format!(
        "{} event={} user={} ip={} actor={} ua={}\n",
        now_rfc3339(),
        clean(event, 32),
        clean(username, 64),
        clean(ip, 64),
        actor,
        clean(user_agent, 96)
    );
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut file) => {
            let _ = file.write_all(line.as_bytes());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o640));
            }
        }
        Err(err) => {
            tracing::warn!(path = %path.display(), error = %err, "cannot write auth log");
        }
    }
}
