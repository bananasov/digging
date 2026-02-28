use std::sync::Arc;

use axum::extract::{MatchedPath, Request};
use axum::{Router, routing::get};

use digging::routes;
use digging::websockets::WebSocketSessionManager;
use kameo::actor::Spawn;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    
    // Initialize metric descriptions
    digging::metrics::describe_metrics();

    let session_manager = WebSocketSessionManager::spawn(WebSocketSessionManager::new());
    let arced = Arc::new(session_manager);

    let app = Router::new()
        .route("/", get(root))
        .nest("/api", routes::api::config())
        .nest("/ws", routes::ws::config())
        .with_state(arced)
        .layer(CorsLayer::permissive())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    // Track HTTP metrics - increment active requests
                    #[cfg(feature = "metrics")]
                    {
                        digging::metrics::gauge!("http.requests.active").increment(1.0);
                    }

                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        matched_path,
                        some_other_field = tracing::field::Empty,
                    )
                })
                .on_response(|_response: &axum::response::Response, _latency: std::time::Duration, _span: &tracing::Span| {
                    #[cfg(feature = "metrics")]
                    {
                        // Get the matched path from the span or use "unknown"
                        let matched_path = _response
                            .extensions()
                            .get::<MatchedPath>()
                            .map(|mp| mp.as_str().to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        
                        let status = _response.status().as_u16().to_string();
                        
                        digging::metrics::counter!("http.requests.total",
                            "path" => matched_path.clone(),
                            "status" => status
                        ).increment(1);
                        
                        digging::metrics::histogram!("http.request.duration",
                            "path" => matched_path
                        ).record(_latency.as_secs_f64());
                        
                        digging::metrics::gauge!("http.requests.active").decrement(1.0);
                    }
                }),
        );

    // Read bind address from environment, default to 0.0.0.0:3000 for Docker compatibility
    let bind_address = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await?;

    Ok(())
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
