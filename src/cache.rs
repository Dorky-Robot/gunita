use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::state::AppState;

fn path_hash(dir: &str, path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(dir.as_bytes());
    hasher.update(b"/");
    hasher.update(path.as_bytes());
    let result = hasher.finalize();
    result[..12]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

pub fn mesh_cache_dir(state: &AppState, device_id: &str) -> std::path::PathBuf {
    state.cache_dir().join(device_id)
}

pub fn mesh_cache_path(
    state: &AppState,
    device_id: &str,
    dir: &str,
    path: &str,
    width: u32,
    height: u32,
) -> std::path::PathBuf {
    let hash = path_hash(dir, path);
    mesh_cache_dir(state, device_id).join(format!("{hash}_{width}x{height}.jpg"))
}

pub async fn ensure_mesh_cache_dir(state: &AppState, device_id: &str) -> Result<(), AppError> {
    let dir = mesh_cache_dir(state, device_id);
    tokio::fs::create_dir_all(dir).await?;
    Ok(())
}
