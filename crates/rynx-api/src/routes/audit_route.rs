use axum::extract::{Query, State};
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_proto::{Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default)]
    pub limit: Option<u32>,
}

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<DaemonResponse>, ApiError> {
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    auth.require_admin()?;
    let response = state
        .daemon
        .call(Request::AuditTail { limit }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
