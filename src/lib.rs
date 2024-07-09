pub mod config;

use axum::Router;

pub fn get_router() -> Router {
    Router::new()
}

