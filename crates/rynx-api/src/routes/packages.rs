use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_panel::{AccountPackageView, PackageInput, PackageView};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<PackageView>>, ApiError> {
    auth.require_write()?;
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

pub async fn assignments(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<AccountPackageView>>, ApiError> {
    if let Some(account) = auth.scoped_account() {
        return Ok(Json(
            state.db.get_account_package(account)?.into_iter().collect(),
        ));
    }
    auth.require_global()?;
    Ok(Json(state.db.list_account_packages()?))
}

/// Package assigned to the authenticated user's linked hosting account.
pub async fn mine(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<PackageView>, ApiError> {
    let account = auth
        .scoped_account()
        .ok_or_else(|| ApiError::NotFound("the user is not linked to an account".into()))?;
    let assignment = state
        .db
        .get_account_package(account)?
        .ok_or_else(|| ApiError::NotFound("no package is assigned to this account".into()))?;
    let package = state
        .db
        .get_package(&assignment.package_id)?
        .ok_or_else(|| ApiError::NotFound("assigned package not found".into()))?;
    Ok(Json(package))
}

#[derive(Debug, Deserialize)]
pub struct AssignmentUpdate {
    pub web_backend: String,
}

pub async fn update_assignment(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(account): Path<String>,
    Json(body): Json<AssignmentUpdate>,
) -> Result<Json<AccountPackageView>, ApiError> {
    auth.require_write()?;
    if let Some(own) = auth.scoped_account() {
        if own != account {
            return Err(ApiError::Forbidden);
        }
    } else {
        auth.require_global()?;
    }
    let current = state.db.get_account_package(&account)?.ok_or_else(|| {
        ApiError::NotFound(format!("package assignment for '{account}' not found"))
    })?;
    let updated = state
        .db
        .set_account_package(&account, &current.package_id, &body.web_backend)?;
    Ok(Json(updated))
}
