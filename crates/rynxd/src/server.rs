//! Local transport server. Accepts length-prefixed CBOR frames, authenticates
//! every request and dispatches it to the reconciler.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use rynx_core::crypto::constant_time_eq;
use rynx_core::DaemonConfig;
use rynx_proto::{
    decode, encode, recv_frame, send_frame, Envelope, ErrorCode, Request, ServerEnvelope,
    WireError, PROTOCOL_VERSION,
};

use crate::reconciler::Reconciler;

const READ_TIMEOUT: Duration = Duration::from_secs(30);
const DISPATCH_TIMEOUT: Duration = Duration::from_secs(900);
const MAX_CONCURRENT: usize = 16;
const MAX_REQUESTS_PER_CONNECTION: usize = 1024;

pub struct Server {
    pub cfg: Arc<DaemonConfig>,
    pub reconciler: Arc<Reconciler>,
    pub token: String,
}

struct Context {
    reconciler: Arc<Reconciler>,
    token: String,
    permits: Arc<Semaphore>,
}

impl Server {
    pub async fn run(self) -> Result<()> {
        let ctx = Arc::new(Context {
            reconciler: self.reconciler.clone(),
            token: self.token.clone(),
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT)),
        });

        let use_tcp = cfg!(windows) || (self.cfg.simulate && self.cfg.tcp_listen.is_some());
        if use_tcp {
            let addr = self
                .cfg
                .tcp_listen
                .clone()
                .unwrap_or_else(|| "127.0.0.1:7801".into());
            if !addr.starts_with("127.")
                && !addr.starts_with("localhost")
                && !addr.starts_with("[::1]")
            {
                bail!("refusing to listen on non-loopback TCP address: {addr}");
            }
            let listener = TcpListener::bind(&addr)
                .await
                .with_context(|| format!("cannot bind {addr}"))?;
            tracing::info!(%addr, mode = self.reconciler.mode(), "rynxd listening on tcp");
            loop {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        tracing::info!("shutdown requested");
                        break;
                    }
                    accepted = listener.accept() => {
                        let (stream, peer) = accepted?;
                        tracing::debug!(%peer, "tcp client connected");
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            if let Err(err) = handle_connection(stream, true, ctx).await {
                                tracing::debug!(%peer, error = %err, "connection closed");
                            }
                        });
                    }
                }
            }
            return Ok(());
        }

        #[cfg(unix)]
        {
            self.run_unix(ctx).await
        }
        #[cfg(not(unix))]
        {
            bail!("unix sockets are not available on this platform")
        }
    }

    #[cfg(unix)]
    async fn run_unix(self, ctx: Arc<Context>) -> Result<()> {
        use nix::unistd::{chown, Gid, Group, Uid};

        let path = self.cfg.socket_path.clone();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if std::fs::symlink_metadata(&path).is_ok() {
            let _ = std::fs::remove_file(&path);
        }
        let listener = tokio::net::UnixListener::bind(&path)
            .with_context(|| format!("cannot bind {}", path.display()))?;

        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o660))?;
        }
        let group_gid = Group::from_name(&self.cfg.socket_group)
            .ok()
            .flatten()
            .map(|g| g.gid.as_raw());
        if let Some(gid) = group_gid {
            if let Err(err) = chown(&path, Some(Uid::from_raw(0)), Some(Gid::from_raw(gid))) {
                tracing::warn!(error = %err, "could not chown daemon socket");
            }
        } else {
            tracing::warn!(
                group = %self.cfg.socket_group,
                "socket group does not exist; only root may connect"
            );
        }

        tracing::info!(socket = %path.display(), mode = self.reconciler.mode(), "rynxd listening on unix socket");

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("shutdown requested");
                    let _ = std::fs::remove_file(&path);
                    break;
                }
                accepted = listener.accept() => {
                    let (stream, _addr) = accepted?;
                    let authorized = match stream.peer_cred() {
                        Ok(cred) => {
                            let uid = cred.uid();
                            let gid = cred.gid();
                            uid == 0
                                || group_gid == Some(gid)
                                || uid == nix::unistd::Uid::effective().as_raw()
                        }
                        Err(err) => {
                            tracing::warn!(error = %err, "cannot read peer credentials");
                            false
                        }
                    };
                    if !authorized {
                        tracing::warn!("rejected client with invalid peer credentials");
                        continue;
                    }
                    let ctx = ctx.clone();
                    tokio::spawn(async move {
                        if let Err(err) = handle_connection(stream, true, ctx).await {
                            tracing::debug!(error = %err, "connection closed");
                        }
                    });
                }
            }
        }
        Ok(())
    }
}

fn sanitize_actor(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return "unknown".into();
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "._@-".contains(c))
    {
        trimmed.to_string()
    } else {
        "unknown".into()
    }
}

async fn handle_connection<S>(stream: S, authorized: bool, ctx: Arc<Context>) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    for _ in 0..MAX_REQUESTS_PER_CONNECTION {
        let frame = match timeout(READ_TIMEOUT, recv_frame(&mut reader)).await {
            Err(_) => break,
            Ok(Err(_)) => break,
            Ok(Ok(frame)) => frame,
        };

        let envelope: Envelope<Request> = match decode(&frame) {
            Ok(envelope) => envelope,
            Err(err) => {
                tracing::warn!(error = %err, "malformed request frame");
                break;
            }
        };

        let request_id = envelope.request_id.clone();
        let actor = sanitize_actor(&envelope.actor);

        let result = if envelope.protocol != PROTOCOL_VERSION {
            Err(WireError::new(
                ErrorCode::Protocol,
                format!(
                    "protocol version {} not supported (daemon speaks {})",
                    envelope.protocol, PROTOCOL_VERSION
                ),
            ))
        } else if !authorized || !constant_time_eq(&envelope.auth, &ctx.token) {
            tracing::warn!(actor = %actor, "rejected request with invalid token");
            Err(WireError::unauthorized())
        } else {
            let permit = ctx
                .permits
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| anyhow::anyhow!("daemon shutting down"))?;
            let outcome = timeout(
                DISPATCH_TIMEOUT,
                ctx.reconciler.dispatch(envelope.body, &actor),
            )
            .await;
            drop(permit);
            match outcome {
                Ok(result) => result,
                Err(_) => Err(WireError::internal("operation timed out")),
            }
        };

        let response = ServerEnvelope {
            protocol: PROTOCOL_VERSION,
            request_id,
            result,
        };
        let payload = encode(&response)?;
        if timeout(READ_TIMEOUT, send_frame(&mut writer, &payload))
            .await
            .is_err()
        {
            break;
        }
    }
    Ok(())
}
