//! Linux executor. Every external command is invoked with an explicit argv,
//! a scrubbed environment and no shell.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use nix::unistd::{chown, Gid, Uid, User};
use time::{format_description::well_known::Rfc3339, macros::format_description, OffsetDateTime};
use tokio::io::AsyncWriteExt;
use tokio::process::Stdio;

use rynx_core::paths::{remove_managed, write_managed};
use rynx_core::DaemonConfig;
use rynx_proto::{
    AccountSpec, AppSpec, CertView, CertificateStatus, PackageUpdate, ProgramInfo, SystemInfo,
};

use super::{ExecError, ExecResult, Executor, UnitStatus};

const SAFE_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

pub struct RealExecutor {
    cfg: DaemonConfig,
}

impl RealExecutor {
    pub fn new(cfg: DaemonConfig) -> Self {
        Self { cfg }
    }

    fn gid_for(&self, username: &str) -> ExecResult<u32> {
        match User::from_name(username) {
            Ok(Some(user)) => Ok(user.gid.as_raw()),
            Ok(None) => Err(ExecError::MissingAccount(username.to_string())),
            Err(e) => Err(ExecError::Io(e.to_string())),
        }
    }

    fn chown_path(&self, path: &Path, uid: u32, gid: u32) -> ExecResult<()> {
        chown(path, Some(Uid::from_raw(uid)), Some(Gid::from_raw(gid)))
            .map_err(|e| ExecError::Io(format!("chown {}: {e}", path.display())))
    }

    fn make_dir(&self, path: &Path, mode: u32, uid: u32, gid: u32) -> ExecResult<()> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(path)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
        self.chown_path(path, uid, gid)?;
        Ok(())
    }

    async fn nginx_test(&self) -> ExecResult<()> {
        let args: Vec<String> = self
            .cfg
            .nginx_test_args
            .split_whitespace()
            .map(str::to_string)
            .collect();
        run_owned("nginx", &args).await.map(|_| ())
    }

    async fn reload_service(&self, service: &str) -> ExecResult<()> {
        run("systemctl", &["reload-or-restart", service])
            .await
            .map(|_| ())
    }

    fn php_binary(&self, version: &str) -> String {
        format!("php-fpm{version}")
    }
}

async fn run(program: &str, args: &[&str]) -> ExecResult<String> {
    let output = tokio::process::Command::new(program)
        .args(args)
        .env_clear()
        .env("PATH", SAFE_PATH)
        .env("LC_ALL", "C")
        .output()
        .await
        .map_err(|e| ExecError::Command {
            program: program.to_string(),
            args: args.join(" "),
            status: -1,
            stderr: e.to_string(),
        })?;
    if !output.status.success() {
        return Err(ExecError::Command {
            program: program.to_string(),
            args: args.join(" "),
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn run_owned(program: &str, args: &[String]) -> ExecResult<String> {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run(program, &refs).await
}

/// Like `run` but returns stdout and stderr on success (nginx prints its
/// configuration check to stderr even when the test passes).
async fn run_combined(program: &str, args: &[String]) -> ExecResult<String> {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = tokio::process::Command::new(program)
        .args(&refs)
        .env_clear()
        .env("PATH", SAFE_PATH)
        .env("LC_ALL", "C")
        .output()
        .await
        .map_err(|e| ExecError::Command {
            program: program.to_string(),
            args: refs.join(" "),
            status: -1,
            stderr: e.to_string(),
        })?;
    let mut combined = String::from_utf8_lossy(&output.stdout).to_string();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(ExecError::Command {
            program: program.to_string(),
            args: refs.join(" "),
            status: output.status.code().unwrap_or(-1),
            stderr: combined,
        });
    }
    Ok(combined)
}

#[async_trait]
impl Executor for RealExecutor {
    fn mode(&self) -> &'static str {
        "linux"
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
        let existing = User::from_name(&spec.username).map_err(|e| ExecError::Io(e.to_string()))?;
        if existing.is_none() {
            let uid_s = uid.to_string();
            let home_s = home.display().to_string();
            run(
                "useradd",
                &[
                    "--uid",
                    &uid_s,
                    "--home-dir",
                    &home_s,
                    "--create-home",
                    "--shell",
                    "/usr/sbin/nologin",
                    "--user-group",
                    &spec.username,
                ],
            )
            .await?;
        }

        let gid = self.gid_for(&spec.username)?;
        self.make_dir(&home, 0o750, uid, gid)?;
        for (relative, mode) in [
            ("public_html", 0o755),
            ("logs", 0o750),
            ("apps", 0o750),
            (".php/sessions", 0o700),
            ("tmp", 0o700),
        ] {
            self.make_dir(&home.join(relative), mode, uid, gid)?;
        }
        Ok(())
    }

    async fn remove_account(&self, username: &str, purge: bool) -> ExecResult<()> {
        let exists = User::from_name(username)
            .map_err(|e| ExecError::Io(e.to_string()))?
            .is_some();
        if exists {
            if purge {
                let _ = run("userdel", &["-r", username]).await;
            } else {
                let _ = run("userdel", &[username]).await;
            }
        }
        if purge {
            let home = self.home(username);
            if home.exists() {
                let _ = std::fs::remove_dir_all(home);
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
        let gid = self.gid_for(username)?;
        self.make_dir(path, 0o755, uid, gid)
    }

    async fn apply_site(&self, domain: &str, content: &str, enable: bool) -> ExecResult<()> {
        let filename = format!("{domain}.conf");
        let path = self.cfg.nginx_sites_dir.join(&filename);
        let link = self.cfg.nginx_enabled_dir.join(&filename);
        let previous = std::fs::read_to_string(&path).ok();
        let previous_link = std::fs::symlink_metadata(&link).is_ok();

        write_managed(
            &self.cfg.nginx_sites_dir,
            &path,
            content,
            0o644,
            Some((0, 0)),
        )?;
        std::fs::create_dir_all(&self.cfg.nginx_enabled_dir)?;
        if link.exists() || std::fs::symlink_metadata(&link).is_ok() {
            let _ = std::fs::remove_file(&link);
        }
        if enable {
            std::os::unix::fs::symlink(&path, &link)
                .map_err(|e| ExecError::Io(format!("symlink {}: {e}", link.display())))?;
        }

        if let Err(err) = self.nginx_test().await {
            let _ = std::fs::remove_file(&link);
            match &previous {
                Some(old) => {
                    let _ =
                        write_managed(&self.cfg.nginx_sites_dir, &path, old, 0o644, Some((0, 0)));
                }
                None => {
                    let _ = std::fs::remove_file(&path);
                }
            }
            if previous_link {
                let _ = std::os::unix::fs::symlink(&path, &link);
            }
            return Err(err);
        }
        self.reload_service("nginx").await
    }

    async fn remove_site(&self, domain: &str) -> ExecResult<()> {
        let filename = format!("{domain}.conf");
        let link = self.cfg.nginx_enabled_dir.join(&filename);
        let _ = std::fs::remove_file(&link);
        let _ = remove_managed(
            &self.cfg.nginx_sites_dir,
            &self.cfg.nginx_sites_dir.join(&filename),
        );
        if self.nginx_test().await.is_ok() {
            self.reload_service("nginx").await.ok();
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
        let path = dir.join(format!("rynx-{username}.conf"));
        if !dir.exists() {
            return Err(ExecError::NotFound(format!(
                "PHP {version} FPM is not installed ({})",
                dir.display()
            )));
        }
        std::fs::create_dir_all(&self.cfg.php_socket_dir)?;
        if enable {
            write_managed(&self.cfg.php_fpm_root, &path, content, 0o644, Some((0, 0)))?;
        } else {
            let _ = remove_managed(&self.cfg.php_fpm_root, &path);
        }

        let php = self.php_binary(version);
        run(&php, &["-t"]).await?;
        let service = format!("php{version}-fpm");
        self.reload_service(&service).await
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
        let php = self.php_binary(version);
        if run(&php, &["-t"]).await.is_ok() {
            self.reload_service(&format!("php{version}-fpm")).await.ok();
        }
        Ok(())
    }

    async fn ensure_app_dir(&self, username: &str, uid: u32, dir: &Path) -> ExecResult<()> {
        let home = self.home(username);
        if !dir.starts_with(&home) {
            return Err(ExecError::Unsupported(
                "application directory escapes the account home".into(),
            ));
        }
        let gid = self.gid_for(username)?;
        self.make_dir(dir, 0o750, uid, gid)
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
        let gid = self.gid_for(&spec.username)?;
        std::fs::create_dir_all(&self.cfg.app_env_dir)?;
        write_managed(
            &self.cfg.app_env_dir,
            &self.cfg.app_env_dir.join(env_filename),
            env_content,
            0o640,
            Some((0, gid)),
        )?;
        write_managed(
            &self.cfg.systemd_dir,
            &self.cfg.systemd_dir.join(unit),
            unit_content,
            0o644,
            Some((0, 0)),
        )?;
        run("systemctl", &["daemon-reload"]).await?;

        if enable {
            run("systemctl", &["enable", unit]).await?;
            if let Err(err) = run("systemctl", &["restart", unit]).await {
                tracing::warn!(unit, error = %err, "application unit failed to start; desired state kept");
            }
        } else if let Err(err) = run("systemctl", &["disable", "--now", unit]).await {
            tracing::warn!(unit, error = %err, "could not disable application unit");
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
        let _ = run("systemctl", &["disable", "--now", unit]).await;
        let _ = remove_managed(&self.cfg.systemd_dir, &self.cfg.systemd_dir.join(unit));
        let _ = remove_managed(
            &self.cfg.app_env_dir,
            &self.cfg.app_env_dir.join(env_filename),
        );
        let _ = run("systemctl", &["daemon-reload"]).await;
        if purge && working_dir.starts_with(&self.home(username)) {
            let _ = std::fs::remove_dir_all(working_dir);
        }
        Ok(())
    }

    async fn issue_cert(
        &self,
        domain: &str,
        domains: &[String],
        email: &str,
        webroot: &Path,
        staging: bool,
    ) -> ExecResult<()> {
        std::fs::create_dir_all(webroot)?;
        let webroot_s = webroot.display().to_string();
        let mut args = vec![
            "certonly".to_string(),
            "--webroot".into(),
            "-w".into(),
            webroot_s,
        ];
        let mut names = vec![domain.to_string()];
        for name in domains {
            if !names.iter().any(|existing| existing == name) {
                names.push(name.clone());
            }
        }
        for name in &names {
            args.push("-d".into());
            args.push(name.clone());
        }
        args.extend([
            "--non-interactive".into(),
            "--agree-tos".into(),
            "-m".into(),
            email.to_string(),
            "--keep-until-expiring".into(),
            "--expand".into(),
        ]);
        if staging {
            args.push("--staging".into());
        }
        run_owned(&self.cfg.certbot_bin, &args).await?;
        self.reload_service("nginx").await.ok();
        Ok(())
    }

    async fn renew_certs(&self, staging: bool) -> ExecResult<()> {
        let mut args = vec![
            "renew".to_string(),
            "--quiet".into(),
            "--non-interactive".into(),
        ];
        if staging {
            args.push("--staging".into());
        }
        run_owned(&self.cfg.certbot_bin, &args).await?;
        self.reload_service("nginx").await.ok();
        Ok(())
    }

    async fn remove_cert(&self, domain: &str) -> ExecResult<()> {
        let _ = run(
            &self.cfg.certbot_bin,
            &["delete", "--cert-name", domain, "--non-interactive"],
        )
        .await;
        Ok(())
    }

    async fn cert_info(&self, domain: &str) -> ExecResult<Option<CertView>> {
        let dir = self.cfg.certs_root.join(domain);
        let cert_path = dir.join("fullchain.pem");
        if !cert_path.exists() {
            return Ok(None);
        }
        let path_s = cert_path.display().to_string();
        let output = run(
            "openssl",
            &["x509", "-in", &path_s, "-noout", "-enddate", "-issuer"],
        )
        .await?;

        let mut issuer = "unknown".to_string();
        let mut not_after_raw = String::new();
        for line in output.lines() {
            if let Some(value) = line.strip_prefix("issuer=") {
                issuer = value.trim().to_string();
            }
            if let Some(value) = line.strip_prefix("notAfter=") {
                not_after_raw = value.trim().to_string();
            }
        }

        let parsed = parse_openssl_date(&not_after_raw);
        let (not_after, status) = match parsed {
            Some(date) => {
                let days = (date - OffsetDateTime::now_utc()).whole_days();
                let status = if days < 0 {
                    CertificateStatus::Expired
                } else if days < 30 {
                    CertificateStatus::Expiring
                } else {
                    CertificateStatus::Active
                };
                (
                    date.format(&Rfc3339)
                        .unwrap_or_else(|_| not_after_raw.clone()),
                    status,
                )
            }
            None => (not_after_raw, CertificateStatus::Active),
        };

        Ok(Some(CertView {
            domain: domain.to_string(),
            issuer,
            not_after,
            status,
            path: dir.display().to_string(),
        }))
    }

    async fn status_unit(&self, unit: &str) -> ExecResult<UnitStatus> {
        let output = run(
            "systemctl",
            &[
                "show",
                unit,
                "--no-pager",
                "-p",
                "ActiveState",
                "-p",
                "SubState",
                "-p",
                "MainPID",
                "-p",
                "MemoryCurrent",
                "-p",
                "CPUUsageNSec",
                "-p",
                "ActiveEnterTimestampMonotonic",
            ],
        )
        .await?;

        let mut map = std::collections::HashMap::new();
        for line in output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                map.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        let active_state = map
            .get("ActiveState")
            .cloned()
            .unwrap_or_else(|| "unknown".into());
        let sub_state = map.get("SubState").cloned().unwrap_or_default();
        let pid = map
            .get("MainPID")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let memory_raw = map
            .get("MemoryCurrent")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let memory_kb = if memory_raw > (1u64 << 60) {
            0
        } else {
            memory_raw / 1024
        };
        let cpu_ns = map
            .get("CPUUsageNSec")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let active_us = map
            .get("ActiveEnterTimestampMonotonic")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let boot_secs = std::fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|raw| raw.split_whitespace().next()?.parse::<f64>().ok())
            .unwrap_or(0.0) as u64;
        let started_secs = active_us / 1_000_000;
        let uptime_secs = if started_secs > 0 && active_us > 0 && boot_secs > started_secs {
            boot_secs - started_secs
        } else {
            0
        };
        let cpu_percent = if uptime_secs > 0 {
            ((cpu_ns as f64) / (uptime_secs as f64 * 1_000_000_000.0) * 100.0) as f32
        } else {
            0.0
        };

        Ok(UnitStatus {
            active: active_state == "active",
            state: format!("{active_state} ({sub_state})"),
            pid,
            memory_kb,
            cpu_percent,
            uptime_secs,
        })
    }

    async fn restart_unit(&self, unit: &str) -> ExecResult<()> {
        run("systemctl", &["restart", unit]).await.map(|_| ())
    }

    async fn reload_unit(&self, unit: &str) -> ExecResult<()> {
        run("systemctl", &["reload-or-restart", unit])
            .await
            .map(|_| ())
    }

    async fn set_unit_running(&self, unit: &str, running: bool) -> ExecResult<()> {
        let action = if running { "start" } else { "stop" };
        run("systemctl", &[action, unit]).await.map(|_| ())
    }

    async fn journal_unit(&self, unit: &str, lines: u32) -> ExecResult<String> {
        let lines = lines.clamp(10, 500).to_string();
        run(
            "journalctl",
            &["-u", unit, "-n", &lines, "--no-pager", "--output=short-iso"],
        )
        .await
    }

    async fn disk_usage(&self) -> ExecResult<String> {
        run("df", &["-h"]).await
    }

    async fn nginx_test_text(&self) -> ExecResult<String> {
        let args: Vec<String> = self
            .cfg
            .nginx_test_args
            .split_whitespace()
            .map(str::to_string)
            .collect();
        run_combined("nginx", &args).await
    }

    async fn create_backup(&self, paths: &[String], dest: &Path) -> ExecResult<u64> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let dest_s = dest.display().to_string();
        let mut args = vec!["-czf".to_string(), dest_s, "-C".into(), "/".into()];
        for path in paths {
            let relative = path.trim_start_matches('/');
            if !relative.is_empty() {
                args.push(relative.to_string());
            }
        }
        run_owned("tar", &args).await?;
        Ok(std::fs::metadata(dest)?.len())
    }

    async fn restore_backup(&self, archive: &Path, username: &str, uid: u32) -> ExecResult<()> {
        let archive_s = archive.display().to_string();
        let listing = run("tar", &["-tzf", &archive_s]).await?;
        let allowed = [
            format!("home/{username}/"),
            "etc/nginx/sites-available/rynxcp/".to_string(),
            "etc/bind/zones/".to_string(),
            "etc/php/".to_string(),
            "etc/systemd/system/rynxcp-".to_string(),
            "etc/rynxcp/apps/".to_string(),
        ];
        for entry in listing.lines() {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            if entry.starts_with('/') || entry.split('/').any(|part| part == "..") {
                return Err(ExecError::Unsupported(format!(
                    "backup entry escapes its root: {entry}"
                )));
            }
            if !allowed.iter().any(|prefix| entry.starts_with(prefix)) {
                return Err(ExecError::Unsupported(format!(
                    "backup entry is outside managed paths: {entry}"
                )));
            }
        }
        run(
            "tar",
            &[
                "-xzf",
                &archive_s,
                "-C",
                "/",
                "--no-same-owner",
                "--no-same-permissions",
            ],
        )
        .await?;

        let home = self.home(username);
        let gid = self.gid_for(username)?;
        if home.exists() {
            let home_s = home.display().to_string();
            run("chown", &["-R", &format!("{uid}:{gid}"), &home_s]).await?;
        }
        Ok(())
    }

    async fn delete_backup_file(&self, path: &Path) -> ExecResult<()> {
        let root = std::fs::canonicalize(&self.cfg.backup_dir)
            .unwrap_or_else(|_| self.cfg.backup_dir.clone());
        let Ok(target) = std::fs::canonicalize(path) else {
            return Ok(());
        };
        if !target.starts_with(&root) {
            return Err(ExecError::Unsupported(
                "refusing to delete a file outside the backup directory".into(),
            ));
        }
        std::fs::remove_file(target)?;
        Ok(())
    }

    async fn apply_zone(&self, domain: &str, content: &str) -> ExecResult<()> {
        std::fs::create_dir_all(&self.cfg.bind_zones_dir)?;
        let path = self.cfg.bind_zones_dir.join(format!("{domain}.db"));
        write_managed(
            &self.cfg.bind_zones_dir,
            &path,
            content,
            0o644,
            Some((0, 0)),
        )?;
        let path_s = path.display().to_string();
        let check = vec![domain.to_string(), path_s];
        run_combined(&self.cfg.bind_checkzone_bin, &check).await?;
        self.reload_unit(&self.cfg.bind_reload_unit).await
    }

    async fn remove_zone(&self, domain: &str) -> ExecResult<()> {
        let _ = remove_managed(
            &self.cfg.bind_zones_dir,
            &self.cfg.bind_zones_dir.join(format!("{domain}.db")),
        );
        Ok(())
    }

    async fn write_dns_conf(&self, content: &str) -> ExecResult<()> {
        let base = self
            .cfg
            .bind_conf_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/etc/bind"));
        write_managed(
            &base,
            &self.cfg.bind_conf_file,
            content,
            0o644,
            Some((0, 0)),
        )?;
        self.reload_unit(&self.cfg.bind_reload_unit).await
    }

    async fn generate_dkim_key(&self, domain: &str, selector: &str) -> ExecResult<String> {
        let dir = self.cfg.dkim_key_dir.join(domain);
        std::fs::create_dir_all(&dir)?;
        let private = dir.join(format!("{selector}.private"));
        let public = dir.join(format!("{selector}.public"));
        let private_s = private.display().to_string();
        let public_s = public.display().to_string();

        run(
            "openssl",
            &[
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
                "-out",
                &private_s,
            ],
        )
        .await?;
        run(
            "openssl",
            &["rsa", "-in", &private_s, "-pubout", "-out", &public_s],
        )
        .await?;

        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o600))?;
        std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o644))?;
        if let Ok(Some(user)) = User::from_name("opendkim") {
            let _ = chown(&private, Some(user.uid), Some(user.gid));
            let _ = chown(&public, Some(user.uid), Some(user.gid));
        }
        let pem = std::fs::read_to_string(&public)?;
        Ok(pem)
    }

    async fn write_dkim_tables(&self, keytable: &str, signingtable: &str) -> ExecResult<()> {
        let owner = User::from_name("opendkim").ok().flatten();
        if let Some(path) = &self.cfg.opendkim_keytable {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, keytable)?;
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o640));
            if let Some(user) = &owner {
                let _ = chown(path, Some(user.uid), Some(user.gid));
            }
        }
        if let Some(path) = &self.cfg.opendkim_signingtable {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, signingtable)?;
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o640));
            if let Some(user) = &owner {
                let _ = chown(path, Some(user.uid), Some(user.gid));
            }
        }
        let _ = self.reload_unit("opendkim").await;
        Ok(())
    }

    async fn create_database(&self, database: &str, user: &str, password: &str) -> ExecResult<()> {
        let sql = format!(
            "CREATE DATABASE IF NOT EXISTS `{database}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;\
             CREATE USER IF NOT EXISTS '{user}'@'localhost' IDENTIFIED BY '{password}';\
             ALTER USER '{user}'@'localhost' IDENTIFIED BY '{password}';\
             GRANT ALL PRIVILEGES ON `{database}`.* TO '{user}'@'localhost';\
             FLUSH PRIVILEGES;"
        );
        run(
            &self.cfg.mysql_bin,
            &["--protocol=socket", "-uroot", "-e", &sql],
        )
        .await
        .map(|_| ())
    }

    async fn drop_database(&self, database: &str, user: &str) -> ExecResult<()> {
        let sql = format!(
            "DROP DATABASE IF EXISTS `{database}`; DROP USER IF EXISTS '{user}'@'localhost'; FLUSH PRIVILEGES;"
        );
        run(
            &self.cfg.mysql_bin,
            &["--protocol=socket", "-uroot", "-e", &sql],
        )
        .await
        .map(|_| ())
    }

    async fn apply_ftp_user(
        &self,
        login: &str,
        uid: u32,
        gid: u32,
        home: &Path,
        password: &str,
    ) -> ExecResult<()> {
        std::fs::create_dir_all(home)?;
        let passwd_file = self.cfg.pure_passwd.display().to_string();
        let _ = run(
            &self.cfg.pure_pw_bin,
            &["userdel", login, "-f", &passwd_file],
        )
        .await;

        let mut command = tokio::process::Command::new(&self.cfg.pure_pw_bin);
        command
            .args([
                "useradd",
                login,
                "-u",
                &uid.to_string(),
                "-g",
                &gid.to_string(),
                "-d",
                &home.display().to_string(),
                "-f",
                &passwd_file,
            ])
            .env_clear()
            .env("PATH", SAFE_PATH)
            .env("LC_ALL", "C")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|e| ExecError::Command {
            program: self.cfg.pure_pw_bin.clone(),
            args: format!("useradd {login}"),
            status: -1,
            stderr: e.to_string(),
        })?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(password.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.write_all(password.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }
        let output = child.wait_with_output().await?;
        if !output.status.success() {
            return Err(ExecError::Command {
                program: self.cfg.pure_pw_bin.clone(),
                args: format!("useradd {login}"),
                status: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let pdb = self.cfg.pure_pdb.display().to_string();
        run_owned(
            &self.cfg.pure_pw_bin,
            &["mkdb".to_string(), pdb, "-f".into(), passwd_file],
        )
        .await?;
        let _ = self.reload_unit(&self.cfg.pure_ftpd_unit).await;
        Ok(())
    }

    async fn remove_ftp_user(&self, login: &str) -> ExecResult<()> {
        let passwd_file = self.cfg.pure_passwd.display().to_string();
        let _ = run(
            &self.cfg.pure_pw_bin,
            &["userdel", login, "-f", &passwd_file],
        )
        .await;
        let pdb = self.cfg.pure_pdb.display().to_string();
        let _ = run_owned(
            &self.cfg.pure_pw_bin,
            &["mkdb".to_string(), pdb, "-f".into(), passwd_file],
        )
        .await;
        Ok(())
    }

    async fn apply_firewall(&self, ruleset: &str) -> ExecResult<()> {
        std::fs::create_dir_all(&self.cfg.state_dir)?;
        let path = self.cfg.state_dir.join("firewall.nft");
        std::fs::write(&path, ruleset)?;
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        let path_s = path.display().to_string();
        run(&self.cfg.nft_bin, &["-f", &path_s]).await.map(|_| ())
    }

    async fn resolve_user_ids(&self, username: &str) -> ExecResult<Option<(u32, u32)>> {
        match User::from_name(username) {
            Ok(Some(user)) => Ok(Some((user.uid.as_raw(), user.gid.as_raw()))),
            Ok(None) => Ok(None),
            Err(e) => Err(ExecError::Io(e.to_string())),
        }
    }

    async fn installed_programs(&self) -> ExecResult<Vec<ProgramInfo>> {
        async fn probe(program: &str, args: &[&str]) -> Option<String> {
            run_combined(
                program,
                &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            )
            .await
            .ok()
            .and_then(|output| {
                output
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty())
                    .map(|line| line.to_string())
            })
        }

        let mut out = Vec::new();
        let mut push = |name: &str, kind: &str, version: Option<String>, detail: &str| {
            let installed = version.is_some();
            out.push(ProgramInfo {
                name: name.to_string(),
                version: version.unwrap_or_else(|| "Ã¢â‚¬â€".into()),
                kind: kind.to_string(),
                status: if installed { "installed" } else { "missing" }.into(),
                detail: detail.to_string(),
            });
        };

        push(
            "nginx",
            "web",
            probe("nginx", &["-v"]).await,
            "reverse proxy",
        );
        if let Ok(entries) = std::fs::read_dir(&self.cfg.php_fpm_root) {
            let mut versions: Vec<String> = entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
                .filter(|name| name.chars().next().is_some_and(|c| c.is_ascii_digit()))
                .collect();
            versions.sort();
            for version in versions {
                let binary = format!("php{version}");
                let found = probe(&binary, &["-v"]).await;
                push(&format!("php{version}"), "runtime", found, "PHP-FPM");
            }
        }
        push(
            "node",
            "runtime",
            probe("node", &["--version"]).await,
            "systemd apps",
        );
        push(
            "python3",
            "runtime",
            probe("python3", &["--version"]).await,
            "gunicorn/uvicorn",
        );
        push(
            "ruby",
            "runtime",
            probe("ruby", &["--version"]).await,
            "puma",
        );
        push(
            "java",
            "runtime",
            probe("java", &["-version"]).await,
            "JAR apps",
        );
        push(
            "dotnet",
            "runtime",
            probe("dotnet", &["--version"]).await,
            "ASP.NET apps",
        );
        push(
            "go",
            "runtime",
            probe("go", &["version"]).await,
            "compiled binaries",
        );
        push(
            "mariadb",
            "database",
            probe("mariadb", &["--version"]).await,
            "per-domain databases",
        );
        push(
            "bind9",
            "dns",
            probe("named", &["-v"]).await,
            "authoritative DNS",
        );
        push(
            "certbot",
            "security",
            probe("certbot", &["--version"]).await,
            "Let's Encrypt",
        );
        push(
            "pure-ftpd",
            "web",
            probe("pure-ftpd", &["-v"]).await,
            "FTPS virtual users",
        );
        push(
            "opendkim",
            "mail",
            probe("opendkim", &["-V"]).await,
            "DKIM signing",
        );
        push(
            "fail2ban",
            "security",
            probe("fail2ban-client", &["--version"]).await,
            "brute-force jail",
        );
        push(
            "rynxcp",
            "panel",
            Some(format!("v{}", rynx_proto::DAEMON_VERSION)),
            "control plane",
        );
        Ok(out)
    }

    async fn available_updates(&self) -> ExecResult<Vec<PackageUpdate>> {
        let output = run("apt", &["list", "--upgradable"]).await?;
        let mut updates = Vec::new();
        for line in output.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() || !line.contains('/') {
                continue;
            }
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 3 {
                continue;
            }
            let package = fields[0].split('/').next().unwrap_or_default().to_string();
            if package.is_empty() {
                continue;
            }
            let candidate = fields[1].trim_end_matches(',').to_string();
            let current = line
                .split("upgradable from:")
                .nth(1)
                .map(|value| value.trim().trim_end_matches(']').to_string())
                .unwrap_or_default();
            let security = fields[0].contains("security") || line.contains("-security");
            updates.push(PackageUpdate {
                package,
                current_version: current,
                candidate_version: candidate,
                security,
            });
        }
        updates.sort_by(|a, b| a.package.cmp(&b.package));
        Ok(updates)
    }

    async fn apply_updates(&self) -> ExecResult<String> {
        let mut combined = String::new();
        for args in [
            vec!["update"],
            vec![
                "-y",
                "-o",
                "Dpkg::Options::=--force-confdef",
                "-o",
                "Dpkg::Options::=--force-confold",
                "upgrade",
            ],
        ] {
            let output = tokio::process::Command::new("apt-get")
                .args(&args)
                .env_clear()
                .env("PATH", SAFE_PATH)
                .env("LC_ALL", "C")
                .env("DEBIAN_FRONTEND", "noninteractive")
                .output()
                .await
                .map_err(|e| ExecError::Command {
                    program: "apt-get".into(),
                    args: args.join(" "),
                    status: -1,
                    stderr: e.to_string(),
                })?;
            combined.push_str(&String::from_utf8_lossy(&output.stdout));
            combined.push_str(&String::from_utf8_lossy(&output.stderr));
            if !output.status.success() {
                return Err(ExecError::Command {
                    program: "apt-get".into(),
                    args: args.join(" "),
                    status: output.status.code().unwrap_or(-1),
                    stderr: combined,
                });
            }
        }
        Ok(combined)
    }

    async fn system_info(&self) -> ExecResult<SystemInfo> {
        let hostname = read_trim("/proc/sys/kernel/hostname").unwrap_or_else(|| "unknown".into());
        let kernel = read_trim("/proc/sys/kernel/osrelease").unwrap_or_else(|| "unknown".into());
        let os = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|raw| {
                raw.lines()
                    .find(|line| line.starts_with("PRETTY_NAME="))
                    .map(|line| {
                        line.trim_start_matches("PRETTY_NAME=")
                            .trim_matches('"')
                            .to_string()
                    })
            })
            .unwrap_or_else(|| "Linux".into());
        let uptime_secs = std::fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|raw| raw.split_whitespace().next()?.parse::<f64>().ok())
            .unwrap_or(0.0) as u64;
        let load = std::fs::read_to_string("/proc/loadavg")
            .ok()
            .map(|raw| {
                let mut values = [0u32; 3];
                for (idx, part) in raw.split_whitespace().take(3).enumerate() {
                    values[idx] = (part.parse::<f64>().unwrap_or(0.0) * 100.0).round() as u32;
                }
                values
            })
            .unwrap_or([0, 0, 0]);

        let (mem_total_kb, mem_available_kb) = read_meminfo();
        let (disk_total_kb, disk_available_kb) = read_disk().await.unwrap_or((0, 0));

        Ok(SystemInfo {
            hostname,
            os,
            kernel,
            uptime_secs,
            load,
            cpu_count: std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(1),
            mem_total_kb,
            mem_available_kb,
            disk_total_kb,
            disk_available_kb,
            daemon_version: rynx_proto::DAEMON_VERSION.into(),
            mode: "linux".into(),
        })
    }

    async fn uid_in_use(&self, uid: u32) -> ExecResult<bool> {
        Ok(User::from_uid(Uid::from_raw(uid))
            .map_err(|e| ExecError::Io(e.to_string()))?
            .is_some())
    }
}

fn read_trim(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn read_meminfo() -> (u64, u64) {
    let raw = match std::fs::read_to_string("/proc/meminfo") {
        Ok(raw) => raw,
        Err(_) => return (0, 0),
    };
    let mut total = 0u64;
    let mut available = 0u64;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        }
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        }
    }
    (total, available)
}

async fn read_disk() -> Option<(u64, u64)> {
    let output = run("df", &["-k", "/"]).await.ok()?;
    let line = output.lines().nth(1)?;
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 4 {
        return None;
    }
    Some((fields[1].parse().ok()?, fields[3].parse().ok()?))
}

fn parse_openssl_date(raw: &str) -> Option<OffsetDateTime> {
    let cleaned = raw.trim().trim_end_matches(" GMT").trim();
    let fmt = format_description!(
        "[month repr:short] [day padding:space] [hour]:[minute]:[second] [year]"
    );
    OffsetDateTime::parse(cleaned, &fmt).ok()
}
