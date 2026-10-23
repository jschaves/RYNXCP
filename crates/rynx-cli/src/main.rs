use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use rynx_api::daemon::DaemonClient;
use rynx_core::crypto::random_password;
use rynx_core::ApiConfig;
use rynx_panel::{PanelDb, Role};
use rynx_proto::{Request, Response};

#[derive(Debug, Parser)]
#[command(name = "rynxctl", version, about = "RYNXCP administration CLI")]
struct Cli {
    /// API configuration containing database and daemon connection settings.
    #[arg(long, global = true, env = "RYNXCP_API_CONFIG")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage panel administrators.
    Admin {
        #[command(subcommand)]
        cmd: AdminCmd,
    },
    /// Inspect the privileged daemon.
    Daemon {
        #[command(subcommand)]
        cmd: DaemonCmd,
    },
    /// Print the most recent audit entries.
    Audit {
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Re-apply every persisted resource (drift repair).
    Reconcile,
    /// Certificate operations.
    Cert {
        #[command(subcommand)]
        cmd: CertCmd,
    },
    /// Backup operations.
    Backup {
        #[command(subcommand)]
        cmd: BackupCmd,
    },
    /// Print live service statistics.
    Monitor,
}

#[derive(Debug, Subcommand)]
enum AdminCmd {
    /// Create a panel user.
    Create {
        username: String,
        #[arg(long)]
        email: String,
        /// Password. If omitted a strong one is generated and printed once.
        #[arg(long, env = "RYNXCTL_PASSWORD")]
        password: Option<String>,
        #[arg(long, default_value = "owner")]
        role: String,
    },
    /// List panel users.
    List,
    /// Reset a user's password.
    Passwd {
        username: String,
        #[arg(long, env = "RYNXCTL_PASSWORD")]
        password: Option<String>,
    },
    /// Disable a panel user and revoke their sessions.
    Disable { username: String },
    /// Re-enable a panel user.
    Enable { username: String },
}

#[derive(Debug, Subcommand)]
enum CertCmd {
    /// Renew every Let's Encrypt certificate and re-render the vhosts.
    Renew {
        #[arg(long)]
        staging: bool,
    },
}

#[derive(Debug, Subcommand)]
enum BackupCmd {
    /// Create a backup for every hosting account.
    All,
    /// List stored backups.
    List,
}

#[derive(Debug, Subcommand)]
enum DaemonCmd {
    Ping,
    Info,
}

fn default_config_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("rynx-api.toml")
    } else {
        PathBuf::from("/etc/rynxcp/api.toml")
    }
}

fn load_config(args: &Cli) -> Result<ApiConfig> {
    let path = args.config.clone().unwrap_or_else(default_config_path);
    ApiConfig::load(&path).with_context(|| format!("cannot load {}", path.display()))
}

fn parse_role(raw: &str) -> Result<Role> {
    Role::parse(raw)
        .ok_or_else(|| anyhow::anyhow!("invalid role '{raw}' (owner, admin, operator, viewer)"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let cfg = load_config(&args)?;

    match args.command {
        Command::Admin { cmd } => {
            let db = PanelDb::open(&cfg.database)?;
            match cmd {
                AdminCmd::Create {
                    username,
                    email,
                    password,
                    role,
                } => {
                    let role = parse_role(&role)?;
                    let (password, generated) = match password {
                        Some(p) => (p, false),
                        None => (random_password(24), true),
                    };
                    let user = db.create_user(&username, &email, &password, role)?;
                    println!(
                        "created user '{}' with role '{}'",
                        user.username,
                        user.role.as_str()
                    );
                    if generated {
                        println!("one-time password: {password}");
                        println!("store it now; it will not be shown again");
                    }
                }
                AdminCmd::List => {
                    for user in db.list_users()? {
                        println!(
                            "{:<24} {:<32} {:<10} {}",
                            user.username,
                            user.email,
                            user.role.as_str(),
                            if user.active { "active" } else { "disabled" }
                        );
                    }
                }
                AdminCmd::Passwd { username, password } => {
                    let (password, generated) = match password {
                        Some(p) => (p, false),
                        None => (random_password(24), true),
                    };
                    db.set_password(&username, &password)?;
                    println!("password updated for '{username}'");
                    if generated {
                        println!("one-time password: {password}");
                    }
                }
                AdminCmd::Disable { username } => {
                    db.set_active(&username, false)?;
                    println!("user '{username}' disabled and sessions revoked");
                }
                AdminCmd::Enable { username } => {
                    db.set_active(&username, true)?;
                    println!("user '{username}' enabled");
                }
            }
            Ok(())
        }
        Command::Daemon { cmd } => {
            let client = DaemonClient::new(&cfg)?;
            let response = match cmd {
                DaemonCmd::Ping => client.call(Request::Health, "rynxctl").await?,
                DaemonCmd::Info => client.call(Request::SystemInfo, "rynxctl").await?,
            };
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Command::Audit { limit } => {
            let client = DaemonClient::new(&cfg)?;
            match client.call(Request::AuditTail { limit }, "rynxctl").await? {
                Response::Audit { entries } => {
                    for entry in entries {
                        println!(
                            "#{:<6} {} {:<20} {:<32} {:<18} {}",
                            entry.seq,
                            entry.at,
                            entry.actor,
                            entry.action,
                            entry.target,
                            entry.detail
                        );
                    }
                }
                other => bail!("unexpected daemon response: {other:?}"),
            }
            Ok(())
        }
        Command::Cert { cmd } => {
            let client = DaemonClient::new(&cfg)?;
            match cmd {
                CertCmd::Renew { staging } => {
                    match client
                        .call(Request::CertRenew { staging }, "rynxctl")
                        .await?
                    {
                        Response::Ack { message } => println!("{message}"),
                        other => bail!("unexpected daemon response: {other:?}"),
                    }
                }
            }
            Ok(())
        }
        Command::Backup { cmd } => {
            let client = DaemonClient::new(&cfg)?;
            match cmd {
                BackupCmd::All => {
                    let accounts = match client.call(Request::AccountList, "rynxctl").await? {
                        Response::Accounts { accounts } => accounts,
                        other => bail!("unexpected daemon response: {other:?}"),
                    };
                    for account in accounts {
                        match client
                            .call(
                                Request::BackupCreate {
                                    username: account.spec.username.clone(),
                                },
                                "rynxctl",
                            )
                            .await
                        {
                            Ok(Response::Backup { backup }) => println!(
                                "backup {} for {} ({} bytes)",
                                backup.id, backup.username, backup.size_bytes
                            ),
                            Ok(other) => println!("unexpected response: {other:?}"),
                            Err(err) => eprintln!("{}: {err}", account.spec.username),
                        }
                    }
                    Ok(())
                }
                BackupCmd::List => {
                    match client.call(Request::BackupList, "rynxctl").await? {
                        Response::Backups { backups } => {
                            for backup in backups {
                                println!(
                                    "{:<40} {:<20} {:>12} bytes  {}",
                                    backup.id, backup.username, backup.size_bytes, backup.path
                                );
                            }
                        }
                        other => bail!("unexpected daemon response: {other:?}"),
                    }
                    Ok(())
                }
            }
        }
        Command::Monitor => {
            let client = DaemonClient::new(&cfg)?;
            match client.call(Request::ServiceStats, "rynxctl").await? {
                Response::ServiceStats { services } => {
                    println!(
                        "{:<8} {:<10} {:<24} {:>8} {:>10} {:>10}",
                        "KIND", "STATUS", "NAME", "CPU%", "MEM", "UPTIME"
                    );
                    for service in services {
                        println!(
                            "{:<8} {:<10} {:<24} {:>8.2} {:>10} {:>10}",
                            service.kind,
                            service.status,
                            service.name,
                            service.cpu_percent,
                            format!("{}MB", service.memory_kb / 1024),
                            format!("{}s", service.uptime_secs)
                        );
                    }
                }
                other => bail!("unexpected daemon response: {other:?}"),
            }
            Ok(())
        }
        Command::Reconcile => {
            let client = DaemonClient::new(&cfg)?;
            match client.call(Request::ReconcileAll, "rynxctl").await? {
                Response::Ack { message } => {
                    println!("{message}");
                    Ok(())
                }
                other => bail!("unexpected daemon response: {other:?}"),
            }
        }
    }
}
