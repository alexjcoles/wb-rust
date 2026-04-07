use serde::{Deserialize, Serialize};

use crate::item::{Apparel, Item, Slot, Weapon};
use crate::stats::{Class, Powder, SkillPoints};

/// A complete Wynncraft build: 8 equipment slots + weapon + metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Build {
    pub helmet: Option<Apparel>,
    pub chestplate: Option<Apparel>,
    pub leggings: Option<Apparel>,
    pub boots: Option<Apparel>,
    pub ring1: Option<Apparel>,
    pub ring2: Option<Apparel>,
    pub bracelet: Option<Apparel>,
    pub necklace: Option<Apparel>,
    pub weapon: Option<Weapon>,

    pub level: u32,
    pub available_points: u32,

    /// Assigned skill points (what the player manually allocates).
    pub assigned_sp: Option<SkillPoints>,

    /// Powders per powderable slot, in encoding order:
    /// [helmet, chestplate, leggings, boots, weapon]
    pub powders: [Vec<Powder>; 5],
}

impl Build {
    pub fn new() -> Self {
        Self {
            helmet: None,
            chestplate: None,
            leggings: None,
            boots: None,
            ring1: None,
            ring2: None,
            bracelet: None,
            necklace: None,
            weapon: None,
            level: 106,
            available_points: 200,
            assigned_sp: None,
            powders: Default::default(),
        }
    }

    /// Get the class from the equipped weapon, if any.
    pub fn class(&self) -> Option<Class> {
        self.weapon.as_ref().map(|w| w.weapon_type.class())
    }

    /// Get apparel in a given slot.
    pub fn apparel(&self, slot: Slot) -> Option<&Apparel> {
        match slot {
            Slot::Helmet => self.helmet.as_ref(),
            Slot::Chestplate => self.chestplate.as_ref(),
            Slot::Leggings => self.leggings.as_ref(),
            Slot::Boots => self.boots.as_ref(),
            Slot::Ring1 => self.ring1.as_ref(),
            Slot::Ring2 => self.ring2.as_ref(),
            Slot::Bracelet => self.bracelet.as_ref(),
            Slot::Necklace => self.necklace.as_ref(),
            Slot::Weapon => None,
        }
    }

    /// Get item in a given slot as a unified Item enum.
    pub fn item(&self, slot: Slot) -> Option<Item> {
        match slot {
            Slot::Weapon => self.weapon.clone().map(Item::Weapon),
            other => self.apparel(other).cloned().map(Item::Apparel),
        }
    }

    /// Iterate over all equipped apparel (non-weapon slots).
    pub fn apparels(&self) -> impl Iterator<Item = &Apparel> {
        [
            self.helmet.as_ref(),
            self.chestplate.as_ref(),
            self.leggings.as_ref(),
            self.boots.as_ref(),
            self.ring1.as_ref(),
            self.ring2.as_ref(),
            self.bracelet.as_ref(),
            self.necklace.as_ref(),
        ]
        .into_iter()
        .flatten()
    }

    /// Iterate over all equipped items (apparel + weapon).
    pub fn all_items(&self) -> impl Iterator<Item = crate::item::Item> + '_ {
        Slot::ALL.iter().filter_map(|&slot| self.item(slot))
    }

    /// Set apparel in a given slot.
    pub fn set_apparel(&mut self, slot: Slot, apparel: Option<Apparel>) {
        match slot {
            Slot::Helmet => self.helmet = apparel,
            Slot::Chestplate => self.chestplate = apparel,
            Slot::Leggings => self.leggings = apparel,
            Slot::Boots => self.boots = apparel,
            Slot::Ring1 => self.ring1 = apparel,
            Slot::Ring2 => self.ring2 = apparel,
            Slot::Bracelet => self.bracelet = apparel,
            Slot::Necklace => self.necklace = apparel,
            Slot::Weapon => {} // use build.weapon directly
        }
    }

    /// Base HP from player level (approximate formula used by WynnBuilder).
    pub fn base_hp(&self) -> i32 {
        // WynnBuilder formula: 5 * level + 5 * level * level (simplified)
        // Actual formula from hppeng: varies by level bracket
        // For level 106: base_hp = 500 + 5 * level (close enough for now, will refine)
        // TODO: verify exact formula from WynnBuilder source
        5 * self.level as i32 + 5
    }
}

impl Default for Build {
    fn default() -> Self {
        Self::new()
    }
}
