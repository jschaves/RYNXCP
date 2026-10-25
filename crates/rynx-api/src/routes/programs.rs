use axum::extract::State;
use axum::{Extension, Json};

use rynx_proto::{Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

pub async fn programs(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::Programs, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn updates(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::Updates, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn apply(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::ApplyUpdates, &auth.user.username)
        .await?;
    Ok(Json(response))
}
