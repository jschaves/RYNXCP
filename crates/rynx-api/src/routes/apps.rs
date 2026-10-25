use axum::extract::{Path, State};
use axum::{Extension, Json};

use rynx_proto::{AppSpec, Request, Response as DaemonResponse};

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
        .call(Request::AppList, &auth.user.username)
        .await?;
    let response = scope::list_response(&state, &auth, response).await?;
    Ok(Json(response))
}

pub async fn create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(spec): Json<AppSpec>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    auth.ensure_account(&spec.username)?;
    let response = state
        .daemon
        .call(Request::AppUpsert(spec), &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path((username, name)): Path<(String, String)>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    auth.ensure_account(&username)?;
    let response = state
        .daemon
        .call(Request::AppDelete { username, name }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
