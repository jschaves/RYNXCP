use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_proto::{FirewallRuleSpec, Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct BlockBody {
    pub ip: String,
    #[serde(default)]
    pub seconds: u32,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct IpBody {
    pub ip: String,
}

pub async fn status(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::FirewallStatus, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn upsert_rule(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(spec): Json<FirewallRuleSpec>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::FirewallRuleUpsert(spec), &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn delete_rule(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::FirewallRuleDelete { id }, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn block(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<BlockBody>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(
            Request::FirewallBlock {
                ip: body.ip,
                seconds: body.seconds,
                reason: body.reason,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(response))
}

pub async fn unblock(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(ip): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::FirewallUnblock { ip }, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn whitelist_add(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<IpBody>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(
            Request::FirewallWhitelistAdd { ip: body.ip },
            &auth.user.username,
        )
        .await?;
    Ok(Json(response))
}

pub async fn whitelist_remove(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(ip): Path<String>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::FirewallWhitelistRemove { ip }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
