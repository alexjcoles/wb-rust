use serde::{Deserialize, Serialize};

/// The five Wynncraft elements, also used for skill points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Element {
    Earth,
    Thunder,
    Water,
    Fire,
    Air,
}

impl Element {
    pub const ALL: [Element; 5] = [
        Element::Earth,
        Element::Thunder,
        Element::Water,
        Element::Fire,
        Element::Air,
    ];

    pub fn index(self) -> usize {
        match self {
            Element::Earth => 0,
            Element::Thunder => 1,
            Element::Water => 2,
            Element::Fire => 3,
            Element::Air => 4,
        }
    }
}

/// Five-element array indexed by Element.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementalValues<T> {
    pub earth: T,
    pub thunder: T,
    pub water: T,
    pub fire: T,
    pub air: T,
}

impl<T: Copy> ElementalValues<T> {
    pub fn get(&self, elem: Element) -> T {
        match elem {
            Element::Earth => self.earth,
            Element::Thunder => self.thunder,
            Element::Water => self.water,
            Element::Fire => self.fire,
            Element::Air => self.air,
        }
    }

    pub fn set(&mut self, elem: Element, val: T) {
        match elem {
            Element::Earth => self.earth = val,
            Element::Thunder => self.thunder = val,
            Element::Water => self.water = val,
            Element::Fire => self.fire = val,
            Element::Air => self.air = val,
        }
    }

    pub fn as_array(&self) -> [T; 5] {
        [self.earth, self.thunder, self.water, self.fire, self.air]
    }

    pub fn from_array(arr: [T; 5]) -> Self {
        Self {
            earth: arr[0],
            thunder: arr[1],
            water: arr[2],
            fire: arr[3],
            air: arr[4],
        }
    }
}

impl ElementalValues<i32> {
    pub fn sum(&self) -> i32 {
        self.earth + self.thunder + self.water + self.fire + self.air
    }
}

/// Skill point assignment (strength, dexterity, intelligence, defence, agility).
/// These map 1:1 to elements.
pub type SkillPoints = ElementalValues<i32>;

/// Elemental defence values.
pub type ElementalDefence = ElementalValues<i32>;

/// Elemental damage percentage bonuses.
pub type ElementalDamagePct = ElementalValues<i32>;

/// A min-max damage range for a single element.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct DamageRange {
    pub min: f64,
    pub max: f64,
}

impl DamageRange {
    pub fn average(&self) -> f64 {
        (self.min + self.max) / 2.0
    }
}

/// Damage ranges for all 6 damage types (neutral + 5 elements).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Damages {
    pub neutral: DamageRange,
    pub earth: DamageRange,
    pub thunder: DamageRange,
    pub water: DamageRange,
    pub fire: DamageRange,
    pub air: DamageRange,
}

/// Attack speed tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttackSpeed {
    SuperSlow,
    VerySlow,
    Slow,
    Normal,
    Fast,
    VeryFast,
    SuperFast,
}

impl AttackSpeed {
    /// Speed multiplier used in damage calculations.
    pub fn multiplier(self) -> f64 {
        match self {
            AttackSpeed::SuperSlow => 0.51,
            AttackSpeed::VerySlow => 0.83,
            AttackSpeed::Slow => 1.5,
            AttackSpeed::Normal => 2.05,
            AttackSpeed::Fast => 2.5,
            AttackSpeed::VeryFast => 3.1,
            AttackSpeed::SuperFast => 4.3,
        }
    }
}

/// Player class / weapon type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Class {
    Warrior, // Spear
    Archer,  // Bow
    Mage,    // Wand
    Assassin, // Dagger
    Shaman,  // Relik
}

impl Class {
    /// Defence multiplier applied in EHP calculation.
    /// Lower = more tanky (warrior/assassin take normal damage, others take reduced).
    pub fn defence_multiplier(self) -> f64 {
        match self {
            Class::Warrior => 1.0,
            Class::Assassin => 1.0,
            Class::Mage => 0.80,
            Class::Archer => 0.70,
            Class::Shaman => 0.60,
        }
    }
}

/// Weapon type, maps to Class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponType {
    Spear,
    Bow,
    Wand,
    Dagger,
    Relik,
}

impl WeaponType {
    pub fn class(self) -> Class {
        match self {
            WeaponType::Spear => Class::Warrior,
            WeaponType::Bow => Class::Archer,
            WeaponType::Wand => Class::Mage,
            WeaponType::Dagger => Class::Assassin,
            WeaponType::Relik => Class::Shaman,
        }
    }
}

/// An item stat that can be either fixed or have a roll range.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RollableValue {
    Fixed(i32),
    Range { min: i32, max: i32, raw: i32 },
}

impl RollableValue {
    /// Get the maximum possible value (used for optimistic calculations).
    pub fn max_value(&self) -> i32 {
        match self {
            RollableValue::Fixed(v) => *v,
            RollableValue::Range { max, .. } => *max,
        }
    }

    /// Get the minimum possible value (used for pessimistic calculations).
    pub fn min_value(&self) -> i32 {
        match self {
            RollableValue::Fixed(v) => *v,
            RollableValue::Range { min, .. } => *min,
        }
    }

    /// Get the base (raw/unrolled) value.
    pub fn raw_value(&self) -> i32 {
        match self {
            RollableValue::Fixed(v) => *v,
            RollableValue::Range { raw, .. } => *raw,
        }
    }
}

/// Powder element and tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Powder {
    pub element: Element,
    pub tier: u8, // 1-6
}

impl Powder {
    /// Encode to powder ID: element_index * num_tiers + (tier - 1).
    pub fn to_id(&self, num_tiers: u8) -> u32 {
        self.element.index() as u32 * num_tiers as u32 + (self.tier - 1) as u32
    }

    /// Decode from powder ID.
    pub fn from_id(id: u32, num_tiers: u8) -> Self {
        let element = Element::ALL[(id / num_tiers as u32) as usize];
        let tier = (id % num_tiers as u32) as u8 + 1;
        Self { element, tier }
    }
}
