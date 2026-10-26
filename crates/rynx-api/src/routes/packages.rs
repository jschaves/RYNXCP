use axum::extract::{Path, State};
use axum::{Extension, Json};

use rynx_panel::{PackageInput, PackageView};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<PackageView>>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    Ok(Json(state.db.list_packages()?))
}

pub async fn create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<PackageInput>,
) -> Result<Json<PackageView>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    Ok(Json(state.db.create_package(body)?))
}

pub async fn update(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(body): Json<PackageInput>,
) -> Result<Json<PackageView>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    Ok(Json(state.db.update_package(&id, body)?))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    state.db.delete_package(&id)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
