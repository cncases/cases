use axum::{routing::get, Router};
use cases::{case, logo, search, style, AppState, Tan, CONFIG};
use redb::Database;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info,tantivy=warn"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let addr: SocketAddr = CONFIG.addr.parse().unwrap();
    info!("listening on http://{}", addr);

    let searcher = Arc::new(Tan::searcher().unwrap());

    let db = Database::create(CONFIG.db.as_str()).unwrap();
    let app_state = AppState {
        db: Arc::new(db),
        searcher,
    };

    let middleware_stack = ServiceBuilder::new()
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().level(Level::INFO)))
        .layer(TimeoutLayer::new(Duration::from_secs(10)));

    let app = Router::new()
        .route("/", get(search))
        .route("/case/:id", get(case))
        .route("/style.css", get(style))
        .route("/logo.png", get(logo))
        .layer(middleware_stack)
        .with_state(app_state);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
