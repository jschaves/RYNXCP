//! Sandbox executor: renders the exact same artifacts but performs no
//! privileged operation. Used for development and CI.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

use rynx_core::paths::{remove_managed, write_managed};
use rynx_core::DaemonConfig;
use rynx_proto::{
    AccountSpec, AppSpec, CertView, CertificateStatus, PackageUpdate, ProgramInfo, SystemInfo,
};

use super::{ExecError, ExecResult, Executor, UnitStatus};

pub struct SimExecutor {
    cfg: DaemonConfig,
}

impl SimExecutor {
    pub fn new(cfg: DaemonConfig) -> Self {
        Self { cfg }
    }

    fn users_dir(&self) -> PathBuf {
        self.cfg.state_dir.join("sim-users")
    }

    fn ensure_cert_dir(&self, domain: &str) -> ExecResult<PathBuf> {
        let dir = self.cfg.certs_root.join(domain);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}

fn write_text(path: &Path, content: &str) -> ExecResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

#[async_trait]
impl Executor for SimExecutor {
    fn mode(&self) -> &'static str {
        "simulation"
    }

    fn home(&self, username: &str) -> PathBuf {
        self.cfg.home_root.join(username)
    }

    fn apps_dir(&self, username: &str) -> PathBuf {
        self.home(username).join("apps")
    }

    fn working_dir(&self, username: &str, app: &str) -> PathBuf {
        self.apps_dir(username).join(app)
    }

    fn acme_root(&self) -> PathBuf {
        self.cfg.acme_root.clone()
    }

    async fn ensure_account(&self, spec: &AccountSpec, uid: u32) -> ExecResult<()> {
        let home = self.home(&spec.username);
        for dir in [
            home.clone(),
            home.join("public_html"),
            home.join("logs"),
            home.join("apps"),
            home.join(".php/sessions"),
            home.join("tmp"),
        ] {
            std::fs::create_dir_all(&dir)?;
        }
        std::fs::create_dir_all(self.users_dir())?;
        write_text(
            &self.users_dir().join(format!("uid-{uid}")),
            &format!("{}={uid}", spec.username),
        )?;
        write_text(
            &self.users_dir().join(format!("{}.json", spec.username)),
            &serde_json::to_string_pretty(spec).unwrap_or_default(),
        )?;
        Ok(())
    }

    async fn remove_account(&self, username: &str, purge: bool) -> ExecResult<()> {
        let home = self.home(username);
        if purge && home.exists() {
            std::fs::remove_dir_all(&home)?;
        }
        if let Ok(entries) = std::fs::read_dir(self.users_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.starts_with(&format!("{username}=")) {
                        let _ = std::fs::remove_file(&path);
                    }
                }
                if path.file_name().and_then(|n| n.to_str()) == Some(&format!("{username}.json")) {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        Ok(())
    }

    async fn ensure_docroot(&self, username: &str, uid: u32, path: &Path) -> ExecResult<()> {
        let home = self.home(username);
        if !path.starts_with(&home) {
            return Err(ExecError::Unsupported(
                "docroot escapes the account home".into(),
            ));
        }
        std::fs::create_dir_all(path)?;
        let _ = uid;
        Ok(())
    }

    async fn apply_site(&self, domain: &str, content: &str, enable: bool) -> ExecResult<()> {
        std::fs::create_dir_all(&self.cfg.nginx_sites_dir)?;
        let path = self.cfg.nginx_sites_dir.join(format!("{domain}.conf"));
        write_managed(&self.cfg.nginx_sites_dir, &path, content, 0o644, None)?;
        std::fs::create_dir_all(&self.cfg.nginx_enabled_dir)?;
        let link = self.cfg.nginx_enabled_dir.join(format!("{domain}.conf"));
        if enable {
            write_text(&link, "../sites-available/rynxcp")?;
        } else if link.exists() {
            let _ = std::fs::remove_file(&link);
        }
        Ok(())
    }

    async fn remove_site(&self, domain: &str) -> ExecResult<()> {
        let _ = remove_managed(
            &self.cfg.nginx_sites_dir,
            &self.cfg.nginx_sites_dir.join(format!("{domain}.conf")),
        );
        let link = self.cfg.nginx_enabled_dir.join(format!("{domain}.conf"));
        if link.exists() {
            let _ = std::fs::remove_file(link);
        }
        Ok(())
    }

    async fn apply_php_pool(
        &self,
        username: &str,
        version: &str,
        content: &str,
        enable: bool,
    ) -> ExecResult<()> {
        let dir = self
            .cfg
            .php_fpm_root
            .join(version)
            .join("fpm")
            .join("pool.d");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("rynx-{username}.conf"));
        if enable {
            write_managed(&self.cfg.php_fpm_root, &path, content, 0o644, None)?;
        } else {
            let _ = remove_managed(&self.cfg.php_fpm_root, &path);
        }
        std::fs::create_dir_all(&self.cfg.php_socket_dir)?;
        Ok(())
    }

    async fn remove_php_pool(&self, username: &str, version: &str) -> ExecResult<()> {
        let path = self
            .cfg
            .php_fpm_root
            .join(version)
            .join("fpm")
            .join("pool.d")
            .join(format!("rynx-{username}.conf"));
        let _ = remove_managed(&self.cfg.php_fpm_root, &path);
        Ok(())
    }

    async fn ensure_app_dir(&self, username: &str, uid: u32, dir: &Path) -> ExecResult<()> {
        let home = self.home(username);
        if !dir.starts_with(&home) {
            return Err(ExecError::Unsupported(
                "application directory escapes the account home".into(),
            ));
        }
        std::fs::create_dir_all(dir)?;
        let _ = uid;
        Ok(())
    }

    async fn apply_app(
        &self,
        spec: &AppSpec,
        uid: u32,
        unit: &str,
        unit_content: &str,
        env_filename: &str,
        env_content: &str,
        working_dir: &Path,
        enable: bool,
    ) -> ExecResult<()> {
        self.ensure_app_dir(&spec.username, uid, working_dir)
            .await?;
        std::fs::create_dir_all(&self.cfg.app_env_dir)?;
        let env_path = self.cfg.app_env_dir.join(env_filename);
        write_managed(&self.cfg.app_env_dir, &env_path, env_content, 0o640, None)?;
        std::fs::create_dir_all(&self.cfg.systemd_dir)?;
        let unit_path = self.cfg.systemd_dir.join(unit);
        write_managed(&self.cfg.systemd_dir, &unit_path, unit_content, 0o644, None)?;
        let enabled_dir = self.cfg.systemd_dir.join("enabled");
        if enable {
            write_text(&enabled_dir.join(unit), "enabled")?;
        } else if enabled_dir.join(unit).exists() {
            let _ = std::fs::remove_file(enabled_dir.join(unit));
        }
        Ok(())
    }

    async fn remove_app(
        &self,
        username: &str,
        unit: &str,
        env_filename: &str,
        working_dir: &Path,
        purge: bool,
    ) -> ExecResult<()> {
        let _ = remove_managed(&self.cfg.systemd_dir, &self.cfg.systemd_dir.join(unit));
        let _ = std::fs::remove_file(self.cfg.systemd_dir.join("enabled").join(unit));
        let _ = remove_managed(
            &self.cfg.app_env_dir,
            &self.cfg.app_env_dir.join(env_filename),
        );
        if purge && working_dir.starts_with(self.home(username)) {
            let _ = std::fs::remove_dir_all(working_dir);
        }
        Ok(())
    }

    async fn issue_cert(
        &self,
        domain: &str,
        domains: &[String],
        _email: &str,
        _webroot: &Path,
        _staging: bool,
    ) -> ExecResult<()> {
        let dir = self.ensure_cert_dir(domain)?;
        write_text(
            &dir.join("fullchain.pem"),
            "-----BEGIN CERTIFICATE-----\nSIMULATION\n-----END CERTIFICATE-----\n",
        )?;
        write_text(
            &dir.join("privkey.pem"),
            "-----BEGIN PRIVATE KEY-----\nSIMULATION\n-----END PRIVATE KEY-----\n",
        )?;
        write_text(&dir.join("domains.txt"), &domains.join("\n"))?;
        Ok(())
    }

    async fn renew_certs(&self, _staging: bool) -> ExecResult<()> {
        if let Ok(entries) = std::fs::read_dir(&self.cfg.certs_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let chain = path.join("fullchain.pem");
                    if chain.exists() {
                        write_text(
                            &chain,
                            "-----BEGIN CERTIFICATE-----\nSIMULATION RENEWED\n-----END CERTIFICATE-----\n",
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn remove_cert(&self, domain: &str) -> ExecResult<()> {
        let dir = self.cfg.certs_root.join(domain);
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    async fn cert_info(&self, domain: &str) -> ExecResult<Option<CertView>> {
        let dir = self.cfg.certs_root.join(domain);
        if !dir.join("fullchain.pem").exists() {
            return Ok(None);
        }
        let not_after = (OffsetDateTime::now_utc() + Duration::days(90))
            .format(&Rfc3339)
            .unwrap_or_else(|_| "unknown".into());
        Ok(Some(CertView {
            domain: domain.to_string(),
            issuer: "RYNXCP Simulation CA".into(),
            not_after,
            status: CertificateStatus::Active,
            path: dir.display().to_string(),
        }))
    }

    async fn status_unit(&self, unit: &str) -> ExecResult<UnitStatus> {
        let active = !unit.contains("stopped");
        Ok(UnitStatus {
            active,
            state: if active {
                "active (running)".into()
            } else {
                "inactive (dead)".into()
            },
            pid: if active { 1000 } else { 0 },
            memory_kb: if active { 65_536 } else { 0 },
            cpu_percent: if active { 1.2 } else { 0.0 },
            uptime_secs: if active { 3600 } else { 0 },
        })
    }

    async fn restart_unit(&self, _unit: &str) -> ExecResult<()> {
        Ok(())
    }

    async fn reload_unit(&self, _unit: &str) -> ExecResult<()> {
        Ok(())
    }

    async fn set_unit_running(&self, _unit: &str, _running: bool) -> ExecResult<()> {
        Ok(())
    }

    async fn journal_unit(&self, unit: &str, lines: u32) -> ExecResult<String> {
        let mut output = String::new();
        for index in (1..=lines.min(20)).rev() {
            output.push_str(&format!(
                "2026-01-01T00:00:{:02}Z {}[sim]: health check ok (line {})\n",
                index % 60,
                unit,
                index
            ));
        }
        Ok(output)
    }

    async fn disk_usage(&self) -> ExecResult<String> {
        Ok("Filesystem      Size  Used Avail Use% Mounted on\n/dev/sim1        80G   20G   56G  27% /\n".into())
    }

    async fn nginx_test_text(&self) -> ExecResult<String> {
        Ok("nginx: the configuration file /etc/nginx/nginx.conf syntax is ok (simulation)\nnginx: configuration file test is successful\n".into())
    }

    async fn create_backup(&self, paths: &[String], dest: &Path) -> ExecResult<u64> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let manifest = format!(
            "{{\"simulation\":true,\"paths\":{}}}",
            serde_json::to_string(paths).unwrap_or_else(|_| "[]".into())
        );
        std::fs::write(dest, manifest)?;
        Ok(std::fs::metadata(dest)?.len())
    }

    async fn restore_backup(&self, archive: &Path, username: &str, _uid: u32) -> ExecResult<()> {
        let home = self.home(username);
        std::fs::create_dir_all(&home)?;
        let marker = format!("restored from {}", archive.display());
        std::fs::write(home.join(".rynx-restored"), marker)?;
        Ok(())
    }

    async fn delete_backup_file(&self, path: &Path) -> ExecResult<()> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    async fn remove_backup_dir(&self, username: &str) -> ExecResult<()> {
        let dir = self.cfg.backup_dir.join(username);
        if dir.exists() {
            let _ = std::fs::remove_dir_all(dir);
        }
        Ok(())
    }

    async fn apply_zone(&self, domain: &str, content: &str) -> ExecResult<()> {
        std::fs::create_dir_all(&self.cfg.bind_zones_dir)?;
        write_managed(
            &self.cfg.bind_zones_dir,
            &self.cfg.bind_zones_dir.join(format!("{domain}.db")),
            content,
            0o644,
            None,
        )?;
        Ok(())
    }

    async fn remove_zone(&self, domain: &str) -> ExecResult<()> {
        let _ = remove_managed(
            &self.cfg.bind_zones_dir,
            &self.cfg.bind_zones_dir.join(format!("{domain}.db")),
        );
        Ok(())
    }

    async fn write_dns_conf(&self, content: &str) -> ExecResult<()> {
        if let Some(parent) = self.cfg.bind_conf_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.cfg.bind_conf_file, content)?;
        Ok(())
    }

    async fn generate_dkim_key(&self, domain: &str, selector: &str) -> ExecResult<String> {
        let dir = self.cfg.dkim_key_dir.join(domain);
        std::fs::create_dir_all(&dir)?;
        write_text(
            &dir.join(format!("{selector}.private")),
            "SIMULATED-PRIVATE-KEY\n",
        )?;
        let public =
            "-----BEGIN PUBLIC KEY-----\nU0lNVUxBVEVELVBVQkxJQy1LRVk=\n-----END PUBLIC KEY-----\n";
        write_text(&dir.join(format!("{selector}.public")), public)?;
        Ok(public.to_string())
    }

    async fn write_dkim_tables(&self, keytable: &str, signingtable: &str) -> ExecResult<()> {
        if let Some(path) = &self.cfg.opendkim_keytable {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, keytable)?;
        }
        if let Some(path) = &self.cfg.opendkim_signingtable {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, signingtable)?;
        }
        Ok(())
    }

    async fn create_database(&self, database: &str, user: &str, password: &str) -> ExecResult<()> {
        let dir = self.cfg.state_dir.join("sim-databases");
        std::fs::create_dir_all(&dir)?;
        write_text(
            &dir.join(format!("{database}.json")),
            &serde_json::json!({
                "database": database,
                "user": user,
                "password": password,
                "engine": "mariadb",
            })
            .to_string(),
        )?;
        Ok(())
    }

    async fn drop_database(&self, database: &str, _user: &str) -> ExecResult<()> {
        let path = self
            .cfg
            .state_dir
            .join("sim-databases")
            .join(format!("{database}.json"));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    async fn apply_ftp_user(
        &self,
        login: &str,
        uid: u32,
        _gid: u32,
        home: &Path,
        password: &str,
    ) -> ExecResult<()> {
        let dir = self.cfg.state_dir.join("sim-ftp");
        std::fs::create_dir_all(&dir)?;
        write_text(
            &dir.join(format!("{login}.json")),
            &serde_json::json!({
                "login": login,
                "uid": uid,
                "home": home.display().to_string(),
                "password": password,
            })
            .to_string(),
        )?;
        Ok(())
    }

    async fn remove_ftp_user(&self, login: &str) -> ExecResult<()> {
        let path = self
            .cfg
            .state_dir
            .join("sim-ftp")
            .join(format!("{login}.json"));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    async fn resolve_user_ids(&self, _username: &str) -> ExecResult<Option<(u32, u32)>> {
        Ok(Some((1000, 1000)))
    }

    async fn apply_firewall(&self, ruleset: &str) -> ExecResult<()> {
        let dir = self.cfg.state_dir.join("sim-firewall");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("ruleset.nft"), ruleset)?;
        Ok(())
    }

    async fn installed_programs(&self) -> ExecResult<Vec<ProgramInfo>> {
        let rows = [
            ("nginx", "1.26.3", "web", "installed", "reverse proxy"),
            ("php-fpm 8.3", "8.3.14", "runtime", "installed", "FPM pool"),
            ("node", "22.11.0", "runtime", "installed", "systemd"),
            (
                "python3",
                "3.12.7",
                "runtime",
                "installed",
                "gunicorn/uvicorn",
            ),
            ("ruby", "3.3.6", "runtime", "installed", "puma"),
            ("java", "21.0.5", "runtime", "installed", "JAR"),
            ("dotnet", "8.0.11", "runtime", "installed", "ASP.NET"),
            ("go", "1.23.4", "runtime", "installed", "binaries"),
            (
                "mariadb",
                "11.4.4",
                "database",
                "installed",
                "per-domain databases",
            ),
            ("bind9", "9.20.3", "dns", "installed", "authoritative zones"),
            ("certbot", "3.0.1", "security", "installed", "Let's Encrypt"),
            (
                "pure-ftpd",
                "1.0.50",
                "web",
                "installed",
                "FTPS virtual users",
            ),
            ("opendkim", "2.11.0", "mail", "installed", "DKIM signing"),
            (
                "fail2ban",
                "1.1.0",
                "security",
                "installed",
                "brute-force jail",
            ),
            ("rynxcp", "0.1.0", "panel", "installed", "control plane"),
        ];
        Ok(rows
            .into_iter()
            .map(|(name, version, kind, status, detail)| ProgramInfo {
                name: name.into(),
                version: version.into(),
                kind: kind.into(),
                status: status.into(),
                detail: detail.into(),
            })
            .collect())
    }

    async fn available_updates(&self) -> ExecResult<Vec<PackageUpdate>> {
        Ok(vec![
            PackageUpdate {
                package: "nginx".into(),
                current_version: "1.26.3-1".into(),
                candidate_version: "1.26.4-1".into(),
                security: true,
            },
            PackageUpdate {
                package: "php8.3-fpm".into(),
                current_version: "8.3.14-1".into(),
                candidate_version: "8.3.15-1".into(),
                security: false,
            },
        ])
    }

    async fn apply_updates(&self) -> ExecResult<String> {
        Ok("simulation: apt-get update && apt-get upgrade completed\n2 packages upgraded\n".into())
    }

    async fn system_info(&self) -> ExecResult<SystemInfo> {
        Ok(SystemInfo {
            hostname: std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_else(|_| "rynx-sim".into()),
            os: format!("{} (simulation)", std::env::consts::OS),
            kernel: "simulation".into(),
            uptime_secs: 0,
            load: [0, 0, 0],
            cpu_count: std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(1),
            mem_total_kb: 8 * 1024 * 1024,
            mem_available_kb: 6 * 1024 * 1024,
            disk_total_kb: 80 * 1024 * 1024,
            disk_available_kb: 60 * 1024 * 1024,
            daemon_version: rynx_proto::DAEMON_VERSION.into(),
            mode: "simulation".into(),
        })
    }

    async fn uid_in_use(&self, uid: u32) -> ExecResult<bool> {
        Ok(self.users_dir().join(format!("uid-{uid}")).exists())
    }
}
