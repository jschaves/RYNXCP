use axum::extract::State;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use rynx_proto::{Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "name": "RYNXCP",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn meta() -> Json<Value> {
    Json(json!({
        "name": "RYNXCP",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": rynx_proto::PROTOCOL_VERSION,
        "runtimes": ["php", "node", "python", "ruby", "java", "dotnet", "go", "static"],
    }))
}

pub async fn info(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::SystemInfo, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn reconcile(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::ReconcileAll, &auth.user.username)
        .await?;
    Ok(Json(response))
}

#[derive(Debug, Serialize)]
pub struct EmailStatus {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub tls: String,
    pub from: Option<String>,
    pub alert_to: Option<String>,
    pub public_url: Option<String>,
}

pub async fn email_status(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<EmailStatus>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    Ok(Json(EmailStatus {
        enabled: state.mailer.is_some(),
        host: state.cfg.smtp_host.clone(),
        port: state.cfg.smtp_port,
        tls: state.cfg.smtp_tls.clone(),
        from: state.cfg.smtp_from.clone(),
        alert_to: state.cfg.smtp_alert_to.clone(),
        public_url: state.cfg.public_url.clone(),
    }))
}

#[derive(Debug, Deserialize, Default)]
pub struct EmailTestRequest {
    #[serde(default)]
    pub to: Option<String>,
}

pub async fn email_test(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<EmailTestRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let mailer = state
        .mailer
        .as_deref()
        .ok_or_else(|| ApiError::Conflict("SMTP is not configured".into()))?;
    let to = body
        .to
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| state.cfg.smtp_alert_to.clone())
        .unwrap_or_else(|| auth.user.email.clone());
    rynx_core::validate_email(&to).map_err(|err| ApiError::BadRequest(err.to_string()))?;
    let (subject, body) = crate::email::test_email_text(
        auth.user.locale.as_deref(),
        &state.cfg.smtp_host,
        state.cfg.smtp_port,
        &state.cfg.smtp_tls,
    );
    mailer
        .send(&to, &subject, &body)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true, "to": to })))
}
