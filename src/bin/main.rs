use axum::{Router, routing::get};
use cases::{AppState, CONFIG, Tan, case, help, kv_sep_partition_option, logo, search, style};
use fjall::Config;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            "info,tantivy=warn,html5ever=error",
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let addr: SocketAddr = CONFIG.addr.parse().unwrap();
    let searcher = Arc::new(Tan::searcher().unwrap());

    let keyspace = Config::new(CONFIG.db.as_str()).open().unwrap();
    let db = keyspace
        .open_partition("cases", kv_sep_partition_option())
        .unwrap();
    let app_state = AppState { db, searcher };

    let middleware_stack = ServiceBuilder::new()
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::new(Duration::from_secs(10)));

    let app = Router::new()
        .route("/", get(search))
        .route("/case/{id}", get(case))
        .route("/style.css", get(style))
        .route("/help.txt", get(help))
        .route("/logo.png", get(logo))
        .layer(middleware_stack)
        .with_state(app_state);

    info!("listening on http://{}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
