use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use serde_json::{json, Value};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use gunita::api;
use gunita::config::{Cli, Command, Config};
use gunita::db;
use gunita::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cli = Cli::parse();
    let config = Config::load(&cli)?;
    let data_dir = Config::data_dir(&cli);
    let db_path = Config::db_path(&cli);

    match cli.command {
        Command::Serve { .. } => {
            // Ensure data dir exists
            std::fs::create_dir_all(&data_dir)?;

            // Create DB pool and run migrations
            let pool = db::create_pool(&db_path)?;
            db::run_migrations(&pool)?;

            // Ensure cache dir exists
            let cache_dir = data_dir.join("cache");
            std::fs::create_dir_all(&cache_dir)?;

            let salita_url = config.salita.url.clone();
            let host = config.server.host.clone();
            let port = config.server.port;

            let state = AppState::new(data_dir, pool, &salita_url, config);

            let app = Router::new()
                .route("/health", get(health))
                .merge(api::router())
                .layer(TraceLayer::new_for_http())
                .with_state(state);

            // Static file serving: API routes take priority, fallback to static files
            let static_dir = std::env::current_dir()?.join("static");
            let app = app.fallback_service(
                tower_http::services::ServeDir::new(&static_dir)
                    .fallback(tower_http::services::ServeFile::new(static_dir.join("index.html"))),
            );

            let addr = format!("{host}:{port}");
            let listener = tokio::net::TcpListener::bind(&addr).await?;

            tracing::info!("Gunita server listening on http://{addr}");
            tracing::info!("Salita endpoint: {salita_url}");

            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
