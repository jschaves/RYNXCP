use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde_json::Value;

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::installer::{self, InstallRequest, InstallerJobView};
use crate::state::ApiState;

pub async fn catalog(Extension(auth): Extension<AuthUser>) -> Result<Json<Value>, ApiError> {
    auth.require_write()?;
    Ok(Json(installer::catalog()))
}

pub async fn install(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(request): Json<InstallRequest>,
) -> Result<Json<InstallerJobView>, ApiError> {
    auth.require_write()?;
    let job = installer::start(&state, request, &auth).await?;
    Ok(Json(job))
}

pub async fn job_status(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<InstallerJobView>, ApiError> {
    auth.require_write()?;
    let job = installer::get(&state, &id, &auth.user.username)
        .await
        .ok_or_else(|| ApiError::NotFound("installer job not found".into()))?;
    Ok(Json(job))
}
