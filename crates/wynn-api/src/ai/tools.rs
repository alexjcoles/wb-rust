use std::sync::Arc;

use serde_json::{json, Value};
use wynn_core::analyze::analyze_build as do_analyze;
use wynn_core::calculate::calculate_build_stats;
use wynn_core::db::ItemDb;
use wynn_core::item::Slot;
use wynn_solver::constraints::{Objective, StatType};
use wynn_solver::search::{solve, SolverConfig};

use super::provider::{ToolCall, ToolResult};

/// Execute a tool call against the item DB and return the result.
pub fn execute_tool(call: &ToolCall, db: &Arc<ItemDb>) -> ToolResult {
    let result = match call.name.as_str() {
        "parse_build" => tool_parse_build(&call.arguments, db),
        "analyze_build" => tool_analyze_build(&call.arguments, db),
        "find_candidate_items" => tool_find_candidates(&call.arguments, db),
        "solve_build" => tool_solve_build(&call.arguments, db),
        _ => Err(format!("unknown tool: {}", call.name)),
    };

    ToolResult {
        name: call.name.clone(),
        result: match result {
            Ok(v) => v,
            Err(e) => json!({ "error": e }),
        },
    }
}

fn tool_parse_build(args: &Value, db: &ItemDb) -> Result<Value, String> {
    let url = args["url"].as_str().ok_or("missing 'url' argument")?;
    let build = wynn_encoding::decode_build(url, db).map_err(|e| e.to_string())?;
    let stats = calculate_build_stats(&build);

    let items: Vec<Value> = Slot::ALL
        .iter()
        .filter_map(|&slot| {
            build.item(slot).map(|item| {
                json!({
                    "slot": format!("{:?}", slot),
                    "name": item.name(),
                    "id": item.id(),
                    "level": item.level(),
                })
            })
        })
        .collect();

    Ok(json!({
        "items": items,
        "level": build.level,
        "stats": {
            "hp": stats.hp,
            "ehp": stats.ehp,
            "hpr": stats.hpr,
            "life_steal": stats.life_steal,
            "mana_regen": stats.mana_regen,
            "spell_damage_raw": stats.spell_damage_raw,
            "spell_damage_pct": stats.spell_damage_pct,
            "walk_speed": stats.walk_speed,
            "earth_defence": stats.elemental_defence.earth,
            "thunder_defence": stats.elemental_defence.thunder,
            "water_defence": stats.elemental_defence.water,
            "fire_defence": stats.elemental_defence.fire,
            "air_defence": stats.elemental_defence.air,
            "assigned_sp_total": stats.sp_assignment.total_assigned(),
        }
    }))
}

fn tool_analyze_build(args: &Value, db: &ItemDb) -> Result<Value, String> {
    let url = args["url"].as_str().ok_or("missing 'url' argument")?;
    let build = wynn_encoding::decode_build(url, db).map_err(|e| e.to_string())?;
    let stats = calculate_build_stats(&build);
    let analysis = do_analyze(&stats);

    let issues: Vec<Value> = analysis
        .issues
        .iter()
        .map(|i| {
            json!({
                "severity": format!("{:?}", i.severity),
                "category": format!("{:?}", i.category),
                "description": i.description,
            })
        })
        .collect();

    Ok(json!({
        "archetype": format!("{:?}", analysis.archetype),
        "survivability_score": analysis.survivability_score,
        "dps_score": analysis.dps_score,
        "issues": issues,
    }))
}

fn tool_find_candidates(args: &Value, db: &ItemDb) -> Result<Value, String> {
    let slot_str = args["slot"].as_str().ok_or("missing 'slot' argument")?;
    let category = match slot_str.to_lowercase().as_str() {
        "helmet" => wynn_core::item::ItemCategory::Helmet,
        "chestplate" | "chest" => wynn_core::item::ItemCategory::Chestplate,
        "leggings" | "legs" => wynn_core::item::ItemCategory::Leggings,
        "boots" => wynn_core::item::ItemCategory::Boots,
        "ring" => wynn_core::item::ItemCategory::Ring,
        "bracelet" => wynn_core::item::ItemCategory::Bracelet,
        "necklace" => wynn_core::item::ItemCategory::Necklace,
        _ => return Err(format!("unknown slot: {slot_str}")),
    };

    let min_level = args["min_level"].as_u64().unwrap_or(90) as u32;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;
    let sort_by = args["sort_by"].as_str().unwrap_or("hp");

    let mut items: Vec<_> = db
        .apparels_in_category(category)
        .into_iter()
        .filter(|a| a.level >= min_level)
        .collect();

    // Sort by requested stat
    items.sort_by(|a, b| {
        let val = |item: &wynn_core::item::Apparel| -> i32 {
            match sort_by {
                "hp" => item.hp,
                "thunder_defence" | "tdef" => item.defence_bonus.thunder,
                "earth_defence" | "edef" => item.defence_bonus.earth,
                "water_defence" | "wdef" => item.defence_bonus.water,
                "fire_defence" | "fdef" => item.defence_bonus.fire,
                "air_defence" | "adef" => item.defence_bonus.air,
                _ => item.hp,
            }
        };
        val(b).cmp(&val(a))
    });

    items.truncate(limit);

    let results: Vec<Value> = items
        .iter()
        .map(|a| {
            json!({
                "name": a.name,
                "id": a.id,
                "level": a.level,
                "tier": format!("{:?}", a.tier),
                "hp": a.hp,
                "defence": {
                    "earth": a.defence_bonus.earth,
                    "thunder": a.defence_bonus.thunder,
                    "water": a.defence_bonus.water,
                    "fire": a.defence_bonus.fire,
                    "air": a.defence_bonus.air,
                },
                "sp_req": {
                    "str": a.requirements.earth,
                    "dex": a.requirements.thunder,
                    "int": a.requirements.water,
                    "def": a.requirements.fire,
                    "agi": a.requirements.air,
                },
                "sp_add": {
                    "str": a.skill_point_bonus.earth,
                    "dex": a.skill_point_bonus.thunder,
                    "int": a.skill_point_bonus.water,
                    "def": a.skill_point_bonus.fire,
                    "agi": a.skill_point_bonus.air,
                },
            })
        })
        .collect();

    Ok(json!({ "items": results }))
}

fn tool_solve_build(args: &Value, db: &ItemDb) -> Result<Value, String> {
    let url = args["url"].as_str().ok_or("missing 'url' argument")?;
    let mut build = wynn_encoding::decode_build(url, db).map_err(|e| e.to_string())?;

    build.available_points = args["available_points"].as_u64().unwrap_or(200) as u32;

    let locked_arr = args["locked_slots"]
        .as_array()
        .ok_or("missing 'locked_slots' argument")?;
    let locked: Vec<Slot> = locked_arr
        .iter()
        .filter_map(|v| v.as_str().and_then(parse_slot))
        .collect();

    let objectives: Vec<Objective> = args
        .get("objectives")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().and_then(parse_objective))
                .collect()
        })
        .unwrap_or_default();

    let config = SolverConfig {
        constraints: args
            .get("constraints")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        objectives,
        max_results: args["max_results"].as_u64().unwrap_or(5) as usize,
        min_item_level: args["min_item_level"].as_u64().unwrap_or(90) as u32,
    };

    let results = solve(&build, &locked, &config, db);

    let output: Vec<Value> = results
        .iter()
        .map(|r| {
            let url_hash = wynn_encoding::encode_build(&r.build);
            let items: Vec<Value> = Slot::ALL
                .iter()
                .filter_map(|&slot| {
                    r.build
                        .item(slot)
                        .map(|item| json!({ "slot": format!("{:?}", slot), "name": item.name() }))
                })
                .collect();

            json!({
                "items": items,
                "url": format!("https://hppeng-wynn.github.io/builder/#{url_hash}"),
                "score": r.score,
                "hp": r.stats.hp,
                "ehp": r.stats.ehp,
                "assigned_sp_total": r.stats.sp_assignment.total_assigned(),
            })
        })
        .collect();

    Ok(json!({ "results": output }))
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
    let stat = match s.as_str() {
        "ehp" => StatType::Ehp,
        "hp" => StatType::Hp,
        "hpr" => StatType::Hpr,
        "mana_regen" | "mr" => StatType::ManaRegen,
        "thunder_defence" | "tdef" => StatType::ThunderDefence,
        "earth_defence" | "edef" => StatType::EarthDefence,
        "water_defence" | "wdef" => StatType::WaterDefence,
        "fire_defence" | "fdef" => StatType::FireDefence,
        "air_defence" | "adef" => StatType::AirDefence,
        "walk_speed" | "spd" => StatType::WalkSpeed,
        _ => return None,
    };
    Some(Objective::Maximise(stat))
}

/// Tool definitions for AI providers (JSON schema format).
pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "parse_build",
            "description": "Decode a WynnBuilder URL into item names, stats, and skill point allocation",
            "input_schema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Full WynnBuilder URL including hash" }
                },
                "required": ["url"]
            }
        },
        {
            "name": "analyze_build",
            "description": "Analyze a build to identify archetype, strengths, weaknesses, and survivability score",
            "input_schema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "WynnBuilder URL to analyze" }
                },
                "required": ["url"]
            }
        },
        {
            "name": "find_candidate_items",
            "description": "Search the item database for items in a specific slot, sorted by a stat",
            "input_schema": {
                "type": "object",
                "properties": {
                    "slot": { "type": "string", "enum": ["helmet","chestplate","leggings","boots","ring","bracelet","necklace"] },
                    "min_level": { "type": "integer", "default": 90 },
                    "sort_by": { "type": "string", "description": "Stat to sort by: hp, thunder_defence, earth_defence, etc." },
                    "limit": { "type": "integer", "default": 10 }
                },
                "required": ["slot"]
            }
        },
        {
            "name": "solve_build",
            "description": "Run the constrained solver. Locks specified slots, searches for valid item combinations in flexible slots. Returns top results with WynnBuilder URLs.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Base build WynnBuilder URL" },
                    "locked_slots": { "type": "array", "items": { "type": "string" }, "description": "Slots to keep unchanged" },
                    "objectives": { "type": "array", "items": { "type": "string" }, "description": "Stats to maximise: ehp, hp, thunder_defence, etc." },
                    "constraints": { "type": "object", "description": "Min stat requirements (e.g. {\"min_hp\": 10000})" },
                    "available_points": { "type": "integer", "default": 200 },
                    "max_results": { "type": "integer", "default": 5 },
                    "min_item_level": { "type": "integer", "default": 90 }
                },
                "required": ["url", "locked_slots"]
            }
        }
    ])
}
