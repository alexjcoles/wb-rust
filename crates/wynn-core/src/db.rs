use std::collections::HashMap;
use std::path::Path;

use crate::item::{Apparel, Identifications, Item, ItemCategory, ItemTier, Weapon};
use crate::stats::{
    AttackSpeed, DamageRange, Damages, ElementalValues, RollableValue, WeaponType,
};

/// The item database, loaded from hppeng-wynn JSON data.
#[derive(Debug, Clone)]
pub struct ItemDb {
    /// All items indexed by their numeric ID.
    pub items_by_id: HashMap<u32, Item>,
    /// Items indexed by name (lowercase).
    pub items_by_name: HashMap<String, u32>,
    /// Apparels grouped by category, for solver queries.
    pub apparels_by_category: HashMap<ItemCategory, Vec<u32>>,
    /// All weapon IDs.
    pub weapon_ids: Vec<u32>,
}

impl ItemDb {
    /// Load the item database from a JSON file.
    /// Expects the compress.json / clean.json format from hppeng-wynn:
    /// `{"items": [...], "version": 2.0, "sets": {...}}`
    pub fn load_from_file(path: &Path) -> Result<Self, DbError> {
        let data =
            std::fs::read_to_string(path).map_err(|e| DbError::Io(e.to_string()))?;
        Self::load_from_str(&data)
    }

    /// Load from a JSON string.
    pub fn load_from_str(json: &str) -> Result<Self, DbError> {
        let raw: serde_json::Value =
            serde_json::from_str(json).map_err(|e| DbError::Parse(e.to_string()))?;

        let mut db = ItemDb {
            items_by_id: HashMap::new(),
            items_by_name: HashMap::new(),
            apparels_by_category: HashMap::new(),
            weapon_ids: Vec::new(),
        };

        // Format: {"items": [...], "version": ..., "sets": {...}}
        let items = raw
            .get("items")
            .and_then(|v| v.as_array())
            .ok_or_else(|| DbError::Parse("expected top-level 'items' array".into()))?;

        for value in items {
            let name = value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("<unnamed>");
            if let Err(e) = db.parse_item(value) {
                tracing::warn!("skipping item {name}: {e}");
            }
        }

        tracing::info!(
            "loaded {} items ({} apparels, {} weapons)",
            db.items_by_id.len(),
            db.apparels_by_category
                .values()
                .map(|v| v.len())
                .sum::<usize>(),
            db.weapon_ids.len(),
        );

        Ok(db)
    }

    fn parse_item(&mut self, value: &serde_json::Value) -> Result<(), String> {
        let obj = value.as_object().ok_or("not an object")?;

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("missing name")?
            .to_string();

        let id = obj
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or("missing id")? as u32;

        let item_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let tier = parse_tier(
            obj.get("tier")
                .and_then(|v| v.as_str())
                .unwrap_or("Normal"),
        );

        let level = get_i32(obj, "lvl").unwrap_or(0) as u32;
        let powder_slots = get_i32(obj, "slots").unwrap_or(0) as u32;
        let hp = get_i32(obj, "hp").unwrap_or(0);

        let fix_id = obj
            .get("fixID")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // SP requirements are at top level: strReq, dexReq, intReq, defReq, agiReq
        let requirements = ElementalValues {
            earth: get_i32(obj, "strReq").unwrap_or(0),
            thunder: get_i32(obj, "dexReq").unwrap_or(0),
            water: get_i32(obj, "intReq").unwrap_or(0),
            fire: get_i32(obj, "defReq").unwrap_or(0),
            air: get_i32(obj, "agiReq").unwrap_or(0),
        };

        // SP bonuses are at top level: str, dex, int, def, agi
        let skill_point_bonus = ElementalValues {
            earth: get_i32(obj, "str").unwrap_or(0),
            thunder: get_i32(obj, "dex").unwrap_or(0),
            water: get_i32(obj, "int").unwrap_or(0),
            fire: get_i32(obj, "def").unwrap_or(0),
            air: get_i32(obj, "agi").unwrap_or(0),
        };

        // Base elemental defence at top level: eDef, tDef, wDef, fDef, aDef
        let defence_bonus = ElementalValues {
            earth: get_i32(obj, "eDef").unwrap_or(0),
            thunder: get_i32(obj, "tDef").unwrap_or(0),
            water: get_i32(obj, "wDef").unwrap_or(0),
            fire: get_i32(obj, "fDef").unwrap_or(0),
            air: get_i32(obj, "aDef").unwrap_or(0),
        };

        let identifications = parse_identifications(obj, fix_id);

        let category = match item_type.to_lowercase().as_str() {
            "helmet" => Some(ItemCategory::Helmet),
            "chestplate" => Some(ItemCategory::Chestplate),
            "leggings" => Some(ItemCategory::Leggings),
            "boots" => Some(ItemCategory::Boots),
            "ring" => Some(ItemCategory::Ring),
            "bracelet" => Some(ItemCategory::Bracelet),
            "necklace" => Some(ItemCategory::Necklace),
            "wand" | "bow" | "dagger" | "spear" | "relik" => Some(ItemCategory::Weapon),
            _ => None,
        };

        let category = category.ok_or_else(|| format!("unknown type: {item_type}"))?;

        if category == ItemCategory::Weapon {
            let weapon_type = match item_type.to_lowercase().as_str() {
                "wand" => WeaponType::Wand,
                "bow" => WeaponType::Bow,
                "dagger" => WeaponType::Dagger,
                "spear" => WeaponType::Spear,
                "relik" => WeaponType::Relik,
                _ => return Err(format!("unknown weapon type: {item_type}")),
            };

            let attack_speed = obj
                .get("atkSpd")
                .and_then(|v| v.as_str())
                .map(parse_attack_speed)
                .unwrap_or(AttackSpeed::Normal);

            let damage = parse_damages(obj);

            let weapon = Weapon {
                name: name.clone(),
                id,
                tier,
                weapon_type,
                level,
                powder_slots,
                hp,
                attack_speed,
                damage,
                requirements,
                skill_point_bonus,
                defence_bonus,
                identifications,
                fixed_id: fix_id,
            };

            self.items_by_id.insert(id, Item::Weapon(weapon));
            self.weapon_ids.push(id);
        } else {
            // HP bonus for armor is in hpBonus, base hp is in hp
            let total_hp = hp + get_i32(obj, "hpBonus").unwrap_or(0);

            let apparel = Apparel {
                name: name.clone(),
                id,
                tier,
                category,
                level,
                powder_slots,
                hp: total_hp,
                requirements,
                skill_point_bonus,
                defence_bonus,
                identifications,
                fixed_id: fix_id,
            };

            self.apparels_by_category
                .entry(category)
                .or_default()
                .push(id);
            self.items_by_id.insert(id, Item::Apparel(apparel));
        }

        self.items_by_name.insert(name.to_lowercase(), id);
        Ok(())
    }

    /// Look up an item by its numeric ID.
    pub fn get_by_id(&self, id: u32) -> Option<&Item> {
        self.items_by_id.get(&id)
    }

    /// Look up an item by name (case-insensitive).
    pub fn get_by_name(&self, name: &str) -> Option<&Item> {
        self.items_by_name
            .get(&name.to_lowercase())
            .and_then(|id| self.items_by_id.get(id))
    }

    /// Get all apparels in a given category.
    pub fn apparels_in_category(&self, category: ItemCategory) -> Vec<&Apparel> {
        self.apparels_by_category
            .get(&category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| match self.items_by_id.get(id) {
                        Some(Item::Apparel(a)) => Some(a),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Fetch the latest item data from hppeng-wynn GitHub repo.
pub async fn fetch_item_data(dest: &Path) -> Result<(), DbError> {
    let url = "https://raw.githubusercontent.com/hppeng-wynn/hppeng-wynn.github.io/dev/compress.json";
    tracing::info!("fetching item data from {url}");

    let resp = reqwest::get(url)
        .await
        .map_err(|e| DbError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(DbError::Network(format!("HTTP {}", resp.status())));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| DbError::Network(e.to_string()))?;

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DbError::Io(e.to_string()))?;
    }
    std::fs::write(dest, &body).map_err(|e| DbError::Io(e.to_string()))?;

    tracing::info!("saved item data to {}", dest.display());
    Ok(())
}

// --- Parsing helpers ---

fn get_i32(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<i32> {
    obj.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

fn parse_tier(s: &str) -> ItemTier {
    match s {
        "Unique" => ItemTier::Unique,
        "Rare" => ItemTier::Rare,
        "Legendary" => ItemTier::Legendary,
        "Fabled" => ItemTier::Fabled,
        "Mythic" => ItemTier::Mythic,
        "Set" => ItemTier::Set,
        "Crafted" => ItemTier::Crafted,
        _ => ItemTier::Normal,
    }
}

fn parse_attack_speed(s: &str) -> AttackSpeed {
    match s {
        "SUPER_SLOW" => AttackSpeed::SuperSlow,
        "VERY_SLOW" => AttackSpeed::VerySlow,
        "SLOW" => AttackSpeed::Slow,
        "NORMAL" => AttackSpeed::Normal,
        "FAST" => AttackSpeed::Fast,
        "VERY_FAST" => AttackSpeed::VeryFast,
        "SUPER_FAST" => AttackSpeed::SuperFast,
        _ => AttackSpeed::Normal,
    }
}

/// Parse identifications from the compressed item format.
/// All stats are at the top level as abbreviated field names.
fn parse_identifications(
    obj: &serde_json::Map<String, serde_json::Value>,
    fix_id: bool,
) -> Identifications {
    let rv = |key: &str| -> Option<RollableValue> {
        let val = get_i32(obj, key)?;
        if val == 0 {
            return None;
        }
        Some(if fix_id {
            RollableValue::Fixed(val)
        } else {
            // For rollable IDs, compute min/max from base value.
            // Positive IDs: min = base * 0.3 (rounded), max = base * 1.3 (rounded)
            // Negative IDs: min = base * 1.3 (rounded), max = base * 0.7 (rounded)
            // However, the compress format already stores the base value.
            // For our purposes, we'll use Fixed since we're working with
            // max-rolled builds (optimistic calculation).
            RollableValue::Fixed(val)
        })
    };

    Identifications {
        health_regen_raw: rv("hprRaw"),
        health_regen_pct: rv("hprPct"),
        life_steal: rv("ls"),
        mana_regen: rv("mr"),
        mana_steal: rv("ms"),
        spell_damage_raw: rv("sdRaw"),
        spell_damage_pct: rv("sdPct"),
        main_attack_damage_raw: rv("mdRaw"),
        main_attack_damage_pct: rv("mdPct"),
        walk_speed: rv("spd"),
        earth_damage_pct: rv("eDamPct"),
        thunder_damage_pct: rv("tDamPct"),
        water_damage_pct: rv("wDamPct"),
        fire_damage_pct: rv("fDamPct"),
        air_damage_pct: rv("aDamPct"),
        neutral_damage_pct: rv("nDamPct"),
        earth_defence_pct: rv("eDefPct"),
        thunder_defence_pct: rv("tDefPct"),
        water_defence_pct: rv("wDefPct"),
        fire_defence_pct: rv("fDefPct"),
        air_defence_pct: rv("aDefPct"),
        exp_bonus: rv("xpb"),
        loot_bonus: rv("lb"),
        spell_cost_1_raw: rv("spRaw1"),
        spell_cost_1_pct: rv("spPct1"),
        spell_cost_2_raw: rv("spRaw2"),
        spell_cost_2_pct: rv("spPct2"),
        spell_cost_3_raw: rv("spRaw3"),
        spell_cost_3_pct: rv("spPct3"),
        spell_cost_4_raw: rv("spRaw4"),
        spell_cost_4_pct: rv("spPct4"),
    }
}

/// Parse weapon damage ranges from "min-max" string format.
fn parse_damages(obj: &serde_json::Map<String, serde_json::Value>) -> Damages {
    Damages {
        neutral: parse_dam_range(obj, "nDam"),
        earth: parse_dam_range(obj, "eDam"),
        thunder: parse_dam_range(obj, "tDam"),
        water: parse_dam_range(obj, "wDam"),
        fire: parse_dam_range(obj, "fDam"),
        air: parse_dam_range(obj, "aDam"),
    }
}

fn parse_dam_range(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> DamageRange {
    match obj.get(key).and_then(|v| v.as_str()) {
        Some(s) => {
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() == 2 {
                DamageRange {
                    min: parts[0].parse().unwrap_or(0.0),
                    max: parts[1].parse().unwrap_or(0.0),
                }
            } else {
                DamageRange::default()
            }
        }
        None => DamageRange::default(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("network error: {0}")]
    Network(String),
}
