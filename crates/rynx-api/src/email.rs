//! Minimal SMTP client for panel notifications.
//!
//! The production default talks to Postfix on loopback without authentication
//! or TLS. Messages are UTF-8 and base64 encoded so headers stay ASCII-safe.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rynx_core::crypto::random_hex;
use rynx_core::{validate_email, ApiConfig};
use time::format_description::well_known::Rfc2822;
use time::OffsetDateTime;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

use crate::state::ApiState;

const SMTP_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone)]
pub struct Mailer {
    host: String,
    port: u16,
    from: String,
    helo: String,
}

impl Mailer {
    pub fn from_config(cfg: &ApiConfig) -> Result<Option<Self>> {
        if !cfg.smtp_enabled {
            return Ok(None);
        }
        if cfg.smtp_host.trim().is_empty() {
            bail!("smtp_host is required when smtp_enabled is true");
        }
        if cfg.smtp_port == 0 {
            bail!("smtp_port must be between 1 and 65535");
        }
        if cfg.smtp_tls != "none" {
            bail!("smtp_tls only supports \"none\" for the local Postfix transport");
        }
        let from = cfg
            .smtp_from
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("smtp_from is required when smtp_enabled is true"))?;
        validate_email(from).map_err(|err| anyhow!("invalid smtp_from: {err}"))?;
        let helo = from
            .rsplit_once('@')
            .map(|(_, domain)| domain.to_ascii_lowercase())
            .filter(|domain| !domain.is_empty())
            .ok_or_else(|| anyhow!("smtp_from must include a domain"))?;
        Ok(Some(Self {
            host: cfg.smtp_host.trim().to_string(),
            port: cfg.smtp_port,
            from: from.to_string(),
            helo,
        }))
    }

    pub async fn send(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        validate_email(to).map_err(|err| anyhow!("invalid recipient: {err}"))?;
        let message = self.render(to, subject, body);
        tokio::time::timeout(SMTP_TIMEOUT, self.deliver(to, &message))
            .await
            .map_err(|_| anyhow!("SMTP timed out"))??;
        Ok(())
    }

    async fn deliver(&self, to: &str, message: &str) -> Result<()> {
        let stream = TcpStream::connect((self.host.as_str(), self.port))
            .await
            .with_context(|| format!("cannot connect to SMTP {}:{}", self.host, self.port))?;
        let (read, mut write) = stream.into_split();
        let mut read = BufReader::new(read);

        expect(read_reply(&mut read).await?, &[220], "greeting")?;
        send_line(&mut write, &format!("EHLO {}", self.helo)).await?;
        expect(read_reply(&mut read).await?, &[250], "EHLO")?;
        send_line(&mut write, &format!("MAIL FROM:<{}>", self.from)).await?;
        expect(read_reply(&mut read).await?, &[250], "MAIL FROM")?;
        send_line(&mut write, &format!("RCPT TO:<{to}>")).await?;
        expect(read_reply(&mut read).await?, &[250, 251], "RCPT TO")?;
        send_line(&mut write, "DATA").await?;
        expect(read_reply(&mut read).await?, &[354], "DATA")?;
        write.write_all(message.as_bytes()).await?;
        write.write_all(b".\r\n").await?;
        write.flush().await?;
        expect(read_reply(&mut read).await?, &[250], "message")?;
        let _ = send_line(&mut write, "QUIT").await;
        let _ = read_reply(&mut read).await;
        Ok(())
    }

    fn render(&self, to: &str, subject: &str, body: &str) -> String {
        let date = OffsetDateTime::now_utc()
            .format(&Rfc2822)
            .unwrap_or_else(|_| "Thu, 01 Jan 1970 00:00:00 +0000".into());
        let message_id = format!("<{}@{}>", random_hex(16), self.helo);
        let encoded_body = wrap_base64(&STANDARD.encode(body.as_bytes()));
        format!(
            "Date: {date}\r\n\
             From: RYNXCP <{from}>\r\n\
             To: <{to}>\r\n\
             Subject: {subject}\r\n\
             Message-ID: {message_id}\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\
             Content-Transfer-Encoding: base64\r\n\
             Auto-Submitted: auto-generated\r\n\
             \r\n\
             {encoded_body}",
            from = self.from,
            subject = encode_header(subject),
        )
    }
}

async fn send_line(write: &mut OwnedWriteHalf, line: &str) -> Result<()> {
    write.write_all(line.as_bytes()).await?;
    write.write_all(b"\r\n").await?;
    write.flush().await?;
    Ok(())
}

async fn read_reply(read: &mut BufReader<OwnedReadHalf>) -> Result<(u16, String)> {
    let mut code: Option<u16> = None;
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        let bytes = read.read_line(&mut line).await?;
        if bytes == 0 {
            bail!("SMTP connection closed unexpectedly");
        }
        if line.len() > 4096 {
            bail!("SMTP reply is too long");
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.len() < 3 {
            bail!("invalid SMTP reply");
        }
        let parsed: u16 = line
            .get(..3)
            .ok_or_else(|| anyhow!("invalid SMTP reply code"))?
            .parse()
            .map_err(|_| anyhow!("invalid SMTP reply code"))?;
        match code {
            Some(expected) if expected != parsed => bail!("inconsistent SMTP reply code"),
            Some(_) => {}
            None => code = Some(parsed),
        }
        lines.push(line.get(3..).unwrap_or_default().trim().to_string());
        if !line.as_bytes().get(3).is_some_and(|byte| *byte == b'-') {
            break;
        }
    }
    Ok((code.unwrap_or(0), lines.join(" ")))
}

fn expect(reply: (u16, String), expected: &[u16], step: &str) -> Result<()> {
    if expected.contains(&reply.0) {
        Ok(())
    } else {
        bail!("SMTP {step} failed with code {}: {}", reply.0, reply.1)
    }
}

fn encode_header(value: &str) -> String {
    let clean: String = value
        .chars()
        .take(200)
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    let clean = clean.trim();
    if clean.is_ascii() {
        clean.to_string()
    } else {
        format!("=?UTF-8?B?{}?=", STANDARD.encode(clean.as_bytes()))
    }
}

fn wrap_base64(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + value.len() / 76 * 2 + 2);
    for chunk in value.as_bytes().chunks(76) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        out.push_str("\r\n");
    }
    out
}

pub fn base_url(cfg: &ApiConfig) -> String {
    cfg.public_url
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:8080".into())
        .trim_end_matches('/')
        .to_string()
}

pub async fn send_credentials(
    state: &ApiState,
    to: &str,
    username: &str,
    password: &str,
) -> Result<()> {
    let Some(mailer) = state.mailer.as_deref() else {
        return Ok(());
    };
    let body = format!(
        "Se ha creado tu acceso al panel de control RYNXCP.\n\n\
         Panel: {url}\n\
         Usuario: {username}\n\
         Contraseña: {password}\n\n\
         Cambia la contraseña después del primer inicio de sesión.\n",
        url = base_url(&state.cfg),
    );
    mailer.send(to, "Tu acceso al panel RYNXCP", &body).await
}

pub async fn send_password_reset(
    state: &ApiState,
    to: &str,
    username: &str,
    token: &str,
) -> Result<()> {
    let Some(mailer) = state.mailer.as_deref() else {
        return Ok(());
    };
    let link = format!("{}/#reset_token={token}", base_url(&state.cfg));
    let body = format!(
        "Se solicitó restablecer la contraseña de la cuenta {username}.\n\n\
         Abre este enlace para elegir una nueva contraseña:\n\n{link}\n\n\
         El enlace caduca en 60 minutos y solo puede usarse una vez.\n\
         Si no solicitaste el cambio, ignora este mensaje.\n",
    );
    mailer
        .send(to, "Restablecer contraseña de RYNXCP", &body)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ApiConfig {
        ApiConfig {
            smtp_enabled: true,
            smtp_from: Some("noreply@example.com".into()),
            ..ApiConfig::default()
        }
    }

    #[test]
    fn disabled_mailer_is_none() {
        let cfg = ApiConfig::default();
        assert!(Mailer::from_config(&cfg).unwrap().is_none());
    }

    #[test]
    fn validates_sender_domain() {
        let mut cfg = config();
        cfg.smtp_from = Some("noreply".into());
        assert!(Mailer::from_config(&cfg).is_err());
    }

    #[test]
    fn encodes_non_ascii_subjects() {
        let encoded = encode_header("Aviso de contraseña\r\nBcc: evil@example.com");
        assert!(!encoded.contains('\r'));
        assert!(!encoded.contains('\n'));
        assert!(encoded.starts_with("=?UTF-8?B?"));
    }

    #[test]
    fn renders_safe_headers_and_base64_body() {
        let mailer = Mailer::from_config(&config()).unwrap().unwrap();
        let message = mailer.render("admin@example.com", "Hola", "línea 1\nlínea 2");
        assert!(message.contains("To: <admin@example.com>\r\n"));
        assert!(message.contains("Content-Transfer-Encoding: base64\r\n"));
        assert!(message.ends_with("\r\n"));
        assert!(!message.contains("línea 1"));
    }

    #[tokio::test]
    async fn delivers_to_a_local_smtp_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read, mut write) = stream.into_split();
            let mut read = BufReader::new(read);
            write.write_all(b"220 test\r\n").await.unwrap();
            let mut transcript = String::new();
            let mut in_data = false;
            loop {
                let mut line = String::new();
                if read.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                transcript.push_str(&line);
                let upper = line.trim_end().to_ascii_uppercase();
                if in_data {
                    if upper == "." {
                        in_data = false;
                        write.write_all(b"250 queued\r\n").await.unwrap();
                    }
                    continue;
                }
                let response = if upper.starts_with("EHLO") {
                    "250-test\r\n250 OK\r\n"
                } else if upper == "DATA" {
                    in_data = true;
                    "354 go\r\n"
                } else if upper.starts_with("QUIT") {
                    "221 bye\r\n"
                } else {
                    "250 OK\r\n"
                };
                write.write_all(response.as_bytes()).await.unwrap();
                if upper.starts_with("QUIT") {
                    break;
                }
            }
            let _ = tx.send(transcript);
        });

        let mailer = Mailer {
            host: "127.0.0.1".into(),
            port,
            from: "noreply@example.com".into(),
            helo: "example.com".into(),
        };
        mailer
            .send("admin@example.com", "Prueba", "hola")
            .await
            .unwrap();
        let transcript = rx.recv().await.unwrap();
        assert!(transcript.contains("EHLO example.com"));
        assert!(transcript.contains("MAIL FROM:<noreply@example.com>"));
        assert!(transcript.contains("RCPT TO:<admin@example.com>"));
        assert!(transcript.contains("Content-Type: text/plain"));
    }
}
