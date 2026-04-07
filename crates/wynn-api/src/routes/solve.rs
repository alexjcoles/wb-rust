use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use wynn_core::item::Slot;
use wynn_solver::constraints::{Constraints, Objective, StatType};
use wynn_solver::search::{solve, SolverConfig};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SolveRequest {
    pub url: String,
    pub locked_slots: Vec<String>,
    #[serde(default)]
    pub constraints: Constraints,
    #[serde(default)]
    pub objectives: Vec<String>,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_min_level")]
    pub min_item_level: u32,
    #[serde(default = "default_available_points")]
    pub available_points: u32,
}

fn default_max_results() -> usize { 5 }
fn default_min_level() -> u32 { 90 }
fn default_available_points() -> u32 { 200 }

#[derive(Debug, Serialize)]
pub struct SolveResponse {
    pub results: Vec<SolveResult>,
}

#[derive(Debug, Serialize)]
pub struct SolveResult {
    pub items: Vec<SlotItem>,
    pub url: String,
    pub score: f64,
    pub hp: i32,
    pub ehp: f64,
    pub assigned_sp_total: i32,
}

#[derive(Debug, Serialize)]
pub struct SlotItem {
    pub slot: String,
    pub name: String,
    pub id: u32,
}

pub async fn solve_build(
    State(state): State<AppState>,
    Json(req): Json<SolveRequest>,
) -> Result<Json<SolveResponse>, (StatusCode, String)> {
    let mut build = wynn_encoding::decode_build(&req.url, &state.db)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to parse: {e}")))?;

    build.available_points = req.available_points;

    let locked: Vec<Slot> = req
        .locked_slots
        .iter()
        .filter_map(|s| parse_slot(s))
        .collect();

    let objectives: Vec<Objective> = req
        .objectives
        .iter()
        .filter_map(|s| parse_objective(s))
        .collect();

    let config = SolverConfig {
        constraints: req.constraints,
        objectives,
        max_results: req.max_results.min(20),
        min_item_level: req.min_item_level,
    };

    let results = solve(&build, &locked, &config, &state.db);

    let response_results: Vec<SolveResult> = results
        .into_iter()
        .map(|r| {
            let url_hash = wynn_encoding::encode_build(&r.build);
            let items: Vec<SlotItem> = Slot::ALL
                .iter()
                .filter_map(|&slot| {
                    r.build.item(slot).map(|item| SlotItem {
                        slot: format!("{:?}", slot),
                        name: item.name().to_string(),
                        id: item.id(),
                    })
                })
                .collect();

            SolveResult {
                items,
                url: format!(
                    "https://hppeng-wynn.github.io/builder/#{}",
                    url_hash
                ),
                score: r.score,
                hp: r.stats.hp,
                ehp: r.stats.ehp,
                assigned_sp_total: r.stats.sp_assignment.total_assigned(),
            }
        })
        .collect();

    Ok(Json(SolveResponse {
        results: response_results,
    }))
}

fn parse_slot(s: &str) -> Option<Slot> {
    match s.to_lowercase().as_str() {
        "helmet" => Some(Slot::Helmet),
        "chestplate" | "chest" => Some(Slot::Chestplate),
        "leggings" | "legs" => Some(Slot::Leggings),
        "boots" => Some(Slot::Boots),
        "ring1" => Some(Slot::Ring1),
        "ring2" => Some(Slot::Ring2),
        "bracelet" => Some(Slot::Bracelet),
        "necklace" => Some(Slot::Necklace),
        "weapon" => Some(Slot::Weapon),
        _ => None,
    }
}

fn parse_objective(s: &str) -> Option<Objective> {
    let s = s.to_lowercase();
    let (dir, stat) = if let Some(rest) = s.strip_prefix("max_") {
        ("max", rest)
    } else if let Some(rest) = s.strip_prefix("min_") {
        ("min", rest)
    } else {
        ("max", s.as_str())
    };

    let stat_type = match stat {
        "hp" => StatType::Hp,
        "ehp" => StatType::Ehp,
        "hpr" => StatType::Hpr,
        "mana_regen" | "mr" => StatType::ManaRegen,
        "life_steal" | "ls" => StatType::LifeSteal,
        "spell_damage" | "sd" => StatType::SpellDamageRaw,
        "main_attack" | "md" => StatType::MainAttackDamageRaw,
        "walk_speed" | "spd" => StatType::WalkSpeed,
        "earth_defence" | "edef" => StatType::EarthDefence,
        "thunder_defence" | "tdef" => StatType::ThunderDefence,
        "water_defence" | "wdef" => StatType::WaterDefence,
        "fire_defence" | "fdef" => StatType::FireDefence,
        "air_defence" | "adef" => StatType::AirDefence,
        _ => return None,
    };

    Some(match dir {
        "min" => Objective::Minimise(stat_type),
        _ => Objective::Maximise(stat_type),
    })
}
