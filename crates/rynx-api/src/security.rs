use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

use rynx_core::ApiConfig;

use crate::error::ApiError;
use crate::state::ApiState;

const CSP: &str = "default-src 'self'; base-uri 'none'; frame-ancestors 'none'; \
form-action 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; \
script-src 'self'; connect-src 'self'; object-src 'none'";

/// Adds hardened response headers and blocks cross-origin state changes.
pub async fn headers(State(state): State<ApiState>, request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    let put = |headers: &mut HeaderMap, name: &'static str, value: &'static str| {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    };
    put(headers, "content-security-policy", CSP);
    put(headers, "x-content-type-options", "nosniff");
    put(headers, "x-frame-options", "DENY");
    put(headers, "referrer-policy", "no-referrer");
    put(
        headers,
        "permissions-policy",
        "geolocation=(), camera=(), microphone=()",
    );
    put(headers, "cross-origin-opener-policy", "same-origin");
    put(headers, "cross-origin-resource-policy", "same-origin");
    if state.cfg.secure_cookies {
        put(
            headers,
            "strict-transport-security",
            "max-age=31536000; includeSubDomains",
        );
    }
    if path.starts_with("/api/") {
        put(headers, "cache-control", "no-store");
    }
    response
}

/// Rejects state-changing requests whose Origin does not match the panel host.
pub fn check_origin(headers: &HeaderMap, cfg: &ApiConfig) -> Result<(), ApiError> {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return Ok(());
    };
    if origin == "null" {
        return Err(ApiError::Forbidden);
    }
    if let Some(public) = &cfg.public_url {
        if public.trim_end_matches('/') == origin.trim_end_matches('/') {
            return Ok(());
        }
    }
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !host.is_empty() && origin.ends_with(&format!("//{host}")) {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}
