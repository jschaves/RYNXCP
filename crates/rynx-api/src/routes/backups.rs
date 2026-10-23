use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_proto::{Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct CreateBackup {
    pub username: String,
}

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    let response = state
        .daemon
        .call(Request::BackupList, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<CreateBackup>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    let response = state
        .daemon
        .call(
            Request::BackupCreate {
                username: body.username,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(response))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    let response = state
        .daemon
        .call(Request::BackupDelete { id }, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn restore(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    let response = state
        .daemon
        .call(Request::BackupRestore { id }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
