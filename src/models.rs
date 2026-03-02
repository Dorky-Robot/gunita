use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub title: String,
    pub description: String,
    pub cover_path: Option<String>,
    pub location: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub memory_id: String,
    pub kind: String,
    pub device_id: String,
    pub dir: String,
    pub path: String,
    pub file_type: String,
    pub caption: String,
    pub taken_at: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNote {
    pub id: String,
    pub memory_id: String,
    pub content: String,
    pub sort_order: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryWithItems {
    #[serde(flatten)]
    pub memory: Memory,
    pub items: Vec<MemoryItem>,
    pub notes: Vec<MemoryNote>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMemory {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub cover_path: Option<String>,
    pub location: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemory {
    pub title: Option<String>,
    pub description: Option<String>,
    pub cover_path: Option<String>,
    pub location: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemoryItem {
    pub device_id: String,
    pub dir: String,
    pub path: String,
    #[serde(default = "default_file_type")]
    pub file_type: String,
    #[serde(default)]
    pub caption: String,
    pub taken_at: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
    pub cid: Option<String>,
}

fn default_file_type() -> String {
    "image".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionWithMemories {
    #[serde(flatten)]
    pub collection: Collection,
    pub memories: Vec<Memory>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCollection {
    pub title: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct AddCollectionMemory {
    pub memory_id: String,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Serialize)]
pub struct PlaybackEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub sort_order: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<MemoryItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<MemoryNote>,
}
