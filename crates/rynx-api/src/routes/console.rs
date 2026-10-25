use axum::extract::State;
use axum::{Extension, Json};
use serde::Deserialize;

use rynx_proto::{ConsoleAction, Request, Response as DaemonResponse};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct ConsoleRequest {
    pub action: ConsoleAction,
}

pub async fn run(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<ConsoleRequest>,
) -> Result<Json<DaemonResponse>, ApiError> {
    auth.require_admin()?;
    auth.require_global()?;
    let response = state
        .daemon
        .call(
            Request::ConsoleRun {
                action: body.action,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(response))
}
