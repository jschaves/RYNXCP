use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_panel::{Role, UserView};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub role: Option<Role>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub role: Option<Role>,
    #[serde(default)]
    pub active: Option<bool>,
}

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<UserView>>, ApiError> {
    auth.require_admin()?;
    Ok(Json(state.db.list_users()?))
}

pub async fn create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<CreateUser>,
) -> Result<Json<UserView>, ApiError> {
    auth.require_admin()?;
    let role = body.role.unwrap_or(Role::Operator);
    if role == Role::Owner && !auth.is_owner() {
        return Err(ApiError::Forbidden);
    }
    let policy =
        rynx_core::PasswordPolicy::new(state.cfg.password_min_len, state.cfg.password_min_classes);
    let user = state.db.create_user_with_policy(
        &policy,
        &body.username,
        &body.email,
        &body.password,
        role,
    )?;
    Ok(Json(user))
}

pub async fn update(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(username): Path<String>,
    Json(body): Json<UpdateUser>,
) -> Result<Json<UserView>, ApiError> {
    auth.require_admin()?;
    let target = state
        .db
        .find_user(&username)?
        .ok_or_else(|| ApiError::NotFound(format!("user '{username}' not found")))?;

    if target.role == Role::Owner && !auth.is_owner() {
        return Err(ApiError::Forbidden);
    }

    if let Some(role) = body.role {
        if username == auth.user.username {
            return Err(ApiError::BadRequest("cannot change your own role".into()));
        }
        if role == Role::Owner && !auth.is_owner() {
            return Err(ApiError::Forbidden);
        }
        if target.role == Role::Owner && role != Role::Owner && state.db.count_owners()? <= 1 {
            return Err(ApiError::Conflict(
                "cannot remove the last active owner".into(),
            ));
        }
        state.db.set_role(&username, role)?;
    }

    if let Some(active) = body.active {
        if username == auth.user.username && !active {
            return Err(ApiError::BadRequest("cannot deactivate yourself".into()));
        }
        if !active && target.role == Role::Owner && state.db.count_owners()? <= 1 {
            return Err(ApiError::Conflict(
                "cannot deactivate the last active owner".into(),
            ));
        }
        state.db.set_active(&username, active)?;
    }

    if let Some(password) = body.password {
        let policy = rynx_core::PasswordPolicy::new(
            state.cfg.password_min_len,
            state.cfg.password_min_classes,
        );
        state
            .db
            .set_password_with_policy(&policy, &username, &password)?;
    }

    if let Some(email) = body.email {
        state.db.set_email(&username, &email)?;
    }

    let updated = state
        .db
        .find_user(&username)?
        .ok_or_else(|| ApiError::NotFound(format!("user '{username}' not found")))?;
    Ok(Json(updated))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_admin()?;
    if username == auth.user.username {
        return Err(ApiError::BadRequest("cannot delete yourself".into()));
    }
    let target = state
        .db
        .find_user(&username)?
        .ok_or_else(|| ApiError::NotFound(format!("user '{username}' not found")))?;
    if target.role == Role::Owner {
        if !auth.is_owner() {
            return Err(ApiError::Forbidden);
        }
        if state.db.count_owners()? <= 1 {
            return Err(ApiError::Conflict(
                "cannot delete the last active owner".into(),
            ));
        }
    }
    state.db.delete_user(&username)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
