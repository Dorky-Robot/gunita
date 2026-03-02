use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::salita_client::{DeviceInfo, FileEntry};
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
    let entries: Vec<BrowseEntry> = files
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .map(|f| {
            let file_type = if f.is_dir {
                "dir"
            } else {
                classify_file(&f.name)
            };
            BrowseEntry {
                name: f.name,
                path: f.path,
                is_dir: f.is_dir,
                size: f.size,
                modified: f.modified,
                file_type,
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
