use axum::extract::{Query, State};
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_proto::{Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct IssueRequest {
    pub domain: String,
    pub email: String,
    #[serde(default)]
    pub staging: bool,
}

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    let response = state
        .daemon
        .call(Request::CertList, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn issue(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<IssueRequest>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    let response = state
        .daemon
        .call(
            Request::CertIssue {
                domain: body.domain,
                email: body.email,
                staging: body.staging,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct RenewQuery {
    #[serde(default)]
    pub staging: bool,
}

pub async fn renew(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<RenewQuery>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_write()?;
    let response = state
        .daemon
        .call(
            Request::CertRenew {
                staging: query.staging,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(response))
}
