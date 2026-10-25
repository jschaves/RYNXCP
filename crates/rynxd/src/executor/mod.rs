//! Privileged system mutations, isolated behind a trait so the whole panel can
//! run against a disposable sandbox during development.

pub mod sim;
#[cfg(unix)]
pub mod unix;

pub use sim::SimExecutor;
#[cfg(unix)]
pub use unix::RealExecutor;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;

use rynx_core::{CoreError, DaemonConfig};
use rynx_proto::{AccountSpec, AppSpec, CertView, PackageUpdate, ProgramInfo, SystemInfo};

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("command `{program} {args}` failed with status {status}: {stderr}")]
    Command {
        program: String,
        args: String,
        status: i32,
        stderr: String,
    },
    #[error("io error: {0}")]
    Io(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("account '{0}' does not exist")]
    MissingAccount(String),
}

impl From<std::io::Error> for ExecError {
    fn from(value: std::io::Error) -> Self {
        ExecError::Io(value.to_string())
    }
}

impl From<CoreError> for ExecError {
    fn from(value: CoreError) -> Self {
        ExecError::Io(value.to_string())
    }
}

pub type ExecResult<T> = Result<T, ExecError>;

#[derive(Debug, Clone, Default)]
pub struct UnitStatus {
    pub active: bool,
    pub state: String,
    pub pid: u32,
    pub memory_kb: u64,
    pub cpu_percent: f32,
    pub uptime_secs: u64,
}

#[async_trait]
pub trait Executor: Send + Sync {
    fn mode(&self) -> &'static str;
    fn home(&self, username: &str) -> PathBuf;
    fn apps_dir(&self, username: &str) -> PathBuf;
    fn working_dir(&self, username: &str, app: &str) -> PathBuf;
    fn acme_root(&self) -> PathBuf;

    async fn ensure_account(&self, spec: &AccountSpec, uid: u32) -> ExecResult<()>;
    async fn remove_account(&self, username: &str, purge: bool) -> ExecResult<()>;
    async fn ensure_docroot(&self, username: &str, uid: u32, path: &Path) -> ExecResult<()>;

    #[allow(clippy::too_many_arguments)]
    async fn ensure_welcome(
        &self,
        username: &str,
        uid: u32,
        docroot: &Path,
        domain: &str,
        locale: &str,
        php: bool,
    ) -> ExecResult<()>;

    async fn apply_site(&self, domain: &str, content: &str, enable: bool) -> ExecResult<()>;
    async fn remove_site(&self, domain: &str) -> ExecResult<()>;

    async fn apply_php_pool(
        &self,
        username: &str,
        version: &str,
        content: &str,
        enable: bool,
    ) -> ExecResult<()>;
    async fn remove_php_pool(&self, username: &str, version: &str) -> ExecResult<()>;

    async fn ensure_app_dir(&self, username: &str, uid: u32, dir: &Path) -> ExecResult<()>;
    #[allow(clippy::too_many_arguments)]
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
    ) -> ExecResult<()>;
    async fn remove_app(
        &self,
        username: &str,
        unit: &str,
        env_filename: &str,
        working_dir: &Path,
        purge: bool,
    ) -> ExecResult<()>;

    async fn issue_cert(
        &self,
        domain: &str,
        domains: &[String],
        email: &str,
        webroot: &Path,
        staging: bool,
    ) -> ExecResult<()>;
    async fn renew_certs(&self, staging: bool) -> ExecResult<()>;
    async fn remove_cert(&self, domain: &str) -> ExecResult<()>;
    async fn cert_info(&self, domain: &str) -> ExecResult<Option<CertView>>;

    async fn status_unit(&self, unit: &str) -> ExecResult<UnitStatus>;
    async fn restart_unit(&self, unit: &str) -> ExecResult<()>;
    async fn reload_unit(&self, unit: &str) -> ExecResult<()>;
    async fn set_unit_running(&self, unit: &str, running: bool) -> ExecResult<()>;
    async fn journal_unit(&self, unit: &str, lines: u32) -> ExecResult<String>;
    async fn disk_usage(&self) -> ExecResult<String>;
    async fn nginx_test_text(&self) -> ExecResult<String>;

    async fn create_backup(&self, paths: &[String], dest: &Path) -> ExecResult<u64>;
    async fn restore_backup(&self, archive: &Path, username: &str, uid: u32) -> ExecResult<()>;
    async fn delete_backup_file(&self, path: &Path) -> ExecResult<()>;
    async fn remove_backup_dir(&self, username: &str) -> ExecResult<()>;

    async fn apply_zone(&self, domain: &str, content: &str) -> ExecResult<()>;
    async fn remove_zone(&self, domain: &str) -> ExecResult<()>;
    async fn write_dns_conf(&self, content: &str) -> ExecResult<()>;
    async fn generate_dkim_key(&self, domain: &str, selector: &str) -> ExecResult<String>;
    async fn write_dkim_tables(&self, keytable: &str, signingtable: &str) -> ExecResult<()>;

    async fn create_database(&self, database: &str, user: &str, password: &str) -> ExecResult<()>;
    async fn drop_database(&self, database: &str, user: &str) -> ExecResult<()>;

    async fn apply_ftp_user(
        &self,
        login: &str,
        uid: u32,
        gid: u32,
        home: &Path,
        password: &str,
    ) -> ExecResult<()>;
    async fn remove_ftp_user(&self, login: &str) -> ExecResult<()>;
    async fn apply_firewall(&self, ruleset: &str) -> ExecResult<()>;
    async fn resolve_user_ids(&self, username: &str) -> ExecResult<Option<(u32, u32)>>;

    async fn installed_programs(&self) -> ExecResult<Vec<ProgramInfo>>;
    async fn available_updates(&self) -> ExecResult<Vec<PackageUpdate>>;
    async fn apply_updates(&self) -> ExecResult<String>;

    async fn system_info(&self) -> ExecResult<SystemInfo>;
    async fn uid_in_use(&self, uid: u32) -> ExecResult<bool>;
}

pub fn build(cfg: &DaemonConfig) -> anyhow::Result<Arc<dyn Executor>> {
    if cfg.simulate {
        Ok(Arc::new(SimExecutor::new(cfg.clone())))
    } else {
        #[cfg(unix)]
        {
            Ok(Arc::new(RealExecutor::new(cfg.clone())))
        }
        #[cfg(not(unix))]
        {
            anyhow::bail!(
                "simulate=false is only supported on Linux; enable simulate for this platform"
            )
        }
    }
}
