use crate::build::Build;
use crate::skill_points::{calculate_sp_assignment, skill_points_to_percentage, SpAssignment};
use crate::stats::{Class, Element, ElementalValues, RollableValue, SkillPoints};

/// Fully calculated stats for a build.
#[derive(Debug, Clone)]
pub struct BuildStats {
    // Survivability
    pub hp: i32,
    pub ehp: f64,
    pub hpr_raw: i32,
    pub hpr_pct: i32,
    pub hpr: i32,
    pub life_steal: i32,

    // Sustain
    pub mana_regen: i32,
    pub mana_steal: i32,

    // Offence
    pub spell_damage_raw: i32,
    pub spell_damage_pct: i32,
    pub main_attack_damage_raw: i32,
    pub main_attack_damage_pct: i32,

    // Mobility
    pub walk_speed: i32,

    // Elemental
    pub elemental_defence: ElementalValues<i32>,
    pub elemental_damage_pct: ElementalValues<i32>,

    // Skill points
    pub sp_assignment: SpAssignment,

    // Secondary
    pub exp_bonus: i32,
}

/// Calculate all stats for a build.
pub fn calculate_build_stats(build: &Build) -> BuildStats {
    let apparels: Vec<_> = build.apparels().collect();

    let (weapon_req, weapon_add) = build
        .weapon
        .as_ref()
        .map(|w| (w.requirements, w.skill_point_bonus))
        .unwrap_or_default();

    let sp_assignment = if let Some(assigned) = &build.assigned_sp {
        // Use the manually specified SP
        let mut total = *assigned;
        for apparel in &apparels {
            for elem in Element::ALL {
                *elem_mut(&mut total, elem) += apparel.skill_point_bonus.get(elem);
            }
        }
        for elem in Element::ALL {
            *elem_mut(&mut total, elem) += weapon_add.get(elem);
        }
        SpAssignment {
            assigned: *assigned,
            total,
        }
    } else {
        calculate_sp_assignment(&apparels, &weapon_req, &weapon_add)
    };

    // Sum HP
    let mut hp = build.base_hp();
    for item in build.all_items() {
        hp += item.hp();
    }

    // Sum identifications
    let mut hpr_raw = 0i32;
    let mut hpr_pct = 0i32;
    let mut life_steal = 0i32;
    let mut mana_regen = 0i32;
    let mut mana_steal = 0i32;
    let mut spell_damage_raw = 0i32;
    let mut spell_damage_pct = 0i32;
    let mut main_attack_damage_raw = 0i32;
    let mut main_attack_damage_pct = 0i32;
    let mut walk_speed = 0i32;
    let mut exp_bonus = 0i32;
    let mut elem_dam_pct = ElementalValues::<i32>::default();
    let mut elem_def_flat = ElementalValues::<i32>::default();
    let mut elem_def_pct = ElementalValues::<i32>::default();

    for item in build.all_items() {
        let ids = item.identifications();

        hpr_raw += id_max_val(&ids.health_regen_raw);
        hpr_pct += id_max_val(&ids.health_regen_pct);
        life_steal += id_max_val(&ids.life_steal);
        mana_regen += id_max_val(&ids.mana_regen);
        mana_steal += id_max_val(&ids.mana_steal);
        spell_damage_raw += id_max_val(&ids.spell_damage_raw);
        spell_damage_pct += id_max_val(&ids.spell_damage_pct);
        main_attack_damage_raw += id_max_val(&ids.main_attack_damage_raw);
        main_attack_damage_pct += id_max_val(&ids.main_attack_damage_pct);
        walk_speed += id_max_val(&ids.walk_speed);
        exp_bonus += id_max_val(&ids.exp_bonus);

        // Elemental damage %
        for (elem, val) in [
            (Element::Earth, &ids.earth_damage_pct),
            (Element::Thunder, &ids.thunder_damage_pct),
            (Element::Water, &ids.water_damage_pct),
            (Element::Fire, &ids.fire_damage_pct),
            (Element::Air, &ids.air_damage_pct),
        ] {
            *elem_mut(&mut elem_dam_pct, elem) += id_max_val(val);
        }

        // Elemental defence flat
        let def = item.defence_bonus();
        for elem in Element::ALL {
            *elem_mut(&mut elem_def_flat, elem) += def.get(elem);
        }

        // Elemental defence %
        for (elem, val) in [
            (Element::Earth, &ids.earth_defence_pct),
            (Element::Thunder, &ids.thunder_defence_pct),
            (Element::Water, &ids.water_defence_pct),
            (Element::Fire, &ids.fire_defence_pct),
            (Element::Air, &ids.air_defence_pct),
        ] {
            *elem_mut(&mut elem_def_pct, elem) += id_max_val(val);
        }
    }

    // Compute effective defence: flat + (pct * |flat|) / 100
    let mut elemental_defence = ElementalValues::<i32>::default();
    for elem in Element::ALL {
        let flat = elem_def_flat.get(elem);
        let pct = elem_def_pct.get(elem);
        let effective = flat + (pct * flat.abs()) / 100;
        elemental_defence.set(elem, effective);
    }

    // Compute HPR: raw + (pct * |raw|) / 100
    let hpr = hpr_raw + (hpr_pct * hpr_raw.abs()) / 100;

    // Compute EHP
    let class = build.class().unwrap_or(Class::Warrior);
    let ehp = calculate_ehp(&sp_assignment.total, hp, &class);

    BuildStats {
        hp,
        ehp,
        hpr_raw,
        hpr_pct,
        hpr,
        life_steal,
        mana_regen,
        mana_steal,
        spell_damage_raw,
        spell_damage_pct,
        main_attack_damage_raw,
        main_attack_damage_pct,
        walk_speed,
        elemental_defence,
        elemental_damage_pct: elem_dam_pct,
        sp_assignment,
        exp_bonus,
    }
}

/// Calculate Effective HP.
/// Formula from WynnBuilderTools:
/// ehp = hp / ((0.1 * agi_pct + (1 - agi_pct) * (1 - def_pct)) * (2 - class_multi))
fn calculate_ehp(total_sp: &SkillPoints, hp: i32, class: &Class) -> f64 {
    let def_pct = skill_points_to_percentage(total_sp.fire) * 0.867;
    let agi_pct = skill_points_to_percentage(total_sp.air) * 0.951;

    let damage_taken = 0.1 * agi_pct + (1.0 - agi_pct) * (1.0 - def_pct);
    let class_factor = 2.0 - class.defence_multiplier();

    hp as f64 / (damage_taken * class_factor)
}

fn id_max_val(val: &Option<RollableValue>) -> i32 {
    val.map(|v| v.max_value()).unwrap_or(0)
}

fn elem_mut(vals: &mut ElementalValues<i32>, elem: Element) -> &mut i32 {
    match elem {
        Element::Earth => &mut vals.earth,
        Element::Thunder => &mut vals.thunder,
        Element::Water => &mut vals.water,
        Element::Fire => &mut vals.fire,
        Element::Air => &mut vals.air,
    }
}
