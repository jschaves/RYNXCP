use axum::extract::{Query, State};
use axum::{Extension, Json};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;

use rynx_proto::{FileEntry, Request, Response as DaemonResponse, WebsiteView};

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::state::ApiState;

const MAX_FILE_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub domain: String,
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub domain: String,
    pub path: String,
    #[serde(default)]
    pub recursive: bool,
}

#[derive(Debug, Deserialize)]
pub struct WriteBody {
    pub domain: String,
    pub path: String,
    /// Base64-encoded contents.
    pub data: String,
}

#[derive(Debug, Deserialize)]
pub struct MkdirBody {
    pub domain: String,
    #[serde(default)]
    pub path: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadBody {
    pub domain: String,
    #[serde(default)]
    pub path: String,
    pub name: String,
    /// Base64-encoded contents.
    pub data: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameBody {
    pub domain: String,
    pub path: String,
    pub new_name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct FileListResponse {
    pub path: String,
    pub entries: Vec<FileEntry>,
}

async fn website_for(
    state: &ApiState,
    auth: &AuthUser,
    domain: &str,
) -> Result<WebsiteView, ApiError> {
    match state
        .daemon
        .call(Request::WebsiteList, &auth.user.username)
        .await?
    {
        DaemonResponse::Websites { websites } => {
            let website = websites
                .into_iter()
                .find(|website| website.spec.domain == domain)
                .ok_or_else(|| ApiError::NotFound("website not found".into()))?;
            auth.ensure_account(&website.spec.username)?;
            Ok(website)
        }
        _ => Err(ApiError::internal("unexpected daemon response")),
    }
}

fn valid_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name.len() > 255
    {
        return Err(ApiError::BadRequest("invalid file name".into()));
    }
    Ok(())
}

fn join_path(directory: &str, name: &str) -> Result<String, ApiError> {
    valid_name(name)?;
    let directory = directory.trim_end_matches('/');
    Ok(if directory.is_empty() {
        name.to_string()
    } else {
        format!("{directory}/{name}")
    })
}

fn decode(data: &str) -> Result<Vec<u8>, ApiError> {
    if data.len() > MAX_FILE_BYTES * 2 {
        return Err(ApiError::BadRequest("file is too large".into()));
    }
    let decoded = STANDARD
        .decode(data)
        .map_err(|_| ApiError::BadRequest("invalid base64 payload".into()))?;
    if decoded.len() > MAX_FILE_BYTES {
        return Err(ApiError::BadRequest("file is too large".into()));
    }
    Ok(decoded)
}

pub async fn list(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<FileQuery>,
) -> Result<Json<FileListResponse>, ApiError> {
    let website = website_for(&state, &auth, &query.domain).await?;
    match state
        .daemon
        .call(
            Request::FileList {
                username: website.spec.username,
                domain: query.domain,
                path: query.path,
            },
            &auth.user.username,
        )
        .await?
    {
        DaemonResponse::Files { entries, path } => Ok(Json(FileListResponse { path, entries })),
        _ => Err(ApiError::internal("unexpected daemon response")),
    }
}

pub async fn read(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<FileQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if query.path.is_empty() {
        return Err(ApiError::BadRequest("a file path is required".into()));
    }
    let website = website_for(&state, &auth, &query.domain).await?;
    match state
        .daemon
        .call(
            Request::FileRead {
                username: website.spec.username,
                domain: query.domain,
                path: query.path,
            },
            &auth.user.username,
        )
        .await?
    {
        DaemonResponse::FileData { data } => Ok(Json(serde_json::json!({
            "data": STANDARD.encode(data)
        }))),
        _ => Err(ApiError::internal("unexpected daemon response")),
    }
}

pub async fn write(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<WriteBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_write()?;
    if body.path.is_empty() {
        return Err(ApiError::BadRequest("a file path is required".into()));
    }
    let data = decode(&body.data)?;
    let website = website_for(&state, &auth, &body.domain).await?;
    state
        .daemon
        .call(
            Request::FileWrite {
                username: website.spec.username,
                domain: body.domain,
                path: body.path,
                data,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn mkdir(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<MkdirBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_write()?;
    let path = join_path(&body.path, &body.name)?;
    let website = website_for(&state, &auth, &body.domain).await?;
    state
        .daemon
        .call(
            Request::FileMkdir {
                username: website.spec.username,
                domain: body.domain,
                path,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn upload(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<UploadBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_write()?;
    let path = join_path(&body.path, &body.name)?;
    let data = decode(&body.data)?;
    let website = website_for(&state, &auth, &body.domain).await?;
    state
        .daemon
        .call(
            Request::FileWrite {
                username: website.spec.username,
                domain: body.domain,
                path,
                data,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn rename(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<RenameBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_write()?;
    if body.path.is_empty() {
        return Err(ApiError::BadRequest("a source path is required".into()));
    }
    valid_name(&body.new_name)?;
    let website = website_for(&state, &auth, &body.domain).await?;
    state
        .daemon
        .call(
            Request::FileRename {
                username: website.spec.username,
                domain: body.domain,
                path: body.path,
                new_name: body.new_name,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn delete(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<DeleteQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_write()?;
    if query.path.is_empty() {
        return Err(ApiError::BadRequest(
            "refusing to delete the site root".into(),
        ));
    }
    let website = website_for(&state, &auth, &query.domain).await?;
    state
        .daemon
        .call(
            Request::FileDelete {
                username: website.spec.username,
                domain: query.domain,
                path: query.path,
                recursive: query.recursive,
            },
            &auth.user.username,
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
