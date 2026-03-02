use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use rusqlite::params;

use crate::error::AppError;
use crate::models::*;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/memories", get(list_memories).post(create_memory))
        .route(
            "/api/memories/{id}",
            get(get_memory).put(update_memory).delete(delete_memory),
        )
        .route("/api/memories/{id}/items", post(add_item))
        .route("/api/memories/{id}/items/{item_id}", delete(remove_item))
        .route("/api/memories/{id}/play", get(playback))
}

async fn list_memories(State(state): State<AppState>) -> Result<Json<Vec<Memory>>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    let mut stmt = conn.prepare(
        "SELECT id, title, description, cover_path, location, started_at, ended_at, created_at, updated_at
         FROM memories ORDER BY created_at DESC",
    )?;

    let memories = stmt
        .query_map([], |row| {
            Ok(Memory {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                cover_path: row.get(3)?,
                location: row.get(4)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(memories))
}

async fn create_memory(
    State(state): State<AppState>,
    Json(body): Json<CreateMemory>,
) -> Result<Json<Memory>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    let id = uuid::Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO memories (id, title, description, cover_path, location, started_at, ended_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            body.title,
            body.description,
            body.cover_path,
            body.location,
            body.started_at,
            body.ended_at,
        ],
    )?;

    let memory = conn.query_row(
        "SELECT id, title, description, cover_path, location, started_at, ended_at, created_at, updated_at
         FROM memories WHERE id = ?1",
        params![id],
        |row| {
            Ok(Memory {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                cover_path: row.get(3)?,
                location: row.get(4)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )?;

    Ok(Json(memory))
}

async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MemoryWithItems>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    let memory = conn
        .query_row(
            "SELECT id, title, description, cover_path, location, started_at, ended_at, created_at, updated_at
             FROM memories WHERE id = ?1",
            params![id],
            |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    cover_path: row.get(3)?,
                    location: row.get(4)?,
                    started_at: row.get(5)?,
                    ended_at: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .map_err(|_| AppError::NotFound(format!("Memory not found: {id}")))?;

    let mut stmt = conn.prepare(
        "SELECT id, memory_id, kind, device_id, dir, path, file_type, caption, taken_at, sort_order, created_at, cid
         FROM memory_items WHERE memory_id = ?1 ORDER BY sort_order, created_at",
    )?;

    let items = stmt
        .query_map(params![id], |row| {
            Ok(MemoryItem {
                id: row.get(0)?,
                memory_id: row.get(1)?,
                kind: row.get(2)?,
                device_id: row.get(3)?,
                dir: row.get(4)?,
                path: row.get(5)?,
                file_type: row.get(6)?,
                caption: row.get(7)?,
                taken_at: row.get(8)?,
                sort_order: row.get(9)?,
                created_at: row.get(10)?,
                cid: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT id, memory_id, content, sort_order, created_at
         FROM memory_notes WHERE memory_id = ?1 ORDER BY sort_order, created_at",
    )?;

    let notes = stmt
        .query_map(params![id], |row| {
            Ok(MemoryNote {
                id: row.get(0)?,
                memory_id: row.get(1)?,
                content: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(MemoryWithItems {
        memory,
        items,
        notes,
    }))
}

async fn update_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateMemory>,
) -> Result<Json<Memory>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    // Verify exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM memories WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!("Memory not found: {id}")));
    }

    if let Some(ref title) = body.title {
        conn.execute(
            "UPDATE memories SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![title, id],
        )?;
    }
    if let Some(ref description) = body.description {
        conn.execute(
            "UPDATE memories SET description = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![description, id],
        )?;
    }
    if let Some(ref cover_path) = body.cover_path {
        conn.execute(
            "UPDATE memories SET cover_path = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![cover_path, id],
        )?;
    }
    if let Some(ref location) = body.location {
        conn.execute(
            "UPDATE memories SET location = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![location, id],
        )?;
    }
    if let Some(ref started_at) = body.started_at {
        conn.execute(
            "UPDATE memories SET started_at = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![started_at, id],
        )?;
    }
    if let Some(ref ended_at) = body.ended_at {
        conn.execute(
            "UPDATE memories SET ended_at = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![ended_at, id],
        )?;
    }

    let memory = conn.query_row(
        "SELECT id, title, description, cover_path, location, started_at, ended_at, created_at, updated_at
         FROM memories WHERE id = ?1",
        params![id],
        |row| {
            Ok(Memory {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                cover_path: row.get(3)?,
                location: row.get(4)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )?;

    Ok(Json(memory))
}

async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    let affected = conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("Memory not found: {id}")));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn add_item(
    State(state): State<AppState>,
    Path(memory_id): Path<String>,
    Json(body): Json<AddMemoryItem>,
) -> Result<Json<MemoryItem>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    // Verify memory exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM memories WHERE id = ?1",
        params![memory_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!(
            "Memory not found: {memory_id}"
        )));
    }

    let id = uuid::Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO memory_items (id, memory_id, kind, device_id, dir, path, file_type, caption, taken_at, sort_order, cid)
         VALUES (?1, ?2, 'media', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            memory_id,
            body.device_id,
            body.dir,
            body.path,
            body.file_type,
            body.caption,
            body.taken_at,
            body.sort_order,
            body.cid,
        ],
    )?;

    let item = conn.query_row(
        "SELECT id, memory_id, kind, device_id, dir, path, file_type, caption, taken_at, sort_order, created_at, cid
         FROM memory_items WHERE id = ?1",
        params![id],
        |row| {
            Ok(MemoryItem {
                id: row.get(0)?,
                memory_id: row.get(1)?,
                kind: row.get(2)?,
                device_id: row.get(3)?,
                dir: row.get(4)?,
                path: row.get(5)?,
                file_type: row.get(6)?,
                caption: row.get(7)?,
                taken_at: row.get(8)?,
                sort_order: row.get(9)?,
                created_at: row.get(10)?,
                cid: row.get(11)?,
            })
        },
    )?;

    Ok(Json(item))
}

async fn remove_item(
    State(state): State<AppState>,
    Path((memory_id, item_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    let affected = conn.execute(
        "DELETE FROM memory_items WHERE id = ?1 AND memory_id = ?2",
        params![item_id, memory_id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "Item {item_id} not found in memory {memory_id}"
        )));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn playback(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<PlaybackEntry>>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    // Verify memory exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM memories WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!("Memory not found: {id}")));
    }

    let mut stmt = conn.prepare(
        "SELECT id, memory_id, kind, device_id, dir, path, file_type, caption, taken_at, sort_order, created_at, cid
         FROM memory_items WHERE memory_id = ?1 ORDER BY sort_order, created_at",
    )?;

    let items: Vec<MemoryItem> = stmt
        .query_map(params![id], |row| {
            Ok(MemoryItem {
                id: row.get(0)?,
                memory_id: row.get(1)?,
                kind: row.get(2)?,
                device_id: row.get(3)?,
                dir: row.get(4)?,
                path: row.get(5)?,
                file_type: row.get(6)?,
                caption: row.get(7)?,
                taken_at: row.get(8)?,
                sort_order: row.get(9)?,
                created_at: row.get(10)?,
                cid: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT id, memory_id, content, sort_order, created_at
         FROM memory_notes WHERE memory_id = ?1 ORDER BY sort_order, created_at",
    )?;

    let notes: Vec<MemoryNote> = stmt
        .query_map(params![id], |row| {
            Ok(MemoryNote {
                id: row.get(0)?,
                memory_id: row.get(1)?,
                content: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // Merge items and notes by sort_order
    let mut entries: Vec<PlaybackEntry> = Vec::new();

    for item in items {
        entries.push(PlaybackEntry {
            entry_type: "media".to_string(),
            sort_order: item.sort_order,
            item: Some(item),
            note: None,
        });
    }

    for note in notes {
        entries.push(PlaybackEntry {
            entry_type: "note".to_string(),
            sort_order: note.sort_order,
            item: None,
            note: Some(note),
        });
    }

    entries.sort_by_key(|e| e.sort_order);

    Ok(Json(entries))
}
