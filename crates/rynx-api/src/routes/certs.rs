use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_proto::{Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::cert_jobs::{self, CertJobView};
use crate::error::ApiError;
use crate::scope;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct IssueRequest {
    pub domain: String,
    pub email: String,
    #[serde(default)]
    pub staging: bool,
}

#[derive(Debug, Deserialize)]
pub struct JobRequest {
    pub domain: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub staging: bool,
}

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    let response = state
        .daemon
        .call(Request::CertList, &auth.user.username)
        .await?;
    let response = scope::list_response(&state, &auth, response).await?;
    Ok(Json(response))
}

pub async fn issue(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<IssueRequest>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    scope::ensure_domain(&state, &auth, &body.domain).await?;
    let response = state
        .daemon
        .call(
            Request::CertIssue {
                domain: body.domain,
                email: body.email,
                staging: body.staging,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(response))
}

pub async fn start_job(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<JobRequest>,
) -> Result<Json<CertJobView>, ApiError> {
    auth.require_write()?;
    let domain = body.domain.trim().to_string();
    rynx_core::validate_domain(&domain).map_err(|err| ApiError::BadRequest(err.to_string()))?;
    scope::ensure_domain(&state, &auth, &domain).await?;
    let email = if body.email.trim().is_empty() {
        auth.user.email.clone()
    } else {
        body.email.trim().to_string()
    };
    rynx_core::validate_email(&email).map_err(|err| ApiError::BadRequest(err.to_string()))?;
    let job = cert_jobs::start(&state, &domain, &email, body.staging, &auth.user.username).await;
    Ok(Json(job))
}

pub async fn job_status(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<CertJobView>, ApiError> {
    let job = cert_jobs::get(&state, &id, &auth.user.username)
        .await
        .ok_or_else(|| ApiError::NotFound("certificate job not found".into()))?;
    Ok(Json(job))
}

#[derive(Debug, Deserialize)]
pub struct RenewQuery {
    #[serde(default)]
    pub staging: bool,
}

pub async fn renew(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<RenewQuery>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(
            Request::CertRenew {
                staging: query.staging,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(response))
}
