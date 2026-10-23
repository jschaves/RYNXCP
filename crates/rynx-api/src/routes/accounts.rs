use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_proto::{AccountSpec, Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct PurgeQuery {
    #[serde(default)]
    pub purge: bool,
}

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    let response = state
        .daemon
        .call(Request::AccountList, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(spec): Json<AccountSpec>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    let response = state
        .daemon
        .call(Request::AccountCreate(spec), &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(username): Path<String>,
    Query(query): Query<PurgeQuery>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    let response = state
        .daemon
        .call(
            Request::AccountDelete {
                username,
                purge: query.purge,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(response))
}
