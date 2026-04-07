use crate::item::Apparel;
use crate::stats::{Element, ElementalValues, SkillPoints};

/// Result of skill point assignment calculation.
#[derive(Debug, Clone)]
pub struct SpAssignment {
    /// Points the player must manually assign.
    pub assigned: SkillPoints,
    /// Total effective skill points (assigned + item bonuses).
    pub total: SkillPoints,
}

impl SpAssignment {
    /// Total manually assigned points.
    pub fn total_assigned(&self) -> i32 {
        self.assigned.sum()
    }

    /// Whether the assignment is valid (each element <= 100, total <= budget).
    pub fn is_valid(&self, available: i32) -> bool {
        let arr = self.assigned.as_array();
        arr.iter().all(|&v| v <= 100) && self.total_assigned() <= available
    }
}

/// Convert skill points to the percentage bonus they provide.
/// This is the fundamental curve used throughout Wynncraft.
const R: f64 = 0.9908;

pub fn skill_points_to_percentage(skp: i32) -> f64 {
    let skp = skp.clamp(0, 150);
    (R / (1.0 - R) * (1.0 - R.powi(skp))) / 100.0
}

/// Fast check: can these items possibly fit within the SP budget?
/// Sums all positive adds and checks against max requirements.
pub fn fast_sp_check(apparels: &[&Apparel], available_points: i32) -> bool {
    let mut total_add = ElementalValues::<i32>::default();
    let mut max_req = ElementalValues::<i32>::default();

    for apparel in apparels {
        for elem in Element::ALL {
            let add = apparel.skill_point_bonus.get(elem);
            *elem_mut(&mut total_add, elem) += add;

            let req = apparel.requirements.get(elem);
            if req > max_req.get(elem) {
                max_req.set(elem, req);
            }
        }
    }

    let mut total_gap = 0i32;
    for elem in Element::ALL {
        let gap = (max_req.get(elem) - total_add.get(elem).max(0)).max(0);
        total_gap += gap;
    }

    total_gap <= available_points
}

/// Calculate the optimal skill point assignment for a set of apparels + weapon.
///
/// Uses Tarjan's SCC algorithm to find groups of mutually-dependent items,
/// tries all permutations within each SCC, and picks the ordering that
/// minimises total assigned SP.
///
/// This matches the `scc_put_calculate` algorithm from WynnBuilderTools.
pub fn calculate_sp_assignment(
    apparels: &[&Apparel],
    weapon_req: &ElementalValues<i32>,
    weapon_add: &ElementalValues<i32>,
) -> SpAssignment {
    let n = apparels.len();
    if n == 0 {
        return apply_weapon_only(weapon_req, weapon_add);
    }

    // Build dependency graph: item i depends on item j if j provides SP
    // that could help meet i's requirements
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            if depends_on(apparels[i], apparels[j]) {
                adj[i].push(j);
            }
        }
    }

    // Find SCCs using Tarjan's algorithm
    let sccs = tarjan_scc(n, &adj);

    // Try all permutations within each SCC, topological order between SCCs.
    // Tarjan returns SCCs with providers before consumers (deepest-first),
    // which is already the order we want: process SP-giving items first.
    let scc_order = sccs;

    // For small SCCs (most cases), try all permutations.
    // For large SCCs (rare), fall back to greedy within the component.
    let mut best_assigned = None;
    let mut best_total = None;
    let mut best_cost = i32::MAX;

    // Generate all possible orderings by permuting within each SCC
    let mut orderings: Vec<Vec<usize>> = vec![vec![]];

    for scc in &scc_order {
        if scc.len() <= 6 {
            // Try all permutations of this SCC
            let perms = permutations(scc);
            let mut new_orderings = Vec::with_capacity(orderings.len() * perms.len());
            for base in &orderings {
                for perm in &perms {
                    let mut o = base.clone();
                    o.extend_from_slice(perm);
                    new_orderings.push(o);
                }
            }
            orderings = new_orderings;
        } else {
            // Too many permutations; use greedy sort within SCC
            let mut sorted = scc.clone();
            sorted.sort_by(|&a, &b| {
                apparels[b]
                    .skill_point_bonus
                    .sum()
                    .cmp(&apparels[a].skill_point_bonus.sum())
            });
            for o in &mut orderings {
                o.extend_from_slice(&sorted);
            }
        }

        // Prune: keep only orderings that are competitive so far.
        // Cap to avoid explosion (shouldn't happen with typical builds of 8 items).
        if orderings.len() > 5000 {
            orderings.truncate(5000);
        }
    }

    for ordering in &orderings {
        let (assigned, total) = evaluate_ordering(apparels, ordering, weapon_req, weapon_add);
        let cost = assigned.sum();
        if cost < best_cost {
            best_cost = cost;
            best_assigned = Some(assigned);
            best_total = Some(total);
        }
    }

    SpAssignment {
        assigned: best_assigned.unwrap_or_default(),
        total: best_total.unwrap_or_default(),
    }
}

/// Check if item `a` depends on item `b` (i.e., b's adds could help meet a's reqs).
fn depends_on(a: &Apparel, b: &Apparel) -> bool {
    for elem in Element::ALL {
        let a_req = a.requirements.get(elem);
        let b_add = b.skill_point_bonus.get(elem);
        if a_req > 0 && b_add > 0 {
            return true;
        }
    }
    false
}

/// Evaluate a specific item ordering: greedily assign SP to meet each item's
/// requirements, then add its bonuses.
fn evaluate_ordering(
    apparels: &[&Apparel],
    order: &[usize],
    weapon_req: &ElementalValues<i32>,
    weapon_add: &ElementalValues<i32>,
) -> (SkillPoints, SkillPoints) {
    let mut assigned = SkillPoints::default();
    let mut current = SkillPoints::default();

    for &idx in order {
        let apparel = apparels[idx];
        // Assign enough SP to meet this item's requirements
        for elem in Element::ALL {
            let req = apparel.requirements.get(elem);
            if req > 0 && current.get(elem) < req {
                let gap = req - current.get(elem);
                *elem_mut(&mut assigned, elem) += gap;
                *elem_mut(&mut current, elem) += gap;
            }
        }
        // Add this item's SP bonuses
        for elem in Element::ALL {
            *elem_mut(&mut current, elem) += apparel.skill_point_bonus.get(elem);
        }
    }

    // Apply weapon requirements
    for elem in Element::ALL {
        let req = weapon_req.get(elem);
        if req > 0 && current.get(elem) < req {
            let gap = req - current.get(elem);
            *elem_mut(&mut assigned, elem) += gap;
            *elem_mut(&mut current, elem) += gap;
        }
    }
    // Add weapon bonuses
    for elem in Element::ALL {
        *elem_mut(&mut current, elem) += weapon_add.get(elem);
    }

    (assigned, current)
}

fn apply_weapon_only(
    weapon_req: &ElementalValues<i32>,
    weapon_add: &ElementalValues<i32>,
) -> SpAssignment {
    let mut assigned = SkillPoints::default();
    let mut current = SkillPoints::default();
    for elem in Element::ALL {
        let req = weapon_req.get(elem);
        if req > 0 {
            *elem_mut(&mut assigned, elem) = req;
            *elem_mut(&mut current, elem) = req;
        }
        *elem_mut(&mut current, elem) += weapon_add.get(elem);
    }
    SpAssignment {
        assigned,
        total: current,
    }
}

/// Tarjan's SCC algorithm.
fn tarjan_scc(n: usize, adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    struct State {
        index_counter: usize,
        stack: Vec<usize>,
        on_stack: Vec<bool>,
        index: Vec<Option<usize>>,
        lowlink: Vec<usize>,
        result: Vec<Vec<usize>>,
    }

    fn strongconnect(v: usize, adj: &[Vec<usize>], state: &mut State) {
        state.index[v] = Some(state.index_counter);
        state.lowlink[v] = state.index_counter;
        state.index_counter += 1;
        state.stack.push(v);
        state.on_stack[v] = true;

        for &w in &adj[v] {
            if state.index[w].is_none() {
                strongconnect(w, adj, state);
                state.lowlink[v] = state.lowlink[v].min(state.lowlink[w]);
            } else if state.on_stack[w] {
                state.lowlink[v] = state.lowlink[v].min(state.index[w].unwrap());
            }
        }

        if state.lowlink[v] == state.index[v].unwrap() {
            let mut component = Vec::new();
            loop {
                let w = state.stack.pop().unwrap();
                state.on_stack[w] = false;
                component.push(w);
                if w == v {
                    break;
                }
            }
            state.result.push(component);
        }
    }

    let mut state = State {
        index_counter: 0,
        stack: Vec::new(),
        on_stack: vec![false; n],
        index: vec![None; n],
        lowlink: vec![0; n],
        result: Vec::new(),
    };

    for v in 0..n {
        if state.index[v].is_none() {
            strongconnect(v, adj, &mut state);
        }
    }

    state.result
}

/// Generate all permutations of a slice.
fn permutations(items: &[usize]) -> Vec<Vec<usize>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut result = Vec::new();
    for (i, &item) in items.iter().enumerate() {
        let rest: Vec<usize> = items[..i]
            .iter()
            .chain(items[i + 1..].iter())
            .copied()
            .collect();
        for mut perm in permutations(&rest) {
            perm.insert(0, item);
            result.push(perm);
        }
    }
    result
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

    #[test]
    fn test_tarjan_single_scc() {
        // A -> B -> A (cycle)
        let adj = vec![vec![1], vec![0]];
        let sccs = tarjan_scc(2, &adj);
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0].len(), 2);
    }

    #[test]
    fn test_tarjan_no_cycles() {
        // A -> B, no cycle
        let adj = vec![vec![1], vec![]];
        let sccs = tarjan_scc(2, &adj);
        assert_eq!(sccs.len(), 2);
    }

    #[test]
    fn test_permutations() {
        let perms = permutations(&[0, 1, 2]);
        assert_eq!(perms.len(), 6);
    }
}
