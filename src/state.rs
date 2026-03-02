use std::path::PathBuf;
use std::sync::Arc;

use crate::config::Config;
use crate::db::DbPool;
use crate::salita_client::SalitaClient;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    data_dir: PathBuf,
    pool: DbPool,
    salita: SalitaClient,
    config: Config,
}

impl AppState {
    pub fn new(data_dir: PathBuf, pool: DbPool, salita_url: &str, config: Config) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                data_dir,
                pool,
                salita: SalitaClient::new(salita_url),
                config,
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
}
