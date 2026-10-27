use axum::extract::{Path, State};
use axum::{Extension, Json};

use rynx_proto::{FtpSpec, Request, Response as DaemonResponse};

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
        .call(Request::FtpList, &auth.user.username)
        .await?;
    let response = scope::list_response(&state, &auth, response).await?;
    Ok(Json(response))
}

pub async fn create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(spec): Json<FtpSpec>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    scope::ensure_domain(&state, &auth, &spec.domain).await?;
    crate::limits::check_ftp(&state, &auth.user.username, &spec.domain).await?;
    let response = state
        .daemon
        .call(Request::FtpCreate(spec), &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    scope::ensure_ftp(&state, &auth, &id).await?;
    let response = state
        .daemon
        .call(Request::FtpDelete { id }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
