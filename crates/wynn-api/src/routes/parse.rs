use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ParseRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ParseResponse {
    pub success: bool,
    pub message: Option<String>,
    // TODO: add build data
}

pub async fn parse_build(
    State(state): State<AppState>,
    Json(req): Json<ParseRequest>,
) -> Json<ParseResponse> {
    match wynn_encoding::decode_build(&req.url, &state.db) {
        Ok(_build) => Json(ParseResponse {
            success: true,
            message: Some("Build parsed successfully".into()),
        }),
        Err(e) => Json(ParseResponse {
            success: false,
            message: Some(format!("Failed to parse: {e}")),
        }),
    }
}
