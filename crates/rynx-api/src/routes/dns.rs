use axum::extract::{Path, State};
use axum::{Extension, Json};

use rynx_proto::{DnsRecordSpec, Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::scope;
use crate::state::ApiState;

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    let response = state
        .daemon
        .call(Request::DnsZoneList, &auth.user.username)
        .await?;
    let response = scope::list_response(&state, &auth, response).await?;
    Ok(Json(response))
}

pub async fn zone(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(domain): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    scope::ensure_domain(&state, &auth, &domain).await?;
    let response = state
        .daemon
        .call(Request::DnsZone { domain }, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn upsert_record(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(domain): Path<String>,
    Json(mut spec): Json<DnsRecordSpec>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    scope::ensure_domain(&state, &auth, &domain).await?;
    spec.domain = domain;
    let response = state
        .daemon
        .call(Request::DnsRecordUpsert(spec), &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn delete_record(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path((domain, id)): Path<(String, String)>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    scope::ensure_domain(&state, &auth, &domain).await?;
    let response = state
        .daemon
        .call(Request::DnsRecordDelete { domain, id }, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn provision_mail(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(domain): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    scope::ensure_domain(&state, &auth, &domain).await?;
    let response = state
        .daemon
        .call(Request::DnsMailProvision { domain }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
