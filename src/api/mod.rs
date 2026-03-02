mod browse;
mod collections;
mod media;
mod memories;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(browse::router())
        .merge(media::router())
        .merge(memories::router())
        .merge(collections::router())
}
