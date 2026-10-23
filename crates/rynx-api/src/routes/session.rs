use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use rynx_panel::db::SESSION_COOKIE;
use rynx_panel::UserView;

use crate::auth::AuthUser;
use crate::authlog;
use crate::error::ApiError;
use crate::security::check_origin;
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub user: UserView,
    pub csrf_token: String,
    pub expires_at: i64,
    pub impersonated_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: UserView,
    pub csrf_token: String,
    pub impersonated_by: Option<String>,
}

pub async fn login(
    State(state): State<ApiState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    check_origin(&headers, &state.cfg)?;

    if body.username.is_empty()
        || body.username.len() > 64
        || body.password.is_empty()
        || body.password.len() > 256
    {
        return Err(ApiError::BadRequest("invalid credentials".into()));
    }

    let ip = client_ip(&headers, Some(&connect), state.cfg.trust_proxy_headers);
    let identity = format!("u:{}|{}", body.username, ip);
    let ip_key = format!("ip:{ip}");
    let keys = [identity.as_str(), ip_key.as_str()];

    if state.limiter.retry_after(&keys).is_some() {
        let _ = state.db.record_auth_event(
            &body.username,
            &ip,
            &user_agent(&headers),
            "blocked",
            "rate_limited",
        );
        authlog::log(
            &state.cfg.auth_log,
            "login_blocked",
            &body.username,
            &ip,
            &user_agent(&headers),
            None,
        );
        let daemon = state.daemon.clone();
        let ban_ip = ip.clone();
        tokio::spawn(async move {
            let _ = daemon
                .call(
                    rynx_proto::Request::FirewallBlock {
                        ip: ban_ip,
                        seconds: 3600,
                        reason: "brute-force".into(),
                    },
                    "security",
                )
                .await;
        });
        return Err(ApiError::RateLimited);
    }

    let user = match state.db.authenticate(&body.username, &body.password)? {
        Some(user) => user,
        None => {
            state.limiter.record_failure(&keys);
            let _ = state.db.record_auth_event(
                &body.username,
                &ip,
                &user_agent(&headers),
                "failure",
                "invalid_credentials",
            );
            authlog::log(
                &state.cfg.auth_log,
                "login_failed",
                &body.username,
                &ip,
                &user_agent(&headers),
                None,
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
            return Err(ApiError::Unauthorized);
        }
    };
    state.limiter.reset(&identity);
    let _ = state
        .db
        .record_auth_event(&user.username, &ip, &user_agent(&headers), "success", "ok");
    authlog::log(
        &state.cfg.auth_log,
        "login_success",
        &user.username,
        &ip,
        &user_agent(&headers),
        None,
    );

    finish_login(&state, user, &ip, &headers, None)
}

/// Admin-only impersonation. Creates a dedicated session for the target user
/// and records the original actor, so it can be stopped safely.
pub async fn impersonate(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    connect: ConnectInfo<SocketAddr>,
    Path(username): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    auth.require_admin()?;
    check_origin(&headers, &state.cfg)?;
    if username == auth.user.username {
        return Err(ApiError::BadRequest(
            "you are already logged in as this user".into(),
        ));
    }
    let target = state
        .db
        .find_user(&username)?
        .ok_or_else(|| ApiError::NotFound(format!("user '{username}' not found")))?;
    if !target.active {
        return Err(ApiError::Conflict("target user is disabled".into()));
    }
    if target.role == rynx_panel::Role::Owner && !auth.is_owner() {
        return Err(ApiError::Forbidden);
    }

    let ip = client_ip(&headers, Some(&connect), state.cfg.trust_proxy_headers);
    let _ = state.db.record_auth_event(
        &target.username,
        &ip,
        &user_agent(&headers),
        "impersonation",
        &format!("by:{}", auth.user.username),
    );
    authlog::log(
        &state.cfg.auth_log,
        "impersonation_start",
        &target.username,
        &ip,
        &user_agent(&headers),
        Some(&auth.user.username),
    );
    finish_login(&state, target, &ip, &headers, Some(&auth.user.username))
}

pub async fn stop_impersonation(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Some(actor_name) = auth.impersonated_by.clone() else {
        return Err(ApiError::BadRequest("not impersonating anyone".into()));
    };
    let actor = state
        .db
        .find_user(&actor_name)?
        .ok_or(ApiError::Forbidden)?;
    if !actor.active || !actor.role.can_manage_users() {
        return Err(ApiError::Forbidden);
    }
    let _ = state.db.delete_session(&auth.session_token);
    let ip = client_ip(&headers, Some(&connect), state.cfg.trust_proxy_headers);
    let _ = state.db.record_auth_event(
        &actor.username,
        &ip,
        &user_agent(&headers),
        "success",
        "stop_impersonation",
    );
    authlog::log(
        &state.cfg.auth_log,
        "impersonation_stop",
        &actor.username,
        &ip,
        &user_agent(&headers),
        Some(&auth.user.username),
    );
    finish_login(&state, actor, &ip, &headers, None)
}

fn finish_login(
    state: &ApiState,
    user: UserView,
    ip: &str,
    headers: &HeaderMap,
    impersonated_by: Option<&str>,
) -> Result<Response, ApiError> {
    let ttl = state.cfg.session_ttl_secs;
    let session =
        state
            .db
            .create_session(&user.id, ttl, ip, &user_agent(headers), impersonated_by)?;
    let cookie = session_cookie(&session.token, ttl, state.cfg.secure_cookies);
    let mut response = Json(SessionResponse {
        user,
        csrf_token: session.csrf_token,
        expires_at: session.expires_at,
        impersonated_by: impersonated_by.map(str::to_string),
    })
    .into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(ApiError::internal)?,
    );
    Ok(response)
}

pub async fn logout(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Response, ApiError> {
    state.db.delete_session(&auth.session_token)?;
    let mut response = Json(json!({ "ok": true })).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&clear_cookie(state.cfg.secure_cookies))
            .map_err(ApiError::internal)?,
    );
    Ok(response)
}

pub async fn me(Extension(auth): Extension<AuthUser>) -> Json<MeResponse> {
    Json(MeResponse {
        user: auth.user,
        csrf_token: auth.csrf_token,
        impersonated_by: auth.impersonated_by,
    })
}

fn user_agent(headers: &HeaderMap) -> String {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .chars()
        .take(200)
        .collect()
}

fn session_cookie(token: &str, ttl_secs: i64, secure: bool) -> String {
    let mut cookie = format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        ttl_secs.max(60)
    );
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

fn clear_cookie(secure: bool) -> String {
    let mut cookie = format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

fn client_ip(
    headers: &HeaderMap,
    connect: Option<&ConnectInfo<SocketAddr>>,
    trust_proxy: bool,
) -> String {
    if trust_proxy {
        if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(first) = value.split(',').next() {
                let first = first.trim();
                if !first.is_empty() {
                    return first.to_string();
                }
            }
        }
    }
    connect
        .map(|info| info.0.ip().to_string())
        .unwrap_or_else(|| "unknown".into())
}
