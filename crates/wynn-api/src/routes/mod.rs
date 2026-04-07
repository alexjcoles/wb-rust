pub mod analyze;
pub mod chat;
pub mod parse;
pub mod solve;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/parse", post(parse::parse_build))
        .route("/analyze", post(analyze::analyze_build))
        .route("/solve", post(solve::solve_build))
        .route("/chat", post(chat::chat))
}
