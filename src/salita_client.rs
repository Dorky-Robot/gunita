use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub endpoint: Option<String>,
    pub port: i64,
    pub is_self: bool,
    pub status: String,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub directories: Vec<String>,
}

#[derive(Clone)]
pub struct SalitaClient {
    client: reqwest::Client,
    base_url: String,
}

impl SalitaClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn base_url(&self) -> String {
        self.base_url.clone()
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn peer_url(endpoint: &str, port: i64) -> String {
        format!("http://{}:{}", endpoint, port)
    }

    /// Get the base URL for a device — local salita for self, peer endpoint for remote
    pub fn device_url(&self, device: &DeviceInfo) -> String {
        if device.is_self {
            self.base_url.clone()
        } else if let Some(ref ep) = device.endpoint {
            Self::peer_url(ep, device.port)
        } else {
            self.base_url.clone()
        }
    }

    pub async fn list_devices(&self) -> Result<Vec<DeviceInfo>, reqwest::Error> {
        self.client
            .get(format!("{}/api/v1/devices", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_node(&self, base: &str) -> Result<NodeInfo, reqwest::Error> {
        self.client
            .get(format!("{}/api/v1/node", base))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn list_files(
        &self,
        base: &str,
        dir: &str,
        path: &str,
    ) -> Result<Vec<FileEntry>, reqwest::Error> {
        self.client
            .get(format!("{}/api/v1/files", base))
            .query(&[("dir", dir), ("path", path)])
            .send()
            .await?
            .json()
            .await
    }

    pub async fn fetch_file_bytes(
        &self,
        base: &str,
        dir: &str,
        path: &str,
    ) -> Result<bytes::Bytes, reqwest::Error> {
        self.client
            .get(format!("{}/api/v1/files/read", base))
            .query(&[("dir", dir), ("path", path)])
            .send()
            .await?
            .bytes()
            .await
    }

    pub async fn file_info(
        &self,
        base: &str,
        dir: &str,
        path: &str,
    ) -> Result<FileInfo, reqwest::Error> {
        self.client
            .get(format!("{}/api/v1/files/info", base))
            .query(&[("dir", dir), ("path", path)])
            .send()
            .await?
            .json()
            .await
    }

    /// Fetch the content catalog from salita, with optional filters.
    pub async fn fetch_catalog(
        &self,
        base: &str,
        dir: Option<&str>,
        file_type: Option<&str>,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<CatalogEntry>, reqwest::Error> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(d) = dir {
            query.push(("dir", d.to_string()));
        }
        if let Some(ft) = file_type {
            query.push(("file_type", ft.to_string()));
        }
        if let Some(o) = offset {
            query.push(("offset", o.to_string()));
        }
        if let Some(l) = limit {
            query.push(("limit", l.to_string()));
        }

        self.client
            .get(format!("{}/api/v1/catalog", base))
            .query(&query)
            .send()
            .await?
            .json()
            .await
    }

    /// Fetch a pre-generated thumbnail by CID from salita.
    pub async fn fetch_thumbnail_by_cid(
        &self,
        base: &str,
        cid: &str,
        w: u32,
        h: u32,
    ) -> Result<bytes::Bytes, reqwest::Error> {
        self.client
            .get(format!("{}/api/v1/content/{}/thumbnail", base, cid))
            .query(&[("w", w.to_string()), ("h", h.to_string())])
            .send()
            .await?
            .bytes()
            .await
    }

    /// Fetch a mid-res preview by CID from salita (1600px JPEG, ~150-250KB).
    pub async fn fetch_preview_by_cid(
        &self,
        base: &str,
        cid: &str,
    ) -> Result<bytes::Bytes, reqwest::Error> {
        self.client
            .get(format!("{}/api/v1/content/{}/preview", base, cid))
            .send()
            .await?
            .bytes()
            .await
    }

    /// Ask salita to index specific files on demand, returning CIDs.
    /// Used to prioritize thumbnail generation for the page the user is viewing.
    pub async fn index_files(
        &self,
        base: &str,
        dir: &str,
        paths: &[String],
    ) -> Result<Vec<IndexResult>, reqwest::Error> {
        self.client
            .post(format!("{}/api/v1/index", base))
            .json(&serde_json::json!({ "dir": dir, "paths": paths }))
            .send()
            .await?
            .json()
            .await
    }

    /// Fetch file content by CID from salita.
    pub async fn fetch_content_by_cid(
        &self,
        base: &str,
        cid: &str,
    ) -> Result<bytes::Bytes, reqwest::Error> {
        self.client
            .get(format!("{}/api/v1/content/{}", base, cid))
            .send()
            .await?
            .bytes()
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    pub path: String,
    pub cid: Option<String>,
    pub has_thumbnail: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub cid: String,
    pub dir: String,
    pub path: String,
    pub filename: String,
    pub size: i64,
    pub mime: Option<String>,
    pub file_type: String,
    pub modified: Option<String>,
    pub has_thumbnail: bool,
    #[serde(default)]
    pub has_preview: bool,
}
