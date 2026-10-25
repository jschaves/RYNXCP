use axum::extract::{Query, State};
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_proto::{Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default)]
    pub minutes: Option<u32>,
}

pub async fn services(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::ServiceStats, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn history(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_global()?;
    let minutes = query.minutes.unwrap_or(1440).clamp(1, 10_080);
    let response = state
        .daemon
        .call(Request::MetricsHistory { minutes }, &auth.user.username)
        .await?;
    Ok(Json(response))
}
