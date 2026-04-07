use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use wynn_core::analyze::{analyze_build as do_analyze, Severity};
use wynn_core::calculate::calculate_build_stats;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub archetype: String,
    pub survivability_score: f64,
    pub dps_score: f64,
    pub stats: StatsSnapshot,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Serialize)]
pub struct StatsSnapshot {
    pub hp: i32,
    pub ehp: f64,
    pub hpr: i32,
    pub life_steal: i32,
    pub mana_regen: i32,
    pub spell_damage_raw: i32,
    pub spell_damage_pct: i32,
    pub walk_speed: i32,
    pub earth_defence: i32,
    pub thunder_defence: i32,
    pub water_defence: i32,
    pub fire_defence: i32,
    pub air_defence: i32,
    pub assigned_sp_total: i32,
}

#[derive(Debug, Serialize)]
pub struct Issue {
    pub severity: String,
    pub category: String,
    pub description: String,
}

pub async fn analyze_build(
    State(state): State<AppState>,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, (StatusCode, String)> {
    let build = wynn_encoding::decode_build(&req.url, &state.db)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to parse: {e}")))?;

    let stats = calculate_build_stats(&build);
    let analysis = do_analyze(&stats);

    Ok(Json(AnalyzeResponse {
        archetype: format!("{:?}", analysis.archetype),
        survivability_score: analysis.survivability_score,
        dps_score: analysis.dps_score,
        stats: StatsSnapshot {
            hp: stats.hp,
            ehp: stats.ehp,
            hpr: stats.hpr,
            life_steal: stats.life_steal,
            mana_regen: stats.mana_regen,
            spell_damage_raw: stats.spell_damage_raw,
            spell_damage_pct: stats.spell_damage_pct,
            walk_speed: stats.walk_speed,
            earth_defence: stats.elemental_defence.earth,
            thunder_defence: stats.elemental_defence.thunder,
            water_defence: stats.elemental_defence.water,
            fire_defence: stats.elemental_defence.fire,
            air_defence: stats.elemental_defence.air,
            assigned_sp_total: stats.sp_assignment.total_assigned(),
        },
        issues: analysis
            .issues
            .iter()
            .map(|i| Issue {
                severity: match i.severity {
                    Severity::Critical => "critical",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                }
                .into(),
                category: format!("{:?}", i.category),
                description: i.description.clone(),
            })
            .collect(),
    }))
}
