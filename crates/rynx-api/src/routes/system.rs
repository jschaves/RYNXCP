use axum::extract::State;
use axum::{Extension, Json};
use serde_json::{json, Value};

use rynx_proto::{Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "name": "RYNXCP",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn meta() -> Json<Value> {
    Json(json!({
        "name": "RYNXCP",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": rynx_proto::PROTOCOL_VERSION,
        "runtimes": ["php", "node", "python", "ruby", "java", "dotnet", "go", "static"],
    }))
}

pub async fn info(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::SystemInfo, &auth.user.username)
        .await?;
    Ok(Json(response))
}

pub async fn reconcile(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(Request::ReconcileAll, &auth.user.username)
        .await?;
    Ok(Json(response))
}
