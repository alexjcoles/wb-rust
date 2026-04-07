use serde::{Deserialize, Serialize};
use wynn_core::calculate::BuildStats;

/// Hard constraints a build must satisfy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Constraints {
    pub min_hp: Option<i32>,
    pub min_ehp: Option<f64>,
    pub min_hpr: Option<i32>,
    pub min_mana_regen: Option<i32>,
    pub min_life_steal: Option<i32>,
    pub min_spell_damage_raw: Option<i32>,
    pub min_walk_speed: Option<i32>,
    pub min_earth_defence: Option<i32>,
    pub min_thunder_defence: Option<i32>,
    pub min_water_defence: Option<i32>,
    pub min_fire_defence: Option<i32>,
    pub min_air_defence: Option<i32>,
}

impl Constraints {
    /// Check if a build's stats satisfy all constraints.
    pub fn is_satisfied(&self, stats: &BuildStats) -> bool {
        let checks: &[(Option<i32>, i32)] = &[
            (self.min_hp, stats.hp),
            (self.min_hpr, stats.hpr),
            (self.min_mana_regen, stats.mana_regen),
            (self.min_life_steal, stats.life_steal),
            (self.min_spell_damage_raw, stats.spell_damage_raw),
            (self.min_walk_speed, stats.walk_speed),
            (self.min_earth_defence, stats.elemental_defence.earth),
            (self.min_thunder_defence, stats.elemental_defence.thunder),
            (self.min_water_defence, stats.elemental_defence.water),
            (self.min_fire_defence, stats.elemental_defence.fire),
            (self.min_air_defence, stats.elemental_defence.air),
        ];

        checks
            .iter()
            .all(|(min, val)| min.map_or(true, |m| *val >= m))
            && self.min_ehp.map_or(true, |m| stats.ehp >= m)
    }
}

/// What to optimise for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Objective {
    Maximise(StatType),
    Minimise(StatType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatType {
    Hp,
    Ehp,
    Hpr,
    ManaRegen,
    LifeSteal,
    SpellDamageRaw,
    MainAttackDamageRaw,
    WalkSpeed,
    EarthDefence,
    ThunderDefence,
    WaterDefence,
    FireDefence,
    AirDefence,
}
