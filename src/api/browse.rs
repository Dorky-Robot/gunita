use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::salita_client::{CatalogEntry, DeviceInfo, FileEntry};
use crate::state::AppState;

#[derive(Serialize)]
struct DeviceWithDirs {
    #[serde(flatten)]
    device: DeviceInfo,
    directories: Vec<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/devices", get(list_devices))
        .route("/api/browse", get(browse_files))
        .route("/api/browse/months", get(browse_months))
        .route("/api/catalog", get(catalog))
        .route("/api/index/stats", get(index_stats))
}

async fn list_devices(State(state): State<AppState>) -> Result<Json<Vec<DeviceWithDirs>>, AppError> {
    let salita = state.salita();
    let devices = salita
        .list_devices()
        .await
        .map_err(|e| AppError::Internal(format!("salita error: {e}")))?;

    let mut result = Vec::new();
    for device in devices {
        let base = salita.device_url(&device);
        let dirs = match salita.get_node(&base).await {
            Ok(node) => node.directories,
            Err(_) => Vec::new(),
        };
        result.push(DeviceWithDirs {
            device,
            directories: dirs,
        });
    }

    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct BrowseParams {
    pub device: String,
    pub dir: String,
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_offset")]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_sort")]
    pub sort: String,
}

fn default_sort() -> String {
    "newest".to_string()
}

fn default_offset() -> usize {
    0
}

fn default_limit() -> usize {
    100
}

const RAW_EXTENSIONS: &[&str] = &[
    "cr2", "cr3", "nef", "arw", "orf", "rw2", "dng", "raf", "pef", "srw", "x3f", "3fr", "mrw",
    "nrw", "raw",
];

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "heic", "heif"];

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "avi", "mkv", "360", "webm"];

fn classify_file(name: &str) -> &'static str {
    let ext = name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();
    if RAW_EXTENSIONS.contains(&ext.as_str()) {
        "raw"
    } else if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        "image"
    } else if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        "video"
    } else {
        "other"
    }
}

#[derive(Serialize)]
struct BrowseEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: Option<String>,
    file_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cid: Option<String>,
    /// true when thumbnail is ready (fully processed)
    processed: bool,
}

#[derive(Serialize)]
struct BrowseResponse {
    entries: Vec<BrowseEntry>,
    total: usize,
    offset: usize,
    has_more: bool,
}

async fn fetch_sorted_files(
    state: &AppState,
    device_id: &str,
    dir: &str,
    path: &str,
    sort: &str,
) -> Result<Vec<FileEntry>, AppError> {
    let salita = state.salita();
    let devices = salita
        .list_devices()
        .await
        .map_err(|e| AppError::Internal(format!("salita error: {e}")))?;

    let device = devices
        .iter()
        .find(|d| d.id == device_id || d.name == device_id)
        .ok_or_else(|| AppError::NotFound(format!("Device not found: {}", device_id)))?;

    let base = salita.device_url(device);
    let mut files = salita
        .list_files(&base, dir, path)
        .await
        .map_err(|e| AppError::Internal(format!("salita error: {e}")))?;

    // Sort by modified date — ISO 8601 strings sort correctly with lexicographic comparison
    match sort {
        "oldest" => files.sort_by(|a, b| {
            let ma = a.modified.as_deref().unwrap_or("");
            let mb = b.modified.as_deref().unwrap_or("");
            ma.cmp(mb)
        }),
        _ => files.sort_by(|a, b| {
            let ma = a.modified.as_deref().unwrap_or("");
            let mb = b.modified.as_deref().unwrap_or("");
            mb.cmp(ma)
        }),
    }

    Ok(files)
}

async fn browse_files(
    State(state): State<AppState>,
    Query(params): Query<BrowseParams>,
) -> Result<Json<BrowseResponse>, AppError> {
    let files = fetch_sorted_files(&state, &params.device, &params.dir, &params.path, &params.sort).await?;

    let total = files.len();
    let page: Vec<FileEntry> = files
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .collect();

    // Look up already-indexed CIDs + EXIF dates (fast, non-blocking DB lookup)
    // and kick off background indexing for files that aren't indexed yet
    let catalog_map = lookup_and_queue_indexing(&state, &params.dir, &page).await;

    let entries: Vec<BrowseEntry> = page
        .into_iter()
        .map(|f| {
            let file_type = if f.is_dir {
                "dir"
            } else {
                classify_file(&f.name)
            };
            let catalog_info = catalog_map.get(&f.path);
            let cid = catalog_info.and_then(|ci| ci.cid.clone());
            let processed = f.is_dir
                || file_type == "other"
                || file_type == "video"
                || catalog_info.map_or(false, |ci| ci.has_thumbnail && ci.has_preview);
            // Prefer EXIF date from catalog over filesystem mtime
            let modified = catalog_info
                .and_then(|ci| ci.modified.clone())
                .or(f.modified);
            BrowseEntry {
                name: f.name,
                path: f.path,
                is_dir: f.is_dir,
                size: f.size,
                modified,
                file_type,
                cid,
                processed,
            }
        })
        .collect();

    let has_more = params.offset + params.limit < total;

    Ok(Json(BrowseResponse {
        entries,
        total,
        offset: params.offset,
        has_more,
    }))
}

struct CatalogInfo {
    cid: Option<String>,       // Only set when thumbnail is ready (fast path)
    modified: Option<String>,  // EXIF date, always set if indexed
    has_thumbnail: bool,
    has_preview: bool,
}

/// Fast: look up already-indexed CIDs + EXIF dates from salita's catalog.
/// Then fire-and-forget background indexing for un-indexed files so next load is fast.
async fn lookup_and_queue_indexing(
    state: &AppState,
    dir: &str,
    page: &[FileEntry],
) -> HashMap<String, CatalogInfo> {
    let image_paths: Vec<String> = page
        .iter()
        .filter(|f| !f.is_dir)
        .filter(|f| {
            let ft = classify_file(&f.name);
            ft == "image" || ft == "raw"
        })
        .map(|f| f.path.clone())
        .collect();

    if image_paths.is_empty() {
        return HashMap::new();
    }

    // Use in-memory cached catalog (refreshes every 60s, instant on repeat loads)
    let catalog_entries = state.cached_catalog(dir).await;
    let catalog_map: HashMap<String, CatalogInfo> = catalog_entries
        .into_iter()
        .map(|e| {
            (
                e.path,
                CatalogInfo {
                    cid: if e.has_thumbnail { Some(e.cid) } else { None },
                    modified: e.modified,
                    has_thumbnail: e.has_thumbnail,
                    has_preview: e.has_preview,
                },
            )
        })
        .collect();

    // Find paths that need thumbnail generation (unindexed or indexed without thumbnail)
    let needs_thumbnail: Vec<String> = image_paths
        .into_iter()
        .filter(|p| {
            catalog_map.get(p).map_or(true, |ci| ci.cid.is_none())
        })
        .collect();

    // Fire-and-forget: ask salita to index the missing files in the background
    if !needs_thumbnail.is_empty() {
        let state2 = state.clone();
        let dir = dir.to_string();
        tokio::spawn(async move {
            let salita = state2.salita();
            let base = salita.base_url();
            let _ = salita.index_files(&base, &dir, &needs_thumbnail).await;
            // Invalidate cache so next load picks up newly indexed files
            state2.invalidate_catalog_cache(&dir).await;
        });
    }

    catalog_map
}

#[derive(Deserialize)]
pub struct MonthsParams {
    pub device: String,
    pub dir: String,
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_sort")]
    pub sort: String,
}

#[derive(Serialize)]
struct MonthGroup {
    month: String,
    label: String,
    count: usize,
    offset: usize,
}

fn month_key(modified: Option<&str>) -> String {
    match modified {
        Some(s) if s.len() >= 7 => s[..7].to_string(),
        _ => "unknown".to_string(),
    }
}

fn month_label(key: &str) -> String {
    if key == "unknown" {
        return "Unknown Date".to_string();
    }
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() < 2 {
        return key.to_string();
    }
    let month_name = match parts[1] {
        "01" => "January",
        "02" => "February",
        "03" => "March",
        "04" => "April",
        "05" => "May",
        "06" => "June",
        "07" => "July",
        "08" => "August",
        "09" => "September",
        "10" => "October",
        "11" => "November",
        "12" => "December",
        _ => "Unknown",
    };
    format!("{} {}", month_name, parts[0])
}

async fn browse_months(
    State(state): State<AppState>,
    Query(params): Query<MonthsParams>,
) -> Result<Json<Vec<MonthGroup>>, AppError> {
    let files = fetch_sorted_files(&state, &params.device, &params.dir, &params.path, &params.sort).await?;

    let mut groups: Vec<MonthGroup> = Vec::new();
    let mut current_key = String::new();
    let mut cumulative_offset = 0;

    for f in &files {
        let key = month_key(f.modified.as_deref());
        if key != current_key {
            current_key = key.clone();
            groups.push(MonthGroup {
                label: month_label(&key),
                month: key,
                count: 1,
                offset: cumulative_offset,
            });
        } else if let Some(last) = groups.last_mut() {
            last.count += 1;
        }
        cumulative_offset += 1;
    }

    Ok(Json(groups))
}

// --- Catalog (CID-based) ---

#[derive(Deserialize)]
struct CatalogParams {
    dir: Option<String>,
    file_type: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

async fn catalog(
    State(state): State<AppState>,
    Query(params): Query<CatalogParams>,
) -> Result<Json<Vec<CatalogEntry>>, AppError> {
    let salita = state.salita();
    let base = &salita.base_url();
    let entries = salita
        .fetch_catalog(
            base,
            params.dir.as_deref(),
            params.file_type.as_deref(),
            params.offset,
            params.limit,
        )
        .await
        .map_err(|e| AppError::Internal(format!("catalog fetch error: {e}")))?;

    Ok(Json(entries))
}

async fn index_stats(
    State(state): State<AppState>,
) -> Result<axum::response::Response, AppError> {
    let salita = state.salita();
    let base = salita.base_url();
    let resp = salita
        .client()
        .get(format!("{}/api/v1/catalog/stats", base))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("stats fetch error: {e}")))?;

    let body = resp.bytes().await
        .map_err(|e| AppError::Internal(format!("stats read error: {e}")))?;

    Ok(axum::response::Response::builder()
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap())
}
