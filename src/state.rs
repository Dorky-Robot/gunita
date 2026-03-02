use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;

use crate::config::Config;
use crate::db::DbPool;
use crate::salita_client::{CatalogEntry, SalitaClient};

const CATALOG_TTL_SECS: u64 = 60;

struct CachedCatalog {
    entries: Vec<CatalogEntry>,
    fetched_at: Instant,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    data_dir: PathBuf,
    pool: DbPool,
    salita: SalitaClient,
    config: Config,
    catalog_cache: Mutex<HashMap<String, CachedCatalog>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf, pool: DbPool, salita_url: &str, config: Config) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                data_dir,
                pool,
                salita: SalitaClient::new(salita_url),
                config,
                catalog_cache: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.inner.data_dir.join("cache")
    }

    pub fn salita(&self) -> &SalitaClient {
        &self.inner.salita
    }

    pub fn db(&self) -> &DbPool {
        &self.inner.pool
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// Get cached catalog entries for a directory. Returns from cache if fresh,
    /// otherwise fetches from salita and caches for 60 seconds.
    pub async fn cached_catalog(&self, dir: &str) -> Vec<CatalogEntry> {
        // Check cache first
        {
            let cache = self.inner.catalog_cache.lock().await;
            if let Some(cached) = cache.get(dir) {
                if cached.fetched_at.elapsed().as_secs() < CATALOG_TTL_SECS {
                    return cached.entries.clone();
                }
            }
        }

        // Fetch from salita
        let salita = self.salita();
        let base = salita.base_url();
        let entries = match salita
            .fetch_catalog(&base, Some(dir), None, None, Some(50000))
            .await
        {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        // Store in cache
        {
            let mut cache = self.inner.catalog_cache.lock().await;
            cache.insert(
                dir.to_string(),
                CachedCatalog {
                    entries: entries.clone(),
                    fetched_at: Instant::now(),
                },
            );
        }

        entries
    }

    /// Invalidate the catalog cache for a directory (e.g. after triggering indexing).
    pub async fn invalidate_catalog_cache(&self, dir: &str) {
        let mut cache = self.inner.catalog_cache.lock().await;
        cache.remove(dir);
    }
}
