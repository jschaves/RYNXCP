//! Client for the privileged daemon. One authenticated connection per call.

use std::path::PathBuf;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::timeout;

use rynx_core::crypto::{random_hex, read_token};
use rynx_core::ApiConfig;
use rynx_proto::{
    decode, encode, recv_frame, send_frame, Envelope, Request, Response, ServerEnvelope,
    PROTOCOL_VERSION,
};

use crate::error::ApiError;

const CALL_TIMEOUT: Duration = Duration::from_secs(910);

pub struct DaemonClient {
    socket: PathBuf,
    tcp: Option<String>,
    token: String,
}

impl DaemonClient {
    pub fn new(cfg: &ApiConfig) -> anyhow::Result<Self> {
        let token = match &cfg.daemon_token {
            Some(token) if token.len() >= 32 => token.clone(),
            Some(_) => anyhow::bail!("configured daemon token is too short"),
            None => read_token(&cfg.daemon_token_file).map_err(|e| anyhow::anyhow!("{e}"))?,
        };
        Ok(Self {
            socket: cfg.daemon_socket.clone(),
            tcp: cfg.daemon_tcp.clone(),
            token,
        })
    }

    pub async fn call(&self, request: Request, actor: &str) -> Result<Response, ApiError> {
        let request_id = random_hex(8);
        let envelope =
            Envelope::new(request_id.clone(), self.token.clone(), request).with_actor(actor);
        let payload = encode(&envelope)?;

        if let Some(addr) = &self.tcp {
            let mut stream = TcpStream::connect(addr)
                .await
                .map_err(|e| ApiError::Upstream(format!("cannot reach rynxd at {addr}: {e}")))?;
            return exchange(&mut stream, &payload, &request_id).await;
        }

        #[cfg(unix)]
        {
            let mut stream = tokio::net::UnixStream::connect(&self.socket)
                .await
                .map_err(|e| {
                    ApiError::Upstream(format!(
                        "cannot reach rynxd at {}: {e}",
                        self.socket.display()
                    ))
                })?;
            exchange(&mut stream, &payload, &request_id).await
        }
        #[cfg(not(unix))]
        {
            let _ = &self.socket;
            Err(ApiError::Upstream(
                "unix sockets are unavailable on this platform".into(),
            ))
        }
    }
}

async fn exchange<S>(stream: &mut S, payload: &[u8], request_id: &str) -> Result<Response, ApiError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    send_frame(stream, payload).await?;
    let raw = timeout(CALL_TIMEOUT, recv_frame(stream))
        .await
        .map_err(|_| ApiError::Upstream("daemon request timed out".into()))??;
    let envelope: ServerEnvelope<Response> = decode(&raw)?;
    if envelope.protocol != PROTOCOL_VERSION {
        return Err(ApiError::Upstream(format!(
            "daemon protocol mismatch: {}",
            envelope.protocol
        )));
    }
    if envelope.request_id != request_id {
        return Err(ApiError::Upstream(
            "daemon response correlation failed".into(),
        ));
    }
    envelope.result.map_err(ApiError::from)
}
