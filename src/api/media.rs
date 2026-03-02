use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;

use crate::cache;
use crate::error::AppError;
use crate::processing;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/thumbnail", get(thumbnail))
        .route("/api/thumbnail/cid", get(thumbnail_by_cid))
        .route("/api/preview", get(preview))
        .route("/api/preview/cid", get(preview_by_cid))
        .route("/api/stream", get(stream))
}

#[derive(Deserialize)]
struct ThumbnailParams {
    device: String,
    dir: String,
    path: String,
    #[serde(default = "default_thumb_size")]
    w: u32,
    #[serde(default = "default_thumb_size")]
    h: u32,
}

fn default_thumb_size() -> u32 {
    300
}

#[derive(Deserialize)]
struct MediaParams {
    device: String,
    dir: String,
    path: String,
}

async fn resolve_device_base(
    state: &AppState,
    device_id: &str,
) -> Result<(String, String), AppError> {
    let salita = state.salita();
    let devices = salita
        .list_devices()
        .await
        .map_err(|e| AppError::Internal(format!("salita error: {e}")))?;

    let device = devices
        .iter()
        .find(|d| d.id == device_id || d.name == device_id)
        .ok_or_else(|| AppError::NotFound(format!("Device not found: {device_id}")))?;

    Ok((device.id.clone(), salita.device_url(device)))
}

fn is_raw(path: &str) -> bool {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    matches!(
        ext.as_str(),
        "cr2" | "cr3" | "nef" | "arw" | "orf" | "rw2" | "dng" | "raf" | "pef" | "srw" | "x3f"
            | "3fr" | "mrw" | "nrw" | "raw"
    )
}

async fn thumbnail(
    State(state): State<AppState>,
    Query(params): Query<ThumbnailParams>,
) -> Result<Response, AppError> {
    let (device_id, base) = resolve_device_base(&state, &params.device).await?;

    // Check cache first
    let cache_path = cache::mesh_cache_path(&state, &device_id, &params.dir, &params.path, params.w, params.h);
    if cache_path.exists() {
        let bytes = tokio::fs::read(&cache_path).await?;
        return Ok(jpeg_response(bytes));
    }

    // Fetch bytes from salita
    let raw_bytes = state
        .salita()
        .fetch_file_bytes(&base, &params.dir, &params.path)
        .await
        .map_err(|e| AppError::Internal(format!("salita fetch error: {e}")))?;

    let w = params.w;
    let h = params.h;
    let path_clone = params.path.clone();

    let jpeg_bytes = tokio::task::spawn_blocking(move || {
        let img = if is_raw(&path_clone) {
            decode_raw_from_bytes(&raw_bytes, w, h)?
        } else {
            decode_image_from_bytes(&raw_bytes, w, h)?
        };
        processing::encode_jpeg(&img, 80)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task join error: {e}")))??;

    // Cache the result
    cache::ensure_mesh_cache_dir(&state, &device_id).await?;
    tokio::fs::write(&cache_path, &jpeg_bytes).await?;

    Ok(jpeg_response(jpeg_bytes))
}

// --- CID-based thumbnail (fetches pre-generated thumbnail from salita) ---

#[derive(Deserialize)]
struct CidThumbnailParams {
    cid: String,
    #[serde(default = "default_thumb_size")]
    w: u32,
    #[serde(default = "default_thumb_size")]
    h: u32,
}

async fn thumbnail_by_cid(
    State(state): State<AppState>,
    Query(params): Query<CidThumbnailParams>,
) -> Result<Response, AppError> {
    // Check local cache first
    let cache_path = cache::cid_cache_path(&state, &params.cid, params.w, params.h);
    if cache_path.exists() {
        let bytes = tokio::fs::read(&cache_path).await?;
        return Ok(jpeg_response(bytes));
    }

    // Fetch from salita's pre-generated thumbnail
    let salita = state.salita();
    let base = salita.base_url();
    let bytes = salita
        .fetch_thumbnail_by_cid(&base, &params.cid, params.w, params.h)
        .await
        .map_err(|e| AppError::Internal(format!("thumbnail fetch error: {e}")))?;

    // Cache locally
    cache::ensure_cid_cache_dir(&state, &params.cid).await?;
    tokio::fs::write(&cache_path, &bytes).await?;

    Ok(jpeg_response(bytes.to_vec()))
}

// --- CID-based preview (1600px mid-res, cached locally) ---

#[derive(Deserialize)]
struct CidPreviewParams {
    cid: String,
}

async fn preview_by_cid(
    State(state): State<AppState>,
    Query(params): Query<CidPreviewParams>,
) -> Result<Response, AppError> {
    // Check local cache
    let cache_path = cache::cid_cache_path(&state, &params.cid, 1600, 1600);
    if cache_path.exists() {
        let bytes = tokio::fs::read(&cache_path).await?;
        return Ok(jpeg_response(bytes));
    }

    // Fetch from salita (generates on-demand if not cached there either)
    let salita = state.salita();
    let base = salita.base_url();
    let bytes = salita
        .fetch_preview_by_cid(&base, &params.cid)
        .await
        .map_err(|e| AppError::Internal(format!("preview fetch error: {e}")))?;

    // Cache locally
    cache::ensure_cid_cache_dir(&state, &params.cid).await?;
    tokio::fs::write(&cache_path, &bytes).await?;

    Ok(jpeg_response(bytes.to_vec()))
}

async fn preview(
    State(state): State<AppState>,
    Query(params): Query<MediaParams>,
) -> Result<Response, AppError> {
    let (device_id, base) = resolve_device_base(&state, &params.device).await?;

    let max_dim: u32 = 2048;
    let cache_path =
        cache::mesh_cache_path(&state, &device_id, &params.dir, &params.path, max_dim, max_dim);

    if cache_path.exists() {
        let bytes = tokio::fs::read(&cache_path).await?;
        return Ok(jpeg_response(bytes));
    }

    let raw_bytes = state
        .salita()
        .fetch_file_bytes(&base, &params.dir, &params.path)
        .await
        .map_err(|e| AppError::Internal(format!("salita fetch error: {e}")))?;

    let path_clone = params.path.clone();

    // For regular images (JPG etc), just serve the original bytes if not RAW
    if !is_raw(&path_clone) {
        // Still cache a resized version for consistency
        let jpeg_bytes = tokio::task::spawn_blocking(move || {
            decode_image_from_bytes(&raw_bytes, max_dim, max_dim)
                .and_then(|img| processing::encode_jpeg(&img, 90))
        })
        .await
        .map_err(|e| AppError::Internal(format!("Task join error: {e}")))??;

        cache::ensure_mesh_cache_dir(&state, &device_id).await?;
        tokio::fs::write(&cache_path, &jpeg_bytes).await?;
        return Ok(jpeg_response(jpeg_bytes));
    }

    let jpeg_bytes = tokio::task::spawn_blocking(move || {
        let img = decode_raw_from_bytes(&raw_bytes, max_dim, max_dim)?;
        processing::encode_jpeg(&img, 90)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task join error: {e}")))??;

    cache::ensure_mesh_cache_dir(&state, &device_id).await?;
    tokio::fs::write(&cache_path, &jpeg_bytes).await?;

    Ok(jpeg_response(jpeg_bytes))
}

async fn stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<MediaParams>,
) -> Result<Response, AppError> {
    let (_device_id, base) = resolve_device_base(&state, &params.device).await?;

    let raw_bytes = state
        .salita()
        .fetch_file_bytes(&base, &params.dir, &params.path)
        .await
        .map_err(|e| AppError::Internal(format!("salita fetch error: {e}")))?;

    let total = raw_bytes.len();
    let content_type = mime_guess::from_path(&params.path)
        .first_or_octet_stream()
        .to_string();

    // Parse Range header if present
    if let Some(range_val) = headers.get(header::RANGE) {
        if let Ok(range_str) = range_val.to_str() {
            if let Some(spec) = range_str.strip_prefix("bytes=") {
                let (start, end) = parse_range(spec, total);
                let length = end - start + 1;
                let slice = raw_bytes.slice(start..=end);

                return Ok(Response::builder()
                    .status(StatusCode::PARTIAL_CONTENT)
                    .header(header::CONTENT_TYPE, &content_type)
                    .header(header::CONTENT_LENGTH, length)
                    .header(header::ACCEPT_RANGES, "bytes")
                    .header(
                        header::CONTENT_RANGE,
                        format!("bytes {}-{}/{}", start, end, total),
                    )
                    .body(Body::from(slice))
                    .unwrap());
            }
        }
    }

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, total)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(Body::from(raw_bytes))
        .unwrap())
}

fn parse_range(spec: &str, total: usize) -> (usize, usize) {
    let parts: Vec<&str> = spec.splitn(2, '-').collect();
    if parts.len() != 2 {
        return (0, total - 1);
    }
    if parts[0].is_empty() {
        // suffix range: "-500" means last 500 bytes
        let suffix: usize = parts[1].parse().unwrap_or(0);
        let start = total.saturating_sub(suffix);
        (start, total - 1)
    } else {
        let start: usize = parts[0].parse().unwrap_or(0);
        let end: usize = if parts[1].is_empty() {
            total - 1
        } else {
            parts[1].parse().unwrap_or(total - 1)
        };
        (start.min(total - 1), end.min(total - 1))
    }
}

fn jpeg_response(bytes: Vec<u8>) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .body(Body::from(bytes))
        .unwrap()
}

/// Decode a RAW file from in-memory bytes using imagepipe.
/// imagepipe requires a file path, so we write to a temp file.
fn decode_raw_from_bytes(
    bytes: &[u8],
    max_w: u32,
    max_h: u32,
) -> Result<image::RgbImage, AppError> {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new()
        .map_err(|e| AppError::Internal(format!("temp file error: {e}")))?;
    tmp.write_all(bytes)
        .map_err(|e| AppError::Internal(format!("temp write error: {e}")))?;
    tmp.flush()
        .map_err(|e| AppError::Internal(format!("temp flush error: {e}")))?;

    let edits = processing::EditParams::default();
    processing::process_raw(tmp.path(), &edits, max_w as usize, max_h as usize)
}

/// Decode a standard image (JPEG, PNG, etc.) from bytes and resize.
fn decode_image_from_bytes(
    bytes: &[u8],
    max_w: u32,
    max_h: u32,
) -> Result<image::RgbImage, AppError> {
    let img = image::load_from_memory(bytes)?;
    let img = img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3);
    Ok(img.to_rgb8())
}
