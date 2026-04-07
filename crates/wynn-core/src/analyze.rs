use crate::calculate::BuildStats;
use crate::stats::Element;

/// A weakness or strength identified in a build.
#[derive(Debug, Clone)]
pub struct BuildIssue {
    pub severity: Severity,
    pub category: IssueCategory,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCategory {
    ElementalDefence,
    Survivability,
    Sustain,
    Mobility,
    Offence,
}

/// Archetype classification for a build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archetype {
    /// High spell damage, intelligence-focused
    SpellCaster,
    /// High melee damage, strength-focused
    Melee,
    /// Mix of spell and melee
    Hybrid,
    /// High EHP and HPR, low damage
    Tank,
    /// High walk speed and sustain
    Support,
}

/// Full analysis of a build.
#[derive(Debug, Clone)]
pub struct BuildAnalysis {
    pub archetype: Archetype,
    pub issues: Vec<BuildIssue>,
    pub survivability_score: f64,
    pub dps_score: f64,
}

/// Analyze a build's stats and identify issues.
pub fn analyze_build(stats: &BuildStats) -> BuildAnalysis {
    let mut issues = Vec::new();

    // Elemental defence checks
    let defences = [
        (Element::Earth, stats.elemental_defence.earth),
        (Element::Thunder, stats.elemental_defence.thunder),
        (Element::Water, stats.elemental_defence.water),
        (Element::Fire, stats.elemental_defence.fire),
        (Element::Air, stats.elemental_defence.air),
    ];

    for (elem, val) in &defences {
        if *val < -60 {
            issues.push(BuildIssue {
                severity: Severity::Critical,
                category: IssueCategory::ElementalDefence,
                description: format!(
                    "{:?} defence is {} — high risk of being one-shot by {:?} mobs",
                    elem, val, elem
                ),
            });
        } else if *val < 0 {
            issues.push(BuildIssue {
                severity: Severity::Warning,
                category: IssueCategory::ElementalDefence,
                description: format!(
                    "{:?} defence is {} — vulnerable to {:?} damage",
                    elem, val, elem
                ),
            });
        }
    }

    // HP / EHP checks
    if stats.hp < 5000 {
        issues.push(BuildIssue {
            severity: Severity::Critical,
            category: IssueCategory::Survivability,
            description: format!("HP is only {} — extremely fragile", stats.hp),
        });
    } else if stats.hp < 10000 {
        issues.push(BuildIssue {
            severity: Severity::Warning,
            category: IssueCategory::Survivability,
            description: format!("HP is {} — below average survivability", stats.hp),
        });
    }

    if stats.ehp < 15000.0 {
        issues.push(BuildIssue {
            severity: Severity::Critical,
            category: IssueCategory::Survivability,
            description: format!("EHP is {:.0} — very low effective survivability", stats.ehp),
        });
    } else if stats.ehp < 25000.0 {
        issues.push(BuildIssue {
            severity: Severity::Warning,
            category: IssueCategory::Survivability,
            description: format!("EHP is {:.0} — moderate survivability", stats.ehp),
        });
    }

    // Sustain checks
    if stats.hpr <= 0 && stats.life_steal <= 0 {
        issues.push(BuildIssue {
            severity: Severity::Warning,
            category: IssueCategory::Sustain,
            description: "No health recovery (HPR and life steal both zero or negative)".into(),
        });
    }

    if stats.mana_regen < 4 {
        issues.push(BuildIssue {
            severity: Severity::Warning,
            category: IssueCategory::Sustain,
            description: format!(
                "Mana regen is {} — may struggle to sustain spell casting",
                stats.mana_regen
            ),
        });
    }

    // Mobility
    if stats.walk_speed < -10 {
        issues.push(BuildIssue {
            severity: Severity::Warning,
            category: IssueCategory::Mobility,
            description: format!(
                "Walk speed is {}% — significantly slower, dangerous for dodging",
                stats.walk_speed
            ),
        });
    }

    // Classify archetype
    let archetype = classify_archetype(stats);

    // Score survivability (0-100 scale)
    let survivability_score = score_survivability(stats);

    // Score DPS potential (0-100 scale)
    let dps_score = score_dps(stats);

    // Sort issues by severity (critical first)
    issues.sort_by(|a, b| b.severity.cmp(&a.severity));

    BuildAnalysis {
        archetype,
        issues,
        survivability_score,
        dps_score,
    }
}

fn classify_archetype(stats: &BuildStats) -> Archetype {
    let has_spell = stats.spell_damage_raw > 100 || stats.spell_damage_pct > 30;
    let has_melee = stats.main_attack_damage_raw > 100 || stats.main_attack_damage_pct > 30;
    let is_tanky = stats.ehp > 40000.0 && stats.hpr > 100;

    if is_tanky && !has_spell && !has_melee {
        Archetype::Tank
    } else if stats.mana_regen > 30 && stats.hpr > 50 && !has_spell {
        Archetype::Support
    } else if has_spell && has_melee {
        Archetype::Hybrid
    } else if has_spell {
        Archetype::SpellCaster
    } else if has_melee {
        Archetype::Melee
    } else {
        Archetype::Hybrid
    }
}

fn score_survivability(stats: &BuildStats) -> f64 {
    let mut score = 0.0;

    // EHP component (0-40 points)
    score += (stats.ehp / 1500.0).min(40.0);

    // HP component (0-20 points)
    score += (stats.hp as f64 / 1000.0).min(20.0);

    // HPR component (0-15 points)
    score += (stats.hpr as f64 / 20.0).min(15.0);

    // Life steal component (0-10 points)
    score += (stats.life_steal as f64 / 50.0).min(10.0);

    // Defence penalty (up to -15 points)
    let defences = [
        stats.elemental_defence.earth,
        stats.elemental_defence.thunder,
        stats.elemental_defence.water,
        stats.elemental_defence.fire,
        stats.elemental_defence.air,
    ];
    for def in &defences {
        if *def < 0 {
            score += (*def as f64 / 100.0).max(-3.0); // -3 per badly negative element
        }
    }

    // Walk speed bonus (0-5 points)
    score += (stats.walk_speed as f64 / 20.0).clamp(-5.0, 5.0);

    score.clamp(0.0, 100.0)
}

fn score_dps(stats: &BuildStats) -> f64 {
    let mut score = 0.0;

    // Spell damage (0-50 points)
    score += (stats.spell_damage_raw as f64 / 20.0).min(25.0);
    score += (stats.spell_damage_pct as f64 / 10.0).min(25.0);

    // Melee damage (0-30 points)
    score += (stats.main_attack_damage_raw as f64 / 20.0).min(15.0);
    score += (stats.main_attack_damage_pct as f64 / 10.0).min(15.0);

    // Mana sustain component (0-20 points) — can't DPS without mana
    score += (stats.mana_regen as f64 / 5.0).min(20.0);

    score.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
    }
}
