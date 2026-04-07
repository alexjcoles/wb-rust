use crate::item::Apparel;
use crate::stats::{ElementalValues, SkillPoints};

/// Result of skill point assignment calculation.
#[derive(Debug, Clone)]
pub struct SpAssignment {
    /// Points the player must manually assign.
    pub assigned: SkillPoints,
    /// Total effective skill points (assigned + item bonuses).
    pub total: SkillPoints,
}

impl SpAssignment {
    /// Total manually assigned points (only positive assignments count).
    pub fn total_assigned(&self) -> i32 {
        let arr = self.assigned.as_array();
        arr.iter().filter(|&&v| v > 0).sum()
    }

    /// Whether the assignment is valid (each element <= 100, total <= budget).
    pub fn is_valid(&self, available: i32) -> bool {
        let arr = self.assigned.as_array();
        arr.iter().all(|&v| v <= 100) && self.total_assigned() <= available
    }
}

/// Convert skill points to the percentage bonus they provide.
const R: f64 = 0.9908;

pub fn skill_points_to_percentage(skp: i32) -> f64 {
    let skp = skp.clamp(0, 150);
    (R / (1.0 - R) * (1.0 - R.powi(skp))) / 100.0
}

/// Fast check: can these items possibly fit within the SP budget?
pub fn fast_sp_check(apparels: &[&Apparel], available_points: i32) -> bool {
    let mut total_add = [0i32; 5];
    let mut max_req = [0i32; 5];

    for apparel in apparels {
        let add = apparel.skill_point_bonus.as_array();
        let req = apparel.requirements.as_array();
        for i in 0..5 {
            total_add[i] += add[i];
            if req[i] > max_req[i] { max_req[i] = req[i]; }
        }
    }

    let mut total_gap = 0i32;
    for i in 0..5 {
        let gap = (max_req[i] - total_add[i].max(0)).max(0);
        if gap > 100 { return false; }
        total_gap += gap;
    }

    total_gap <= available_points
}

// ---------------------------------------------------------------------------
// WynnBuilder-matching SP calculation
//
// Matches hppeng-wynn skillpoints.js `calculate_skillpoints` exactly:
// - Recursive permutation search over item orderings
// - "Skip constraint" checks: an ordering is only valid if items that were
//   skipped truly could not have been equipped at the point they were skipped
// - "Pop check" (fix_should_pop): after all items, verify each item won't
//   pop off due to its own negative SP bonuses
// - Weapon is applied last, after all apparel
// - Game equip order: boots, leggings, chestplate, helmet, ring1, ring2,
//   bracelet, necklace
// ---------------------------------------------------------------------------

/// Calculate skill point assignment matching WynnBuilder exactly.
pub fn calculate_sp_assignment(
    apparels: &[&Apparel],
    weapon_req: &ElementalValues<i32>,
    weapon_add: &ElementalValues<i32>,
) -> SpAssignment {
    if apparels.is_empty() {
        return apply_weapon_only(weapon_req, weapon_add);
    }

    // Extract SP data into flat arrays for fast access
    let n = apparels.len();
    let mut reqs = vec![[0i32; 5]; n];
    let mut adds = vec![[0i32; 5]; n];
    for (i, a) in apparels.iter().enumerate() {
        reqs[i] = a.requirements.as_array();
        adds[i] = a.skill_point_bonus.as_array();
    }

    let w_req = weapon_req.as_array();
    let w_add = weapon_add.as_array();

    let mut best = RecurseState {
        best_assigned: [0; 5],
        best_total: [0; 5],
        best_cost: i32::MAX,
        best_under100: false,
    };

    let initial_order: Vec<usize> = (0..n).collect();

    recurse_check(
        &reqs, &adds, &w_req, &w_add,
        &mut [0; 5],     // assigned
        &mut [0; 5],     // totals
        0,               // total_applied
        &mut vec![],      // skipped_states (SP totals when each item was skipped)
        &mut vec![],      // prior_skipped (indices of skipped items)
        &mut vec![],      // equipped (indices of equipped items)
        &initial_order,   // remains_in_order
        &mut best,
    );

    // If no valid ordering found, fall back to simple greedy
    if best.best_cost == i32::MAX {
        return greedy_fallback(apparels, weapon_req, weapon_add);
    }

    SpAssignment {
        assigned: SkillPoints::from_array(best.best_assigned),
        total: SkillPoints::from_array(best.best_total),
    }
}

struct RecurseState {
    best_assigned: [i32; 5],
    best_total: [i32; 5],
    best_cost: i32,
    best_under100: bool,
}

fn recurse_check(
    reqs: &[[i32; 5]],
    adds: &[[i32; 5]],
    w_req: &[i32; 5],
    w_add: &[i32; 5],
    assigned: &mut [i32; 5],
    totals: &mut [i32; 5],
    total_applied: i32,
    skipped_states: &mut Vec<[i32; 5]>,
    prior_skipped: &mut Vec<usize>,
    equipped: &mut Vec<usize>,
    remains: &[usize],
    best: &mut RecurseState,
) {
    if remains.len() == 1 {
        // Base case: equip the last item, then weapon, then pop check
        let item_idx = remains[0];

        // Save state for backtracking
        let saved_assigned = *assigned;
        let saved_totals = *totals;

        // Equip last item
        apply_to_fit(assigned, totals, &reqs[item_idx]);
        apply_bonuses(totals, &adds[item_idx]);

        // Equip weapon
        apply_to_fit(assigned, totals, w_req);
        apply_bonuses(totals, w_add);

        // Pop check: for every equipped item + this one, verify it won't pop off
        let mut all_items: Vec<usize> = equipped.clone();
        all_items.push(item_idx);

        for &idx in &all_items {
            fix_should_pop(assigned, totals, &reqs[idx], &adds[idx]);
        }

        // Check skipped constraints with final deltas
        let delta = compute_delta(&saved_totals, totals);
        let mut valid = true;
        for (skip_idx, skip_state) in prior_skipped.iter().zip(skipped_states.iter()) {
            if can_equip_with_delta(skip_state, &delta, &reqs[*skip_idx]) {
                valid = false;
                break;
            }
        }

        if valid {
            let cost: i32 = assigned.iter().filter(|&&v| v > 0).sum();
            let under100 = assigned.iter().all(|&v| v <= 100);

            let is_better = cost < best.best_cost
                || (cost == best.best_cost && under100 && !best.best_under100);

            if is_better {
                best.best_assigned = *assigned;
                best.best_total = *totals;
                best.best_cost = cost;
                best.best_under100 = under100;
            }
        }

        // Restore state
        *assigned = saved_assigned;
        *totals = saved_totals;
        return;
    }

    // Recursive case: try each item in remains as the next to equip
    for pick in 0..remains.len() {
        let item_idx = remains[pick];

        // Items before `pick` in remains are being "skipped" (head)
        let head: Vec<usize> = remains[..pick].to_vec();
        let tail: Vec<usize> = remains[pick + 1..].to_vec();

        // Save state
        let saved_assigned = *assigned;
        let saved_totals = *totals;
        let _saved_total_applied = total_applied;

        // Equip this item
        apply_to_fit(assigned, totals, &reqs[item_idx]);

        // Check skip constraint 1: previously skipped items
        // If any previously skipped item could have been equipped given the
        // delta from this step, this ordering is invalid
        let delta = compute_delta(&saved_totals, totals);
        let mut skip1_valid = true;
        for (skip_idx, skip_state) in prior_skipped.iter().zip(skipped_states.iter()) {
            if can_equip_with_delta(skip_state, &delta, &reqs[*skip_idx]) {
                skip1_valid = false;
                break;
            }
        }

        if !skip1_valid {
            *assigned = saved_assigned;
            *totals = saved_totals;
            continue;
        }

        // Check skip constraint 2: head items being skipped now
        // After fitting the picked item, could any head item be equipped?
        let mut skip2_valid = true;
        for &head_idx in &head {
            if can_equip(totals, &reqs[head_idx]) {
                skip2_valid = false;
                break;
            }
        }

        // Apply bonuses after skip2 check (WynnBuilder applies bonuses
        // after checking if skipped items could fit)
        apply_bonuses(totals, &adds[item_idx]);

        if !skip2_valid {
            *assigned = saved_assigned;
            *totals = saved_totals;
            continue;
        }

        let new_total_applied: i32 = assigned.iter().filter(|&&v| v > 0).sum();

        // Prune: if already worse than best, skip
        if new_total_applied >= best.best_cost {
            *assigned = saved_assigned;
            *totals = saved_totals;
            continue;
        }

        // Record skip states for head items
        let prev_skipped_len = prior_skipped.len();
        let prev_states_len = skipped_states.len();
        for &h in &head {
            prior_skipped.push(h);
            skipped_states.push(saved_totals);
        }

        // Build new remains: tail + head (skipped items go to end)
        let mut new_remains = tail.clone();
        new_remains.extend_from_slice(&head);

        equipped.push(item_idx);

        recurse_check(
            reqs, adds, w_req, w_add,
            assigned, totals, new_total_applied,
            skipped_states, prior_skipped, equipped,
            &new_remains, best,
        );

        equipped.pop();
        prior_skipped.truncate(prev_skipped_len);
        skipped_states.truncate(prev_states_len);

        // Restore state
        *assigned = saved_assigned;
        *totals = saved_totals;
    }
}

/// Assign enough SP to meet an item's requirements.
fn apply_to_fit(assigned: &mut [i32; 5], totals: &mut [i32; 5], req: &[i32; 5]) {
    for i in 0..5 {
        if req[i] > 0 && totals[i] < req[i] {
            let gap = req[i] - totals[i];
            assigned[i] += gap;
            totals[i] += gap;
        }
    }
}

/// Add an item's SP bonuses to totals.
fn apply_bonuses(totals: &mut [i32; 5], add: &[i32; 5]) {
    for i in 0..5 {
        totals[i] += add[i];
    }
}

/// Pop check: after all items are equipped, verify an item won't "pop off".
/// For non-crafted items: effective_req = req + bonus.
/// If effective_req > current total, assign more SP.
fn fix_should_pop(assigned: &mut [i32; 5], totals: &mut [i32; 5], req: &[i32; 5], add: &[i32; 5]) {
    for i in 0..5 {
        if req[i] == 0 { continue; }
        // Effective requirement: the item needs req[i] SP, but it also
        // contributes add[i]. If add[i] is negative, we need extra SP
        // to compensate. If add[i] is positive, the check is stricter
        // because the item's own bonus inflates the total.
        let effective_req = req[i] + add[i];
        if effective_req > totals[i] {
            let gap = effective_req - totals[i];
            assigned[i] += gap;
            totals[i] += gap;
        }
    }
}

/// Check if an item can be equipped given current totals.
fn can_equip(totals: &[i32; 5], req: &[i32; 5]) -> bool {
    for i in 0..5 {
        if req[i] > 0 && totals[i] < req[i] {
            return false;
        }
    }
    true
}

/// Check if an item could be equipped given a saved state + delta.
fn can_equip_with_delta(saved_state: &[i32; 5], delta: &[i32; 5], req: &[i32; 5]) -> bool {
    for i in 0..5 {
        if req[i] > 0 && (saved_state[i] + delta[i]) < req[i] {
            return false;
        }
    }
    true
}

/// Compute the delta between two SP states.
fn compute_delta(old: &[i32; 5], new: &[i32; 5]) -> [i32; 5] {
    let mut delta = [0; 5];
    for i in 0..5 {
        delta[i] = new[i] - old[i];
    }
    delta
}

/// Simple greedy fallback if the recursive search finds nothing.
fn greedy_fallback(
    apparels: &[&Apparel],
    weapon_req: &ElementalValues<i32>,
    weapon_add: &ElementalValues<i32>,
) -> SpAssignment {
    let mut assigned = [0i32; 5];
    let mut totals = [0i32; 5];

    // Sort by total SP bonus descending (process providers first)
    let mut order: Vec<usize> = (0..apparels.len()).collect();
    order.sort_by(|&a, &b| {
        apparels[b].skill_point_bonus.sum().cmp(&apparels[a].skill_point_bonus.sum())
    });

    for &idx in &order {
        let req = apparels[idx].requirements.as_array();
        let add = apparels[idx].skill_point_bonus.as_array();
        apply_to_fit(&mut assigned, &mut totals, &req);
        apply_bonuses(&mut totals, &add);
    }

    let w_req = weapon_req.as_array();
    let w_add = weapon_add.as_array();
    apply_to_fit(&mut assigned, &mut totals, &w_req);
    apply_bonuses(&mut totals, &w_add);

    // Pop check
    for &idx in &order {
        let req = apparels[idx].requirements.as_array();
        let add = apparels[idx].skill_point_bonus.as_array();
        fix_should_pop(&mut assigned, &mut totals, &req, &add);
    }

    SpAssignment {
        assigned: SkillPoints::from_array(assigned),
        total: SkillPoints::from_array(totals),
    }
}

fn apply_weapon_only(
    weapon_req: &ElementalValues<i32>,
    weapon_add: &ElementalValues<i32>,
) -> SpAssignment {
    let mut assigned = [0i32; 5];
    let mut totals = [0i32; 5];
    let req = weapon_req.as_array();
    let add = weapon_add.as_array();
    apply_to_fit(&mut assigned, &mut totals, &req);
    apply_bonuses(&mut totals, &add);
    SpAssignment {
        assigned: SkillPoints::from_array(assigned),
        total: SkillPoints::from_array(totals),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_points_to_percentage() {
        assert!((skill_points_to_percentage(0) - 0.0).abs() < 0.001);
        let pct_150 = skill_points_to_percentage(150);
        assert!(pct_150 > 0.0 && pct_150 < 1.0);
        assert!((skill_points_to_percentage(-10) - 0.0).abs() < 0.001);
    }
}
