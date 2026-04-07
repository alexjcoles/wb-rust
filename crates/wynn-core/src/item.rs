use serde::{Deserialize, Serialize};

use crate::stats::{AttackSpeed, Damages, ElementalValues, RollableValue, WeaponType};

/// Equipment slot in a build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Slot {
    Helmet,
    Chestplate,
    Leggings,
    Boots,
    Ring1,
    Ring2,
    Bracelet,
    Necklace,
    Weapon,
}

impl Slot {
    /// All 9 slots in encoding order.
    pub const ALL: [Slot; 9] = [
        Slot::Helmet,
        Slot::Chestplate,
        Slot::Leggings,
        Slot::Boots,
        Slot::Ring1,
        Slot::Ring2,
        Slot::Bracelet,
        Slot::Necklace,
        Slot::Weapon,
    ];

    /// Whether this slot can hold powders.
    pub fn is_powderable(self) -> bool {
        matches!(
            self,
            Slot::Helmet | Slot::Chestplate | Slot::Leggings | Slot::Boots | Slot::Weapon
        )
    }

    /// The item category this slot accepts.
    pub fn category(self) -> ItemCategory {
        match self {
            Slot::Helmet => ItemCategory::Helmet,
            Slot::Chestplate => ItemCategory::Chestplate,
            Slot::Leggings => ItemCategory::Leggings,
            Slot::Boots => ItemCategory::Boots,
            Slot::Ring1 | Slot::Ring2 => ItemCategory::Ring,
            Slot::Bracelet => ItemCategory::Bracelet,
            Slot::Necklace => ItemCategory::Necklace,
            Slot::Weapon => ItemCategory::Weapon,
        }
    }
}

/// Item category (armour type, accessory type, or weapon).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemCategory {
    Helmet,
    Chestplate,
    Leggings,
    Boots,
    Ring,
    Bracelet,
    Necklace,
    Weapon,
}

/// Item rarity tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemTier {
    Normal,
    Unique,
    Rare,
    Legendary,
    Fabled,
    Mythic,
    Set,
    Crafted,
}

/// Common identifications (stats) shared by all equipment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Identifications {
    // Survivability
    pub health_regen_raw: Option<RollableValue>,
    pub health_regen_pct: Option<RollableValue>,
    pub life_steal: Option<RollableValue>,

    // Sustain
    pub mana_regen: Option<RollableValue>,
    pub mana_steal: Option<RollableValue>,

    // Offence
    pub spell_damage_raw: Option<RollableValue>,
    pub spell_damage_pct: Option<RollableValue>,
    pub main_attack_damage_raw: Option<RollableValue>,
    pub main_attack_damage_pct: Option<RollableValue>,

    // Mobility
    pub walk_speed: Option<RollableValue>,

    // Elemental damage %
    pub earth_damage_pct: Option<RollableValue>,
    pub thunder_damage_pct: Option<RollableValue>,
    pub water_damage_pct: Option<RollableValue>,
    pub fire_damage_pct: Option<RollableValue>,
    pub air_damage_pct: Option<RollableValue>,
    pub neutral_damage_pct: Option<RollableValue>,

    // Elemental defence %
    pub earth_defence_pct: Option<RollableValue>,
    pub thunder_defence_pct: Option<RollableValue>,
    pub water_defence_pct: Option<RollableValue>,
    pub fire_defence_pct: Option<RollableValue>,
    pub air_defence_pct: Option<RollableValue>,

    // Secondary
    pub exp_bonus: Option<RollableValue>,
    pub loot_bonus: Option<RollableValue>,

    // Spell costs
    pub spell_cost_1_raw: Option<RollableValue>,
    pub spell_cost_1_pct: Option<RollableValue>,
    pub spell_cost_2_raw: Option<RollableValue>,
    pub spell_cost_2_pct: Option<RollableValue>,
    pub spell_cost_3_raw: Option<RollableValue>,
    pub spell_cost_3_pct: Option<RollableValue>,
    pub spell_cost_4_raw: Option<RollableValue>,
    pub spell_cost_4_pct: Option<RollableValue>,
}

/// An armour piece or accessory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Apparel {
    pub name: String,
    pub id: u32,
    pub tier: ItemTier,
    pub category: ItemCategory,
    pub level: u32,
    pub powder_slots: u32,
    pub hp: i32,

    /// Minimum skill point requirements to wear.
    pub requirements: ElementalValues<i32>,
    /// Skill points this item adds when worn.
    pub skill_point_bonus: ElementalValues<i32>,
    /// Flat elemental defence bonuses.
    pub defence_bonus: ElementalValues<i32>,

    pub identifications: Identifications,
    pub fixed_id: bool,
}

/// A weapon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub name: String,
    pub id: u32,
    pub tier: ItemTier,
    pub weapon_type: WeaponType,
    pub level: u32,
    pub powder_slots: u32,
    pub hp: i32,
    pub attack_speed: AttackSpeed,
    pub damage: Damages,

    /// Minimum skill point requirements to use.
    pub requirements: ElementalValues<i32>,
    /// Skill points this weapon adds.
    pub skill_point_bonus: ElementalValues<i32>,
    /// Flat elemental defence bonuses.
    pub defence_bonus: ElementalValues<i32>,

    pub identifications: Identifications,
    pub fixed_id: bool,
}

/// A unified item enum for when we need to handle both types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    Apparel(Apparel),
    Weapon(Weapon),
}

impl Item {
    pub fn name(&self) -> &str {
        match self {
            Item::Apparel(a) => &a.name,
            Item::Weapon(w) => &w.name,
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            Item::Apparel(a) => a.id,
            Item::Weapon(w) => w.id,
        }
    }

    pub fn level(&self) -> u32 {
        match self {
            Item::Apparel(a) => a.level,
            Item::Weapon(w) => w.level,
        }
    }

    pub fn requirements(&self) -> &ElementalValues<i32> {
        match self {
            Item::Apparel(a) => &a.requirements,
            Item::Weapon(w) => &w.requirements,
        }
    }

    pub fn skill_point_bonus(&self) -> &ElementalValues<i32> {
        match self {
            Item::Apparel(a) => &a.skill_point_bonus,
            Item::Weapon(w) => &w.skill_point_bonus,
        }
    }

    pub fn hp(&self) -> i32 {
        match self {
            Item::Apparel(a) => a.hp,
            Item::Weapon(w) => w.hp,
        }
    }

    pub fn identifications(&self) -> &Identifications {
        match self {
            Item::Apparel(a) => &a.identifications,
            Item::Weapon(w) => &w.identifications,
        }
    }

    pub fn defence_bonus(&self) -> &ElementalValues<i32> {
        match self {
            Item::Apparel(a) => &a.defence_bonus,
            Item::Weapon(w) => &w.defence_bonus,
        }
    }
}
