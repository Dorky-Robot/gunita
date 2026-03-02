use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use rusqlite::params;

use crate::error::AppError;
use crate::models::*;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/collections", get(list_collections).post(create_collection))
        .route("/api/collections/{id}", get(get_collection))
        .route("/api/collections/{id}/memories", post(add_memory))
        .route(
            "/api/collections/{id}/memories/{mid}",
            delete(remove_memory),
        )
}

async fn list_collections(
    State(state): State<AppState>,
) -> Result<Json<Vec<Collection>>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    let mut stmt = conn.prepare(
        "SELECT id, title, description, created_at, updated_at
         FROM collections ORDER BY created_at DESC",
    )?;

    let collections = stmt
        .query_map([], |row| {
            Ok(Collection {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(collections))
}

async fn create_collection(
    State(state): State<AppState>,
    Json(body): Json<CreateCollection>,
) -> Result<Json<Collection>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    let id = uuid::Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO collections (id, title, description) VALUES (?1, ?2, ?3)",
        params![id, body.title, body.description],
    )?;

    let collection = conn.query_row(
        "SELECT id, title, description, created_at, updated_at
         FROM collections WHERE id = ?1",
        params![id],
        |row| {
            Ok(Collection {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )?;

    Ok(Json(collection))
}

async fn get_collection(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CollectionWithMemories>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    let collection = conn
        .query_row(
            "SELECT id, title, description, created_at, updated_at
             FROM collections WHERE id = ?1",
            params![id],
            |row| {
                Ok(Collection {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .map_err(|_| AppError::NotFound(format!("Collection not found: {id}")))?;

    let mut stmt = conn.prepare(
        "SELECT m.id, m.title, m.description, m.cover_path, m.location, m.started_at, m.ended_at, m.created_at, m.updated_at
         FROM memories m
         JOIN collection_memories cm ON cm.memory_id = m.id
         WHERE cm.collection_id = ?1
         ORDER BY cm.sort_order, m.created_at",
    )?;

    let memories = stmt
        .query_map(params![id], |row| {
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

    Ok(Json(CollectionWithMemories {
        collection,
        memories,
    }))
}

async fn add_memory(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
    Json(body): Json<AddCollectionMemory>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    // Verify collection exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM collections WHERE id = ?1",
        params![collection_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!(
            "Collection not found: {collection_id}"
        )));
    }

    // Verify memory exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM memories WHERE id = ?1",
        params![body.memory_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!(
            "Memory not found: {}",
            body.memory_id
        )));
    }

    conn.execute(
        "INSERT OR REPLACE INTO collection_memories (collection_id, memory_id, sort_order)
         VALUES (?1, ?2, ?3)",
        params![collection_id, body.memory_id, body.sort_order],
    )?;

    Ok(Json(serde_json::json!({ "added": true })))
}

async fn remove_memory(
    State(state): State<AppState>,
    Path((collection_id, memory_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state.db();
    let conn = pool.get().map_err(|e| AppError::Pool(e.to_string()))?;

    let affected = conn.execute(
        "DELETE FROM collection_memories WHERE collection_id = ?1 AND memory_id = ?2",
        params![collection_id, memory_id],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "Memory {memory_id} not in collection {collection_id}"
        )));
    }

    Ok(Json(serde_json::json!({ "removed": true })))
}
