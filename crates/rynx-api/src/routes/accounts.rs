use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use rynx_core::crypto::random_hex;
use rynx_panel::{Role, UserView};
use rynx_proto::{AccountSpec, AccountView, Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::scope;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct PurgeQuery {
    #[serde(default)]
    pub purge: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccount {
    #[serde(flatten)]
    pub spec: AccountSpec,
    /// `None` leaves the linked panel user untouched (used when toggling an
    /// account), `Some(true)` creates it when missing, `Some(false)` disables
    /// its access.
    #[serde(default)]
    pub panel_access: Option<bool>,
    /// Only `viewer` and `operator` are allowed. Defaults to `viewer`.
    #[serde(default)]
    pub panel_role: Option<Role>,
}

#[derive(Debug, Serialize)]
pub struct CreateAccountResponse {
    pub account: DaemonResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_user: Option<UserView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_password: Option<String>,
}

async fn account_list(state: &ApiState, actor: &str) -> Result<Vec<AccountView>, ApiError> {
    match state.daemon.call(Request::AccountList, actor).await? {
        DaemonResponse::Accounts { accounts } => Ok(accounts),
        _ => Ok(Vec::new()),
    }
}

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    let response = state
        .daemon
        .call(Request::AccountList, &auth.user.username)
        .await?;
    let response = scope::list_response(&state, &auth, response).await?;
    Ok(Json(response))
}

pub async fn create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<CreateAccount>,
) -> Result<Json<CreateAccountResponse>, ApiError> {
    auth.require_write()?;
    auth.require_global()?;
    let spec = body.spec;

    if let Some(role) = body.panel_role {
        if !matches!(role, Role::Viewer | Role::Operator) {
            return Err(ApiError::BadRequest(
                "panel_role must be viewer or operator".into(),
            ));
        }
    }

    let accounts = account_list(&state, &auth.user.username).await?;

    if accounts.iter().any(|account| {
        account.spec.username != spec.username
            && account.spec.email.eq_ignore_ascii_case(&spec.email)
    }) {
        return Err(ApiError::Conflict(
            "Ese email ya está en uso por otra cuenta de hosting.".into(),
        ));
    }

    if let Some(user) = state.db.find_user_by_email(&spec.email)? {
        if user.account.as_deref() != Some(spec.username.as_str()) {
            return Err(ApiError::Conflict(
                "Ese email ya está en uso por un usuario del panel. Usa otro email para la cuenta."
                    .into(),
            ));
        }
    }

    if let Some(user) = state.db.find_user(&spec.username)? {
        if user.account.as_deref() != Some(spec.username.as_str()) {
            return Err(ApiError::Conflict(
                "Ese nombre de usuario ya está en uso por un usuario del panel.".into(),
            ));
        }
    }

    let response = state
        .daemon
        .call(Request::AccountCreate(spec.clone()), &auth.user.username)
        .await?;

    let mut panel_user = state.db.find_user_by_account(&spec.username)?;
    let mut panel_password = None;

    if let Some(user) = panel_user.as_mut() {
        if !user.email.eq_ignore_ascii_case(&spec.email) {
            state.db.set_email(&user.username, &spec.email)?;
            user.email = spec.email.clone();
        }
        let desired_active = match body.panel_access {
            Some(false) => false,
            _ => spec.enabled,
        };
        if user.active != desired_active {
            state.db.set_active(&user.username, desired_active)?;
            user.active = desired_active;
        }
    } else if body.panel_access == Some(true) {
        let policy = rynx_core::PasswordPolicy::new(
            state.cfg.password_min_len,
            state.cfg.password_min_classes,
        );
        let role = body.panel_role.unwrap_or(Role::Viewer);
        // Generated password: 36 chars, four character classes.
        let password = format!("Aa1!{}", random_hex(16));
        let mut user = state.db.create_account_user(
            &policy,
            &spec.username,
            &spec.email,
            &password,
            role,
            &spec.username,
        )?;
        if !spec.enabled {
            state.db.set_active(&user.username, false)?;
            user.active = false;
        }
        panel_user = Some(user);
        panel_password = Some(password);
    }

    Ok(Json(CreateAccountResponse {
        account: response,
        panel_user,
        panel_password,
    }))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(username): Path<String>,
    Query(query): Query<PurgeQuery>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(
            Request::AccountDelete {
                username: username.clone(),
                purge: query.purge,
            },
            &auth.user.username,
        )
        .await?;
    if let Some(user) = state.db.find_user_by_account(&username)? {
        let _ = state.db.delete_user(&user.username);
    }
    Ok(Json(response))
}
