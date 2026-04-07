use wynn_core::calculate::BuildStats;

use crate::constraints::{Objective, StatType};

/// Score a build against a set of objectives.
/// Higher score = better build.
pub fn score_build(stats: &BuildStats, objectives: &[Objective]) -> f64 {
    let mut total = 0.0;

    for objective in objectives {
        let (stat_type, sign) = match objective {
            Objective::Maximise(s) => (s, 1.0),
            Objective::Minimise(s) => (s, -1.0),
        };

        let value = get_stat_value(stats, *stat_type);
        total += sign * value;
    }

    total
}

fn get_stat_value(stats: &BuildStats, stat: StatType) -> f64 {
    match stat {
        StatType::Hp => stats.hp as f64,
        StatType::Ehp => stats.ehp,
        StatType::Hpr => stats.hpr as f64,
        StatType::ManaRegen => stats.mana_regen as f64,
        StatType::LifeSteal => stats.life_steal as f64,
        StatType::SpellDamageRaw => stats.spell_damage_raw as f64,
        StatType::MainAttackDamageRaw => stats.main_attack_damage_raw as f64,
        StatType::WalkSpeed => stats.walk_speed as f64,
        StatType::EarthDefence => stats.elemental_defence.earth as f64,
        StatType::ThunderDefence => stats.elemental_defence.thunder as f64,
        StatType::WaterDefence => stats.elemental_defence.water as f64,
        StatType::FireDefence => stats.elemental_defence.fire as f64,
        StatType::AirDefence => stats.elemental_defence.air as f64,
    }
}
