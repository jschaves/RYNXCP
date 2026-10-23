use axum::extract::{Path, State};
use axum::{Extension, Json};

use rynx_proto::{Request, Response as DaemonResponse, WebsiteSpec};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    let response = state
        .daemon
        .call(Request::WebsiteList, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(spec): Json<WebsiteSpec>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    let response = state
        .daemon
        .call(Request::WebsiteUpsert(spec), &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(domain): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    let response = state
        .daemon
        .call(Request::WebsiteDelete { domain }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
