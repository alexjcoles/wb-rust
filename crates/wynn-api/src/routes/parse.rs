use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ParseRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ParseResponse {
    pub items: Vec<SlotItem>,
    pub level: u32,
    pub assigned_sp: Option<SpValues>,
}

#[derive(Debug, Serialize)]
pub struct SlotItem {
    pub slot: String,
    pub name: String,
    pub id: u32,
}

#[derive(Debug, Serialize)]
pub struct SpValues {
    pub earth: i32,
    pub thunder: i32,
    pub water: i32,
    pub fire: i32,
    pub air: i32,
}

pub async fn parse_build(
    State(state): State<AppState>,
    Json(req): Json<ParseRequest>,
) -> Result<Json<ParseResponse>, (StatusCode, String)> {
    let build = wynn_encoding::decode_build(&req.url, &state.db)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to parse: {e}")))?;

    let items: Vec<SlotItem> = wynn_core::item::Slot::ALL
        .iter()
        .filter_map(|&slot| {
            build.item(slot).map(|item| SlotItem {
                slot: format!("{:?}", slot),
                name: item.name().to_string(),
                id: item.id(),
            })
        })
        .collect();

    let assigned_sp = build.assigned_sp.map(|sp| SpValues {
        earth: sp.earth,
        thunder: sp.thunder,
        water: sp.water,
        fire: sp.fire,
        air: sp.air,
    });

    Ok(Json(ParseResponse {
        items,
        level: build.level,
        assigned_sp,
    }))
}
