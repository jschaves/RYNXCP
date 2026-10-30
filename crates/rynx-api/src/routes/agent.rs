use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::agent;
use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct FindingsQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct StatusBody {
    pub status: String,
}

pub async fn findings(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<FindingsQuery>,
) -> Result<Json<Value>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    let status = query.status.as_deref().filter(|status| *status != "all");
    let findings = state.db.list_agent_findings(limit, status)?;
    let stats = state.db.agent_stats()?;
    let last_run = state.db.latest_agent_run()?;
    Ok(Json(json!({
        "findings": findings,
        "stats": stats,
        "last_run": last_run,
        "enabled": state.cfg.agent.enabled,
        "mode": state.cfg.agent.mode,
    })))
}

pub async fn set_status(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(body): Json<StatusBody>,
) -> Result<Json<Value>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    if !matches!(body.status.as_str(), "open" | "acknowledged" | "ignored") {
        return Err(ApiError::BadRequest(
            "invalid finding status; use open, acknowledged or ignored".into(),
        ));
    }
    state.db.set_agent_finding_status(&id, &body.status)?;
    Ok(Json(json!({ "ok": true, "status": body.status })))
}

pub async fn run(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let summary = agent::run_once(&state).await.map_err(ApiError::internal)?;
    Ok(Json(json!({
        "run": summary.run,
        "new_findings": summary.new_findings,
        "total_findings": summary.total_findings,
        "notified": summary.notified,
    })))
}
