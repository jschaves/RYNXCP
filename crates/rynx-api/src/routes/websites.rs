use axum::extract::{Path, State};
use axum::{Extension, Json};

use rynx_proto::{Request, Response as DaemonResponse, TlsMode, WebsiteSpec};

use crate::auth::AuthUser;
use crate::cert_jobs;
use crate::error::ApiError;
use crate::scope;
use crate::state::ApiState;

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    let response = state
        .daemon
        .call(Request::WebsiteList, &auth.user.username)
        .await?;
    let response = scope::list_response(&state, &auth, response).await?;
    Ok(Json(response))
}

pub async fn create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(spec): Json<WebsiteSpec>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_write()?;
    scope::ensure_website_upsert(&state, &auth, &spec.domain, &spec.username).await?;
    crate::limits::check_website(&state, &auth.user.username, &spec).await?;
    let wants_cert = spec.enabled && spec.tls == TlsMode::LetsEncrypt;
    let domain = spec.domain.clone();
    let response = state
        .daemon
        .call(Request::WebsiteUpsert(spec), &auth.user.username)
        .await?;
    let mut body = serde_json::to_value(&response).map_err(ApiError::internal)?;
    if wants_cert {
        let job = cert_jobs::start(
            &state,
            &domain,
            &auth.user.email,
            false,
            &auth.user.username,
        )
        .await;
        if let Some(object) = body.as_object_mut() {
            object.insert(
                "cert_job".into(),
                serde_json::to_value(job).map_err(ApiError::internal)?,
            );
        }
    }
    Ok(Json(body))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(domain): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    scope::ensure_domain(&state, &auth, &domain).await?;
    let response = state
        .daemon
        .call(Request::WebsiteDelete { domain }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
