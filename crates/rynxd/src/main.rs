use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use rynx_core::audit::AuditLog;
use rynx_core::config::validate_daemon_config;
use rynx_core::crypto::load_or_create_token;
use rynx_core::DaemonConfig;
use rynx_proto::DAEMON_VERSION;

use rynxd::executor;
use rynxd::reconciler::Reconciler;
use rynxd::server::Server;
use rynxd::state::StateStore;

#[derive(Debug, Parser)]
#[command(name = "rynxd", version, about = "RYNXCP privileged daemon")]
struct Args {
    /// Path to the daemon configuration (TOML).
    #[arg(long, env = "RYNXCP_CONFIG")]
    config: Option<PathBuf>,
    /// Validate configuration and exit.
    #[arg(long)]
    check: bool,
}

fn default_config_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("rynxd.toml")
    } else {
        PathBuf::from("/etc/rynxcp/rynxd.toml")
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(default_config_path);

    let mut cfg = DaemonConfig::load(&config_path)
        .with_context(|| format!("cannot load {}", config_path.display()))?;
    if cfg.simulate {
        cfg = cfg.sandboxed();
        tracing::warn!("simulation mode: no operating system changes will be made");
    }
    validate_daemon_config(&cfg)?;

    #[cfg(unix)]
    if !cfg.simulate {
        use nix::unistd::Uid;
        if !Uid::effective().is_root() {
            bail!("rynxd must run as root (or enable simulate for development)");
        }
    }

    std::fs::create_dir_all(&cfg.state_dir)
        .with_context(|| format!("cannot create {}", cfg.state_dir.display()))?;

    let token = match &cfg.token {
        Some(token) if token.len() >= 32 => token.clone(),
        Some(_) => bail!("configured daemon token is too short (minimum 32 characters)"),
        None => load_or_create_token(&cfg.token_file)?,
    };

    let state = Arc::new(StateStore::open(&cfg.state_dir.join("state.db"))?);
    let audit = Arc::new(AuditLog::open(&cfg.audit_log)?);
    let exec = executor::build(&cfg)?;

    tracing::info!(
        version = DAEMON_VERSION,
        mode = exec.mode(),
        state_dir = %cfg.state_dir.display(),
        audit = %cfg.audit_log.display(),
        token_file = %cfg.token_file.display(),
        "starting rynxd"
    );

    let reconciler = Arc::new(Reconciler {
        cfg: Arc::new(cfg.clone()),
        state,
        audit,
        exec,
    });

    if args.check {
        println!(
            "configuration OK: mode={} state_dir={} socket={}",
            reconciler.mode(),
            cfg.state_dir.display(),
            cfg.socket_path.display()
        );
        return Ok(());
    }

    {
        let sampler = reconciler.clone();
        tokio::spawn(async move {
            let _ = sampler.sample_metrics().await;
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await;
            loop {
                interval.tick().await;
                if let Err(err) = sampler.sample_metrics().await {
                    tracing::warn!(error = %err.message, "metrics sample failed");
                }
            }
        });
    }

    let server = Server {
        cfg: Arc::new(cfg),
        reconciler,
        token,
    };
    server.run().await
}
