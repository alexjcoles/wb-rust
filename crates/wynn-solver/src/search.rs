use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use rayon::prelude::*;
use wynn_core::build::Build;
use wynn_core::calculate::{calculate_build_stats, BuildStats};
use wynn_core::db::ItemDb;
use wynn_core::hive::IllegalCombinations;
use wynn_core::item::{Apparel, Slot};
use wynn_core::stats::{Element, RollableValue};

use crate::constraints::{Constraints, Objective, StatType};
use crate::scoring::score_build;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A solver result: a valid build with its stats.
#[derive(Debug, Clone)]
pub struct SolverResult {
    pub build: Build,
    pub stats: BuildStats,
    pub score: f64,
}

/// Configuration for the solver.
#[derive(Debug, Clone)]
pub struct SolverConfig {
    pub constraints: Constraints,
    pub objectives: Vec<Objective>,
    pub max_results: usize,
    /// Minimum item level to consider. Set to 0 to include all items.
    pub min_item_level: u32,
    /// Max candidates per slot after diverse selection.
    pub max_candidates_per_slot: usize,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            constraints: Constraints::default(),
            objectives: vec![],
            max_results: 5,
            min_item_level: 0,
            max_candidates_per_slot: 100,
        }
    }
}

// ---------------------------------------------------------------------------
// Precomputed item stats (flat, cache-friendly)
// ---------------------------------------------------------------------------

/// Flat stat summary for a single item, precomputed for fast filtering.
#[derive(Debug, Clone, Copy)]
struct ItemStats {
    hp: i32,
    hpr_raw: i32,
    hpr_pct: i32,
    mr: i32,
    ls: i32,
    sd_raw: i32,
    walk_speed: i32,
    def_flat: [i32; 5],  // earth, thunder, water, fire, air
    def_pct: [i32; 5],
    sp_add: [i32; 5],
    sp_req: [i32; 5],
}

fn summarise_apparel(a: &Apparel) -> ItemStats {
    let ids = &a.identifications;
    ItemStats {
        hp: a.hp,
        hpr_raw: id_val(&ids.health_regen_raw),
        hpr_pct: id_val(&ids.health_regen_pct),
        mr: id_val(&ids.mana_regen),
        ls: id_val(&ids.life_steal),
        sd_raw: id_val(&ids.spell_damage_raw),
        walk_speed: id_val(&ids.walk_speed),
        def_flat: [
            a.defence_bonus.earth,
            a.defence_bonus.thunder,
            a.defence_bonus.water,
            a.defence_bonus.fire,
            a.defence_bonus.air,
        ],
        def_pct: [
            id_val(&ids.earth_defence_pct),
            id_val(&ids.thunder_defence_pct),
            id_val(&ids.water_defence_pct),
            id_val(&ids.fire_defence_pct),
            id_val(&ids.air_defence_pct),
        ],
        sp_add: a.skill_point_bonus.as_array(),
        sp_req: a.requirements.as_array(),
    }
}

fn id_val(v: &Option<RollableValue>) -> i32 {
    v.map(|rv| rv.max_value()).unwrap_or(0)
}

/// Precomputed stats from all locked items.
#[derive(Debug, Clone)]
struct LockedStats {
    hp: i32,
    hpr_raw: i32,
    hpr_pct: i32,
    mr: i32,
    ls: i32,
    sd_raw: i32,
    walk_speed: i32,
    def_flat: [i32; 5],
    def_pct: [i32; 5],
    sp_add: [i32; 5],
    sp_max_req: [i32; 5],
}

fn compute_locked_stats(build: &Build, flexible_slots: &[Slot]) -> LockedStats {
    let mut s = LockedStats {
        hp: build.base_hp(),
        hpr_raw: 0, hpr_pct: 0, mr: 0, ls: 0, sd_raw: 0, walk_speed: 0,
        def_flat: [0; 5], def_pct: [0; 5],
        sp_add: [0; 5], sp_max_req: [0; 5],
    };

    // Add weapon stats
    if let Some(w) = &build.weapon {
        let ids = &w.identifications;
        s.hp += w.hp;
        s.hpr_raw += id_val(&ids.health_regen_raw);
        s.hpr_pct += id_val(&ids.health_regen_pct);
        s.mr += id_val(&ids.mana_regen);
        s.ls += id_val(&ids.life_steal);
        s.sd_raw += id_val(&ids.spell_damage_raw);
        s.walk_speed += id_val(&ids.walk_speed);
        for (i, elem) in Element::ALL.iter().enumerate() {
            s.def_flat[i] += w.defence_bonus.get(*elem);
            s.def_pct[i] += id_val(match i {
                0 => &ids.earth_defence_pct,
                1 => &ids.thunder_defence_pct,
                2 => &ids.water_defence_pct,
                3 => &ids.fire_defence_pct,
                _ => &ids.air_defence_pct,
            });
            s.sp_add[i] += w.skill_point_bonus.get(*elem);
            let req = w.requirements.get(*elem);
            if req > s.sp_max_req[i] { s.sp_max_req[i] = req; }
        }
    }

    // Add locked apparel stats
    for slot in Slot::ALL {
        if slot == Slot::Weapon || flexible_slots.contains(&slot) {
            continue;
        }
        if let Some(apparel) = build.apparel(slot) {
            let is = summarise_apparel(apparel);
            s.hp += is.hp;
            s.hpr_raw += is.hpr_raw;
            s.hpr_pct += is.hpr_pct;
            s.mr += is.mr;
            s.ls += is.ls;
            s.sd_raw += is.sd_raw;
            s.walk_speed += is.walk_speed;
            for i in 0..5 {
                s.def_flat[i] += is.def_flat[i];
                s.def_pct[i] += is.def_pct[i];
                s.sp_add[i] += is.sp_add[i];
                if is.sp_req[i] > s.sp_max_req[i] { s.sp_max_req[i] = is.sp_req[i]; }
            }
        }
    }

    s
}

// ---------------------------------------------------------------------------
// Diverse candidate selection
// ---------------------------------------------------------------------------

/// Select candidates for a slot using multiple ranked lists to preserve
/// synergistic items that single-metric scoring would miss.
fn select_diverse_candidates<'a>(
    all_items: &[&'a Apparel],
    summaries: &[ItemStats],
    objectives: &[Objective],
    constraints: &Constraints,
    locked: &LockedStats,
    max_total: usize,
) -> Vec<usize> {
    let n = all_items.len();
    if n <= max_total {
        return (0..n).collect();
    }

    let mut selected = HashSet::with_capacity(max_total);
    let per_list = (max_total / 5).max(10); // divide budget across lists

    // Helper: add top-K by a scoring function
    let mut add_top_k = |score_fn: &dyn Fn(usize) -> i64, k: usize| {
        let mut ranked: Vec<(i64, usize)> = (0..n).map(|i| (score_fn(i), i)).collect();
        ranked.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        for (_, idx) in ranked.into_iter().take(k) {
            selected.insert(idx);
        }
    };

    // List 1: Top by HP (always useful)
    add_top_k(&|i| summaries[i].hp as i64, per_list);

    // List 2: Top by total SP bonus (synergy preservation)
    add_top_k(
        &|i| summaries[i].sp_add.iter().map(|&v| v.max(0) as i64).sum::<i64>(),
        per_list,
    );

    // List 3+: Top by each active objective
    for obj in objectives {
        let stat = match obj {
            Objective::Maximise(s) | Objective::Minimise(s) => *s,
        };
        add_top_k(&|i| item_stat_value(&summaries[i], stat), per_list);
    }

    // List: Top by each active constraint's stat (items that help meet it)
    let constraint_stats = active_constraint_stats(constraints);
    for stat in &constraint_stats {
        add_top_k(&|i| item_stat_value(&summaries[i], *stat), per_list);
    }

    // List: Top by SP bonus in elements with high locked requirements
    // (preserves items that enable other items via SP)
    for elem_idx in 0..5 {
        if locked.sp_max_req[elem_idx] > 30 {
            add_top_k(&|i| summaries[i].sp_add[elem_idx] as i64, per_list / 2);
        }
    }

    // Drop the closure so we can borrow selected immutably
    drop(add_top_k);

    // If still under budget, fill with top by combined defence
    if selected.len() < max_total {
        let remaining = max_total - selected.len();
        let mut ranked: Vec<(i64, usize)> = (0..n)
            .map(|i| (summaries[i].def_flat.iter().sum::<i32>() as i64, i))
            .collect();
        ranked.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        for (_, idx) in ranked.into_iter().take(remaining) {
            selected.insert(idx);
        }
    }

    let mut result: Vec<usize> = selected.into_iter().collect();
    result.sort_unstable(); // deterministic order
    result
}

fn item_stat_value(s: &ItemStats, stat: StatType) -> i64 {
    match stat {
        StatType::Hp => s.hp as i64,
        StatType::Ehp => s.hp as i64, // proxy: HP is main EHP driver at item level
        StatType::Hpr => (s.hpr_raw + s.hpr_pct) as i64,
        StatType::ManaRegen => s.mr as i64,
        StatType::LifeSteal => s.ls as i64,
        StatType::SpellDamageRaw => s.sd_raw as i64,
        StatType::MainAttackDamageRaw => 0, // TODO when we track this
        StatType::WalkSpeed => s.walk_speed as i64,
        StatType::EarthDefence => s.def_flat[0] as i64,
        StatType::ThunderDefence => s.def_flat[1] as i64,
        StatType::WaterDefence => s.def_flat[2] as i64,
        StatType::FireDefence => s.def_flat[3] as i64,
        StatType::AirDefence => s.def_flat[4] as i64,
    }
}

fn active_constraint_stats(c: &Constraints) -> Vec<StatType> {
    let mut stats = vec![];
    if c.min_hp.is_some() { stats.push(StatType::Hp); }
    if c.min_hpr.is_some() { stats.push(StatType::Hpr); }
    if c.min_mana_regen.is_some() { stats.push(StatType::ManaRegen); }
    if c.min_life_steal.is_some() { stats.push(StatType::LifeSteal); }
    if c.min_spell_damage_raw.is_some() { stats.push(StatType::SpellDamageRaw); }
    if c.min_walk_speed.is_some() { stats.push(StatType::WalkSpeed); }
    if c.min_earth_defence.is_some() { stats.push(StatType::EarthDefence); }
    if c.min_thunder_defence.is_some() { stats.push(StatType::ThunderDefence); }
    if c.min_water_defence.is_some() { stats.push(StatType::WaterDefence); }
    if c.min_fire_defence.is_some() { stats.push(StatType::FireDefence); }
    if c.min_air_defence.is_some() { stats.push(StatType::AirDefence); }
    stats
}

// ---------------------------------------------------------------------------
// Cheap cascading filters
// ---------------------------------------------------------------------------

/// Check cheap additive stats against constraints before expensive SP calc.
/// Returns false if the combination definitely can't satisfy constraints.
fn cheap_stat_check(
    locked: &LockedStats,
    flex_stats: &[&ItemStats],
    constraints: &Constraints,
) -> bool {
    // Sum HP
    let mut hp = locked.hp;
    for s in flex_stats { hp += s.hp; }
    if let Some(min) = constraints.min_hp {
        if hp < min { return false; }
    }

    // Sum defences (flat only — pct needs full calc, but flat is the main signal)
    let mut def = locked.def_flat;
    for s in flex_stats {
        for i in 0..5 { def[i] += s.def_flat[i]; }
    }
    if let Some(min) = constraints.min_earth_defence { if def[0] < min { return false; } }
    if let Some(min) = constraints.min_thunder_defence { if def[1] < min { return false; } }
    if let Some(min) = constraints.min_water_defence { if def[2] < min { return false; } }
    if let Some(min) = constraints.min_fire_defence { if def[3] < min { return false; } }
    if let Some(min) = constraints.min_air_defence { if def[4] < min { return false; } }

    // Sum other cheap stats
    let mut mr = locked.mr;
    let mut ls = locked.ls;
    let mut ws = locked.walk_speed;
    let mut sd = locked.sd_raw;
    let mut hpr_raw = locked.hpr_raw;
    for s in flex_stats {
        mr += s.mr;
        ls += s.ls;
        ws += s.walk_speed;
        sd += s.sd_raw;
        hpr_raw += s.hpr_raw;
    }
    if let Some(min) = constraints.min_mana_regen { if mr < min { return false; } }
    if let Some(min) = constraints.min_life_steal { if ls < min { return false; } }
    if let Some(min) = constraints.min_walk_speed { if ws < min { return false; } }
    if let Some(min) = constraints.min_spell_damage_raw { if sd < min { return false; } }
    if let Some(min) = constraints.min_hpr { if hpr_raw < min { return false; } }

    true
}

/// Tighter SP feasibility check with per-element 100 cap.
fn tight_sp_check(
    locked: &LockedStats,
    flex_stats: &[&ItemStats],
    available: i32,
) -> bool {
    let mut total_add = locked.sp_add;
    let mut max_req = locked.sp_max_req;

    for s in flex_stats {
        for i in 0..5 {
            total_add[i] += s.sp_add[i];
            if s.sp_req[i] > max_req[i] { max_req[i] = s.sp_req[i]; }
        }
    }

    let mut total_gap = 0i32;
    for i in 0..5 {
        let gap = (max_req[i] - total_add[i].max(0)).max(0);
        if gap > 100 { return false; } // Can't assign >100 to any element
        total_gap += gap;
    }

    total_gap <= available
}

// ---------------------------------------------------------------------------
// Main solver entry point
// ---------------------------------------------------------------------------

/// Progressive solver: tries the given locked slots first. If no results
/// are found within the time budget, progressively unlocks more slots
/// and retries until results are found or all slots are flexible.
pub fn solve(
    base_build: &Build,
    locked_slots: &[Slot],
    config: &SolverConfig,
    db: &ItemDb,
) -> Vec<SolverResult> {
    let time_budget = std::time::Duration::from_secs(3);

    // First attempt with the requested locked slots
    let results = solve_inner(base_build, locked_slots, config, db, time_budget);
    if !results.is_empty() {
        return results;
    }

    // No results — progressively unlock more slots.
    // Priority: unlock slots with the worst stat contributions first.
    let mut current_locked: Vec<Slot> = locked_slots.to_vec();
    let unlock_priority = [
        Slot::Bracelet, Slot::Necklace, Slot::Ring1, Slot::Ring2,
        Slot::Boots, Slot::Leggings, Slot::Helmet, Slot::Chestplate,
    ];

    for slot_to_unlock in &unlock_priority {
        if !current_locked.contains(slot_to_unlock) {
            continue; // already flexible
        }
        current_locked.retain(|s| s != slot_to_unlock);
        tracing::info!("solver: widening search — unlocked {:?}", slot_to_unlock);

        let results = solve_inner(base_build, &current_locked, config, db, time_budget);
        if !results.is_empty() {
            return results;
        }
    }

    vec![]
}

fn solve_inner(
    base_build: &Build,
    locked_slots: &[Slot],
    config: &SolverConfig,
    db: &ItemDb,
    time_budget: std::time::Duration,
) -> Vec<SolverResult> {
    let illegal = IllegalCombinations::default_groups();
    let deadline = std::time::Instant::now() + time_budget;

    let flexible_slots: Vec<Slot> = Slot::ALL
        .iter()
        .filter(|s| !locked_slots.contains(s) && **s != Slot::Weapon)
        .copied()
        .collect();

    if flexible_slots.is_empty() {
        let stats = calculate_build_stats(base_build);
        if config.constraints.is_satisfied(&stats) {
            let score = score_build(&stats, &config.objectives);
            return vec![SolverResult { build: base_build.clone(), stats, score }];
        }
        return vec![];
    }

    // Precompute locked item stats
    let locked = compute_locked_stats(base_build, &flexible_slots);

    // Scale candidates per slot based on number of flexible slots
    // to keep total combos bounded
    let effective_cap = match flexible_slots.len() {
        1 => config.max_candidates_per_slot,        // 100 → 100 combos
        2 => config.max_candidates_per_slot.min(80), // 80 → 6.4K combos
        3 => config.max_candidates_per_slot.min(50), // 50 → 125K combos
        _ => config.max_candidates_per_slot.min(30), // 30 → 810K combos for 4 slots
    };

    // Gather and select candidates per slot
    let candidates: Vec<(Slot, Vec<&Apparel>, Vec<ItemStats>)> = flexible_slots
        .iter()
        .map(|&slot| {
            let category = slot.category();
            let all_items: Vec<&Apparel> = db
                .apparels_in_category(category)
                .into_iter()
                .filter(|a| {
                    (config.min_item_level == 0 || a.level >= config.min_item_level)
                        && a.level <= base_build.level
                })
                .collect();

            let all_summaries: Vec<ItemStats> = all_items.iter().map(|a| summarise_apparel(a)).collect();

            let selected = select_diverse_candidates(
                &all_items,
                &all_summaries,
                &config.objectives,
                &config.constraints,
                &locked,
                effective_cap,
            );

            let items: Vec<&Apparel> = selected.iter().map(|&i| all_items[i]).collect();
            let summaries: Vec<ItemStats> = selected.iter().map(|&i| all_summaries[i]).collect();

            (slot, items, summaries)
        })
        .collect();

    let total_combos: u64 = candidates.iter().map(|(_, items, _)| items.len() as u64).product();
    let candidate_counts: Vec<_> = candidates.iter().map(|(_, v, _)| v.len()).collect();
    tracing::info!(
        "solver: {} flexible slots, {:?} candidates/slot, {} total combinations",
        candidates.len(), candidate_counts, total_combos
    );

    // Choose enumeration strategy
    if total_combos <= 200_000 {
        solve_enumerate(base_build, &candidates, &locked, config, &illegal, deadline)
    } else {
        solve_parallel(base_build, &candidates, &locked, config, &illegal, deadline)
    }
}

// ---------------------------------------------------------------------------
// Enumeration (single-threaded for small spaces)
// ---------------------------------------------------------------------------

fn solve_enumerate(
    base_build: &Build,
    candidates: &[(Slot, Vec<&Apparel>, Vec<ItemStats>)],
    locked: &LockedStats,
    config: &SolverConfig,
    illegal: &IllegalCombinations,
    deadline: std::time::Instant,
) -> Vec<SolverResult> {
    let mut results = TopN::new(config.max_results);
    let mut combo = vec![0usize; candidates.len()];
    let mut iterations = 0u64;

    loop {
        evaluate_staged(base_build, candidates, &combo, locked, config, illegal, &mut results);
        if !increment_combo(&mut combo, candidates) { break; }

        iterations += 1;
        if iterations % 10_000 == 0 && std::time::Instant::now() > deadline {
            tracing::warn!("solver: time budget exceeded after {} iterations", iterations);
            break;
        }
    }

    results.into_sorted()
}

// ---------------------------------------------------------------------------
// Parallel enumeration (rayon over first slot)
// ---------------------------------------------------------------------------

fn solve_parallel(
    base_build: &Build,
    candidates: &[(Slot, Vec<&Apparel>, Vec<ItemStats>)],
    locked: &LockedStats,
    config: &SolverConfig,
    illegal: &IllegalCombinations,
    deadline: std::time::Instant,
) -> Vec<SolverResult> {
    if candidates.is_empty() { return vec![]; }

    let first_count = candidates[0].1.len();
    let rest = &candidates[1..];
    let expired = std::sync::atomic::AtomicBool::new(false);

    let partial_results: Vec<Vec<SolverResult>> = (0..first_count)
        .into_par_iter()
        .map(|first_idx| {
            if expired.load(std::sync::atomic::Ordering::Relaxed) {
                return vec![];
            }

            // Build locked stats including this first-slot item
            let first_summary = &candidates[0].2[first_idx];
            let mut locked_plus = locked.clone();
            locked_plus.hp += first_summary.hp;
            locked_plus.hpr_raw += first_summary.hpr_raw;
            locked_plus.hpr_pct += first_summary.hpr_pct;
            locked_plus.mr += first_summary.mr;
            locked_plus.ls += first_summary.ls;
            locked_plus.sd_raw += first_summary.sd_raw;
            locked_plus.walk_speed += first_summary.walk_speed;
            for i in 0..5 {
                locked_plus.def_flat[i] += first_summary.def_flat[i];
                locked_plus.def_pct[i] += first_summary.def_pct[i];
                locked_plus.sp_add[i] += first_summary.sp_add[i];
                if first_summary.sp_req[i] > locked_plus.sp_max_req[i] {
                    locked_plus.sp_max_req[i] = first_summary.sp_req[i];
                }
            }

            let mut results = TopN::new(config.max_results);

            if rest.is_empty() {
                let combo = vec![first_idx];
                evaluate_staged(base_build, candidates, &combo, locked, config, illegal, &mut results);
            } else {
                let mut combo = vec![0usize; rest.len()];
                let mut iterations = 0u64;
                loop {
                    let flex_stats: Vec<&ItemStats> = combo.iter().enumerate()
                        .map(|(i, &c)| &rest[i].2[c])
                        .collect();

                    if cheap_stat_check(&locked_plus, &flex_stats, &config.constraints)
                        && tight_sp_check(&locked_plus, &flex_stats, base_build.available_points as i32)
                    {
                        let mut full_combo = vec![first_idx];
                        full_combo.extend_from_slice(&combo);
                        evaluate_staged(base_build, candidates, &full_combo, locked, config, illegal, &mut results);
                    }

                    if !increment_combo_rest(&mut combo, rest) { break; }

                    iterations += 1;
                    if iterations % 5_000 == 0 && std::time::Instant::now() > deadline {
                        expired.store(true, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                }
            }

            results.into_sorted()
        })
        .collect();

    if expired.load(std::sync::atomic::Ordering::Relaxed) {
        tracing::warn!("solver: parallel search hit time budget");
    }

    let mut final_results = TopN::new(config.max_results);
    for batch in partial_results {
        for r in batch { final_results.push(r); }
    }
    final_results.into_sorted()
}

// ---------------------------------------------------------------------------
// Staged evaluation
// ---------------------------------------------------------------------------

fn evaluate_staged(
    base_build: &Build,
    candidates: &[(Slot, Vec<&Apparel>, Vec<ItemStats>)],
    combo: &[usize],
    locked: &LockedStats,
    config: &SolverConfig,
    illegal: &IllegalCombinations,
    results: &mut TopN,
) {
    // Gather flex item summaries
    let flex_stats: Vec<&ItemStats> = combo.iter().enumerate()
        .map(|(i, &c)| &candidates[i].2[c])
        .collect();

    // Stage 1: cheap stat check
    if !cheap_stat_check(locked, &flex_stats, &config.constraints) {
        return;
    }

    // Stage 2: tight SP feasibility
    if !tight_sp_check(locked, &flex_stats, base_build.available_points as i32) {
        return;
    }

    // Stage 3: assemble build and check illegal combos
    let mut build = base_build.clone();
    for (i, &c) in combo.iter().enumerate() {
        build.set_apparel(candidates[i].0, Some(candidates[i].1[c].clone()));
    }
    build.assigned_sp = None;

    let apparels: Vec<&Apparel> = build.apparels().collect();
    if illegal.is_illegal(&apparels) {
        return;
    }

    // Stage 4: full stat calculation (expensive — SP assignment + EHP)
    let stats = calculate_build_stats(&build);

    // Stage 5: SP validity
    if !stats.sp_assignment.is_valid(build.available_points as i32) {
        return;
    }

    // Stage 6: full constraint check (includes EHP)
    if !config.constraints.is_satisfied(&stats) {
        return;
    }

    let score = score_build(&stats, &config.objectives);
    results.push(SolverResult { build, stats, score });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn increment_combo(combo: &mut [usize], candidates: &[(Slot, Vec<&Apparel>, Vec<ItemStats>)]) -> bool {
    for i in (0..combo.len()).rev() {
        combo[i] += 1;
        if combo[i] < candidates[i].1.len() { return true; }
        combo[i] = 0;
    }
    false
}

fn increment_combo_rest(combo: &mut [usize], rest: &[(Slot, Vec<&Apparel>, Vec<ItemStats>)]) -> bool {
    for i in (0..combo.len()).rev() {
        combo[i] += 1;
        if combo[i] < rest[i].1.len() { return true; }
        combo[i] = 0;
    }
    false
}

// ---------------------------------------------------------------------------
// Top-N heap
// ---------------------------------------------------------------------------

struct TopN {
    capacity: usize,
    heap: BinaryHeap<MinScoreResult>,
}

impl TopN {
    fn new(capacity: usize) -> Self {
        Self { capacity, heap: BinaryHeap::with_capacity(capacity + 1) }
    }

    fn push(&mut self, result: SolverResult) {
        if self.heap.len() < self.capacity {
            self.heap.push(MinScoreResult(result));
        } else if let Some(worst) = self.heap.peek() {
            if result.score > worst.0.score {
                self.heap.pop();
                self.heap.push(MinScoreResult(result));
            }
        }
    }

    fn into_sorted(self) -> Vec<SolverResult> {
        let mut results: Vec<SolverResult> = self.heap.into_iter().map(|m| m.0).collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        results
    }
}

struct MinScoreResult(SolverResult);

impl PartialEq for MinScoreResult {
    fn eq(&self, other: &Self) -> bool { self.0.score == other.0.score }
}
impl Eq for MinScoreResult {}
impl PartialOrd for MinScoreResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for MinScoreResult {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.score.partial_cmp(&self.0.score).unwrap_or(Ordering::Equal)
    }
}
