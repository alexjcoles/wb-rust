use crate::item::Apparel;

/// Groups of mutually exclusive items.
/// At most 1 item from each group can appear in a build.
pub struct IllegalCombinations {
    pub groups: Vec<Vec<String>>,
}

impl IllegalCombinations {
    /// Default illegal combination groups (Hive sets, elemental sets, etc.).
    pub fn default_groups() -> Self {
        Self {
            groups: vec![
                // Hive set (max 1)
                vec![
                    "Abyss-Touched Helmet", "Abyss-Touched Chestplate",
                    "Abyss-Touched Leggings", "Abyss-Touched Boots",
                    "Hive Helmet", "Hive Chestplate", "Hive Leggings", "Hive Boots",
                    "Infused Hive Relik", "Infused Hive Wand", "Infused Hive Dagger",
                    "Infused Hive Spear", "Infused Hive Bow",
                    "Contrast", "Prowess", "Intensity",
                    "Breezehands", "Flashfire Gauntlet", "Thundersurge",
                ]
                .into_iter().map(String::from).collect(),
                // Thunder set (max 1)
                vec![
                    "Sparkling Visor", "Insulated Plate Mail",
                    "Static-Charged Leggings", "Thunderous Step",
                    "Bottled Thunderstorm", "Storm Breaker",
                ]
                .into_iter().map(String::from).collect(),
                // Air set (max 1)
                vec![
                    "Pride of the Aerie", "Gale's Freedom",
                    "Turbine Greaves", "Flashstep",
                    "Breeze", "Zephyr",
                ]
                .into_iter().map(String::from).collect(),
                // Earth set (max 1)
                vec![
                    "Ambertoise Shell", "Beetle Aegis",
                    "Tough Pants", "Scarab Hide",
                    "Iron Bracelet", "Terra's Mold",
                ]
                .into_iter().map(String::from).collect(),
                // Water set (max 1)
                vec![
                    "Whitecap Crown", "Stillwater Blue",
                    "Waterlogged Leggings", "Tidepool Walkers",
                    "Moon Pool Circlet", "Aquamarine",
                ]
                .into_iter().map(String::from).collect(),
                // Fire set (max 1)
                vec![
                    "Sparkweaver", "Soulflare",
                    "Cinderchain", "Mantlewalkers",
                    "Clockwork", "Duplex",
                ]
                .into_iter().map(String::from).collect(),
                // Ornate Shadow set (max 1)
                vec![
                    "Ornate Shadow Cowl", "Ornate Shadow Garb",
                    "Ornate Shadow Cover", "Ornate Shadow Cloud",
                ]
                .into_iter().map(String::from).collect(),
                // Dragon set (max 1)
                vec![
                    "Dragon's Eye Bracelet", "Draoi Fair", "Renda Langit",
                ]
                .into_iter().map(String::from).collect(),
            ],
        }
    }

    /// Check if a combination of apparels contains an illegal pairing.
    pub fn is_illegal(&self, apparels: &[&Apparel]) -> bool {
        for group in &self.groups {
            let mut count = 0;
            for apparel in apparels {
                if group.contains(&apparel.name) {
                    count += 1;
                    if count > 1 {
                        return true;
                    }
                }
            }
        }
        false
    }
}
