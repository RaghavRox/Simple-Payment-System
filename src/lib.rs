mod api_doc;
mod app_state;
mod balance;
mod config;
mod db;
mod error;
mod transaction;
mod user;
mod utils;

use api_doc::ApiDoc;
use app_state::AppState;
use axum::Router;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
pub async fn get_router() -> anyhow::Result<Router> {
    //Construct App State
    let app_state = AppState::init().await?;

    Ok(Router::new()
        .nest("/users", user::get_router(app_state.clone()))
        .nest("/transactions", transaction::get_router(app_state.clone()))
        .nest("/balance", balance::get_router(app_state.clone()))
        .merge(
            SwaggerUi::new("/docs")
                .url("/docs/openapi.json", ApiDoc::openapi())
                .config(
                    utoipa_swagger_ui::Config::default()
                        .doc_expansion(r#"["list"*,"full","none"]"#)
                        .request_snippets_enabled(true)
                        .persist_authorization(true),
                ),
        ))
}
