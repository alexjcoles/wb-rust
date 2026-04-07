mod ai;
mod routes;
mod state;

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use tower_http::cors::CorsLayer;
use wynn_core::db::{fetch_item_data, ItemDb};

use crate::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wynn_api=debug,tower_http=debug".into()),
        )
        .init();

    tracing::info!("wynn-api starting");

    // Load item database
    let data_path = PathBuf::from("data/items.json");
    if !data_path.exists() {
        tracing::info!("item data not found, fetching...");
        fetch_item_data(&data_path)
            .await
            .expect("failed to fetch item data");
    }

    let db = ItemDb::load_from_file(&data_path).expect("failed to load item database");
    let state = AppState {
        db: Arc::new(db),
    };

    let app = Router::new()
        .nest("/api", routes::api_routes())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:5656";
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
