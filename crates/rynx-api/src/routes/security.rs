use axum::extract::{Query, State};
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    #[serde(default)]
    pub hours: Option<i64>,
}

pub async fn events(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Value>, ApiError> {
    auth.require_admin()?;
    let events = state.db.list_auth_events(query.limit.unwrap_or(200))?;
    Ok(Json(json!({ "events": events })))
}

pub async fn stats(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<Value>, ApiError> {
    auth.require_admin()?;
    let hours = query.hours.unwrap_or(24).clamp(1, 24 * 90);
    let stats = state.db.auth_stats(hours * 3600)?;
    Ok(Json(json!(stats)))
}
