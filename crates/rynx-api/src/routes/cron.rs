use axum::extract::{Path, State};
use axum::{Extension, Json};

use rynx_proto::{CronSpec, Request, Response as DaemonResponse};

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
        .call(Request::CronList, &auth.user.username)
        .await?;
    let response = scope::list_response(&state, &auth, response).await?;
    Ok(Json(response))
}

pub async fn upsert(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(spec): Json<CronSpec>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    auth.ensure_account(&spec.username)?;
    let response = state
        .daemon
        .call(Request::CronUpsert(spec), &auth.user.username)
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
        .call(Request::CronList, &auth.user.username)
        .await?;
    let DaemonResponse::CronJobs { jobs } = response else {
        return Err(ApiError::internal("unexpected daemon response"));
    };
    let job = jobs
        .into_iter()
        .find(|job| job.id == id)
        .ok_or_else(|| ApiError::NotFound("cron job not found".into()))?;
    auth.ensure_account(&job.username)?;
    let response = state
        .daemon
        .call(Request::CronDelete { id }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
