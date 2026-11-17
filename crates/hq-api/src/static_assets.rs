//! Embedded web build and SPA fallback. API, WebSocket and SSE paths are never served from here.

use axum::{
    Router,
    body::Body,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../web/build/"]
#[exclude = "**/*.map"]
struct UiAssets;

/// Paths owned by the API, WS or SSE router must never reach the SPA fallback.
pub fn is_public_asset_path(path: &str) -> bool {
    !(path.starts_with("/api") || path.starts_with("/ws") || path.starts_with("/events"))
}

fn load(path: &str) -> Option<(Vec<u8>, String)> {
    let raw = if path == "/" || path.is_empty() {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };
    let key = raw.replace('\\', "/");
    if key.split('/').any(|segment| {
        segment == "." || segment == ".." || segment.to_ascii_lowercase().contains("%2e")
    }) {
        return None;
    }

    if let Ok(directory) = std::env::var("HQ_WEB_DIR") {
        let root = std::fs::canonicalize(directory).ok()?;
        let candidate = std::fs::canonicalize(root.join(&key)).ok()?;
        if !candidate.starts_with(&root) || !candidate.is_file() {
            return None;
        }
        let mime = mime_guess::from_path(&candidate)
            .first_or_octet_stream()
            .to_string();
        return std::fs::read(&candidate).ok().map(|bytes| (bytes, mime));
    }

    UiAssets::get(&key).map(|file| {
        let mime = mime_guess::from_path(&key)
            .first_or_octet_stream()
            .to_string();
        (file.data.into_owned(), mime)
    })
}

fn response(bytes: Vec<u8>, mime: String, path: &str) -> Response {
    let cache = if path.starts_with("/_app/immutable/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, cache.to_string()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
        ],
        Body::from(bytes),
    )
        .into_response()
}

pub async fn serve_asset(uri: Uri) -> Response {
    let path = uri.path();
    if !is_public_asset_path(path) {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Some((bytes, mime)) = load(path) {
        return response(bytes, mime, path);
    }
    if path
        .rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    match load("index.html") {
        Some((bytes, mime)) => response(bytes, mime, path),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn ui_router() -> Router {
    Router::new().fallback(serve_asset)
}
