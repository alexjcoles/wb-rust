use std::cmp::Ordering;
use std::collections::BinaryHeap;

use rayon::prelude::*;
use wynn_core::build::Build;
use wynn_core::calculate::{calculate_build_stats, BuildStats};
use wynn_core::db::ItemDb;
use wynn_core::hive::IllegalCombinations;
use wynn_core::item::{Apparel, Slot};
use wynn_core::skill_points::fast_sp_check;

use crate::constraints::{Constraints, Objective};
use crate::scoring::score_build;

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
    /// Minimum item level to consider (filters out low-level junk).
    pub min_item_level: u32,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            constraints: Constraints::default(),
            objectives: vec![],
            max_results: 5,
            min_item_level: 80,
        }
    }
}

/// Run a constrained build search.
///
/// Takes a base build with locked slots, and searches for valid
/// item combinations in the flexible (unlocked) slots.
pub fn solve(
    base_build: &Build,
    locked_slots: &[Slot],
    config: &SolverConfig,
    db: &ItemDb,
) -> Vec<SolverResult> {
    let illegal = IllegalCombinations::default_groups();

    // Identify flexible apparel slots (weapon is always locked for now)
    let flexible_slots: Vec<Slot> = Slot::ALL
        .iter()
        .filter(|s| !locked_slots.contains(s) && **s != Slot::Weapon)
        .copied()
        .collect();

    if flexible_slots.is_empty() {
        // Nothing to flex - just evaluate the base build
        let stats = calculate_build_stats(base_build);
        if config.constraints.is_satisfied(&stats) {
            let score = score_build(&stats, &config.objectives);
            return vec![SolverResult {
                build: base_build.clone(),
                stats,
                score,
            }];
        }
        return vec![];
    }

    // Get candidate items for each flexible slot, filtered by level
    let candidates: Vec<(Slot, Vec<&Apparel>)> = flexible_slots
        .iter()
        .map(|&slot| {
            let category = slot.category();
            let mut items: Vec<&Apparel> = db
                .apparels_in_category(category)
                .into_iter()
                .filter(|a| a.level >= config.min_item_level && a.level <= base_build.level)
                .collect();
            // Sort by HP descending as a heuristic for early pruning
            items.sort_by(|a, b| b.hp.cmp(&a.hp));
            (slot, items)
        })
        .collect();

    let total_combos: u64 = candidates
        .iter()
        .map(|(_, items)| items.len() as u64)
        .product();
    tracing::info!(
        "solver: {} flexible slots, {} total combinations",
        candidates.len(),
        total_combos
    );

    // For small search spaces, enumerate directly.
    // For large ones, use parallel iteration.
    if candidates.len() <= 2 || total_combos <= 100_000 {
        solve_enumerate(base_build, locked_slots, &candidates, config, &illegal)
    } else {
        solve_parallel(base_build, locked_slots, &candidates, config, &illegal)
    }
}

/// Direct enumeration for small search spaces.
fn solve_enumerate(
    base_build: &Build,
    _locked_slots: &[Slot],
    candidates: &[(Slot, Vec<&Apparel>)],
    config: &SolverConfig,
    illegal: &IllegalCombinations,
) -> Vec<SolverResult> {
    let mut results = TopN::new(config.max_results);
    let mut combo = vec![0usize; candidates.len()];

    loop {
        let build = assemble_build(base_build, candidates, &combo);
        evaluate_and_insert(&build, config, illegal, &mut results);

        // Increment combination counter (odometer style)
        if !increment_combo(&mut combo, candidates) {
            break;
        }
    }

    results.into_sorted()
}

/// Parallel enumeration for large search spaces.
/// Parallelises over the first flexible slot's candidates.
fn solve_parallel(
    base_build: &Build,
    _locked_slots: &[Slot],
    candidates: &[(Slot, Vec<&Apparel>)],
    config: &SolverConfig,
    illegal: &IllegalCombinations,
) -> Vec<SolverResult> {
    if candidates.is_empty() {
        return vec![];
    }

    let (first_slot, first_items) = &candidates[0];
    let rest = &candidates[1..];

    let partial_results: Vec<Vec<SolverResult>> = first_items
        .par_iter()
        .map(|&first_item| {
            let mut partial_build = base_build.clone();
            partial_build.set_apparel(*first_slot, Some(first_item.clone()));

            let mut results = TopN::new(config.max_results);
            let mut combo = vec![0usize; rest.len()];

            if rest.is_empty() {
                evaluate_and_insert(&partial_build, config, illegal, &mut results);
            } else {
                loop {
                    let build = assemble_build_from(&partial_build, rest, &combo);
                    evaluate_and_insert(&build, config, illegal, &mut results);
                    if !increment_combo(&mut combo, rest) {
                        break;
                    }
                }
            }

            results.into_sorted()
        })
        .collect();

    // Merge all partial results and keep top N
    let mut final_results = TopN::new(config.max_results);
    for batch in partial_results {
        for r in batch {
            final_results.push(r);
        }
    }

    final_results.into_sorted()
}

fn assemble_build(
    base: &Build,
    candidates: &[(Slot, Vec<&Apparel>)],
    combo: &[usize],
) -> Build {
    let mut build = base.clone();
    for (i, (slot, items)) in candidates.iter().enumerate() {
        build.set_apparel(*slot, Some(items[combo[i]].clone()));
    }
    build.assigned_sp = None; // Force recalculation
    build
}

fn assemble_build_from(
    base: &Build,
    candidates: &[(Slot, Vec<&Apparel>)],
    combo: &[usize],
) -> Build {
    let mut build = base.clone();
    for (i, (slot, items)) in candidates.iter().enumerate() {
        build.set_apparel(*slot, Some(items[combo[i]].clone()));
    }
    build.assigned_sp = None;
    build
}

fn evaluate_and_insert(
    build: &Build,
    config: &SolverConfig,
    illegal: &IllegalCombinations,
    results: &mut TopN,
) {
    // Fast pre-checks before expensive stat calculation
    let apparels: Vec<&Apparel> = build.apparels().collect();

    // Illegal combination check
    if illegal.is_illegal(&apparels) {
        return;
    }

    // Fast SP feasibility check
    if !fast_sp_check(&apparels, build.available_points as i32) {
        return;
    }

    // Full stat calculation
    let stats = calculate_build_stats(build);

    // SP validity check
    if !stats.sp_assignment.is_valid(build.available_points as i32) {
        return;
    }

    // Constraint check
    if !config.constraints.is_satisfied(&stats) {
        return;
    }

    let score = score_build(&stats, &config.objectives);
    results.push(SolverResult {
        build: build.clone(),
        stats,
        score,
    });
}

/// Increment an odometer-style combination counter. Returns false when wrapped.
fn increment_combo(combo: &mut [usize], candidates: &[(Slot, Vec<&Apparel>)]) -> bool {
    for i in (0..combo.len()).rev() {
        combo[i] += 1;
        if combo[i] < candidates[i].1.len() {
            return true;
        }
        combo[i] = 0;
    }
    false
}

/// Keeps the top N results by score using a min-heap.
struct TopN {
    capacity: usize,
    heap: BinaryHeap<MinScoreResult>,
}

impl TopN {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            heap: BinaryHeap::with_capacity(capacity + 1),
        }
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

/// Wrapper to make BinaryHeap a min-heap on score (lowest score at top = first to evict).
struct MinScoreResult(SolverResult);

impl PartialEq for MinScoreResult {
    fn eq(&self, other: &Self) -> bool {
        self.0.score == other.0.score
    }
}

impl Eq for MinScoreResult {}

impl PartialOrd for MinScoreResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MinScoreResult {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering: smaller score = "greater" in heap = popped first
        other
            .0
            .score
            .partial_cmp(&self.0.score)
            .unwrap_or(Ordering::Equal)
    }
}
