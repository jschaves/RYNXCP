//! Mapping from installed programs to their systemd service units.

/// Return the systemd unit that controls a listed program, when it has one.
///
/// Runtimes such as Node, Python or Java are binaries, not services, so they
/// intentionally return `None`.
pub fn service_unit_for_program(name: &str) -> Option<String> {
    match name {
        "nginx" | "mariadb" | "pure-ftpd" | "opendkim" | "fail2ban" => Some(name.to_string()),
        "bind9" => Some("named".into()),
        _ => php_unit(name),
    }
}

fn php_unit(name: &str) -> Option<String> {
    let version = name
        .strip_prefix("php-fpm ")
        .or_else(|| name.strip_prefix("php"))?;
    if version.is_empty() || !version.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return None;
    }
    Some(format!("php{version}-fpm"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_only_service_backed_programs() {
        assert_eq!(service_unit_for_program("nginx").as_deref(), Some("nginx"));
        assert_eq!(service_unit_for_program("bind9").as_deref(), Some("named"));
        assert_eq!(
            service_unit_for_program("php8.3").as_deref(),
            Some("php8.3-fpm")
        );
        assert_eq!(
            service_unit_for_program("php-fpm 8.3").as_deref(),
            Some("php8.3-fpm")
        );
        assert_eq!(service_unit_for_program("node"), None);
        assert_eq!(service_unit_for_program("certbot"), None);
        assert_eq!(service_unit_for_program("php"), None);
    }
}
