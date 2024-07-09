mod app_state;
mod config;
mod db;

use app_state::AppState;
use axum::Router;

pub async fn get_router() -> anyhow::Result<Router> {
    //Construct App State
    let app_state = AppState::init().await?;

    Ok(Router::new().with_state(app_state))
}

