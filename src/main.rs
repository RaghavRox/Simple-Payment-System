use std::time::Duration;

use axum::{
    error_handling::HandleErrorLayer,
    http::{Request, Response, StatusCode},
    BoxError,
};
use simple_payment_system::{config::Config, get_router};
use tokio::{self, net::TcpListener, signal};
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};
use tower_http::{catch_panic::CatchPanicLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{info, info_span, Span};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //Initiate logging
    tracing_subscriber::fmt::init();

    //Read env variables
    dotenvy::dotenv().ok();
    Config::init_from_env();

    // Create a axum app.
    let app = get_router().layer((
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|err: BoxError| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled error: {}", err),
                )
            }))
            .layer(BufferLayer::new(1024))
            .layer(RateLimitLayer::new(10_000, Duration::from_secs(1))),
        CatchPanicLayer::new(),
        TraceLayer::new_for_http()
            .make_span_with(|request: &Request<_>| {
                let path = request.uri().to_string();

                info_span!(
                    "http_request",
                    method = ?request.method(),
                    path,
                )
            })
            .on_response(|_response: &Response<_>, latency: Duration, _span: &Span| {
                info!("latency = {:#?}", latency);
            }),
        // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
        // requests don't hang forever.
        TimeoutLayer::new(Duration::from_secs(10)),
    ));
    // Create a `TcpListener` using tokio.
    let listener = TcpListener::bind("0.0.0.0:80").await?;
    // Run the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
    _ = ctrl_c => {},
    _ = terminate => {},
     }
}
