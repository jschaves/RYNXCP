use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, Method};
use axum::middleware::Next;
use axum::response::Response;

use rynx_core::crypto::constant_time_eq;
use rynx_panel::db::{CSRF_HEADER, SESSION_COOKIE};
use rynx_panel::{Role, UserView};

use crate::error::ApiError;
use crate::state::ApiState;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user: UserView,
    pub csrf_token: String,
    pub session_token: String,
    pub impersonated_by: Option<String>,
}

impl AuthUser {
    pub fn require_write(&self) -> Result<(), ApiError> {
        if self.user.role.can_write() {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn require_admin(&self) -> Result<(), ApiError> {
        if self.user.role.can_manage_users() {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn is_owner(&self) -> bool {
        self.user.role == Role::Owner
    }
}

pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            if key.trim() == name {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn is_mutating(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PATCH | Method::PUT | Method::DELETE
    )
}

/// Session + CSRF enforcement for every protected route.
pub async fn require_auth(
    State(state): State<ApiState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = cookie_value(request.headers(), SESSION_COOKIE).ok_or(ApiError::Unauthorized)?;
    let session = state
        .db
        .lookup_session(&token, state.cfg.session_idle_secs)
        .map_err(ApiError::from)?
        .ok_or(ApiError::Unauthorized)?;

    if is_mutating(request.method()) {
        let provided = request
            .headers()
            .get(CSRF_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !constant_time_eq(provided, &session.csrf_token) {
            return Err(ApiError::Forbidden);
        }
    }

    request.extensions_mut().insert(AuthUser {
        user: session.user,
        csrf_token: session.csrf_token,
        session_token: token,
        impersonated_by: session.impersonated_by,
    });
    Ok(next.run(request).await)
}
