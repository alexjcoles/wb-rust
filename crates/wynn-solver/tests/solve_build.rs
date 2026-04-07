use std::path::Path;
use wynn_core::db::ItemDb;
use wynn_core::item::Slot;
use wynn_solver::constraints::{Constraints, Objective, StatType};
use wynn_solver::search::{solve, SolverConfig};

fn load_db() -> Option<ItemDb> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("data/items.json");

    if !path.exists() {
        eprintln!("skipping test: items.json not found");
        return None;
    }

    Some(ItemDb::load_from_file(&path).expect("failed to load item DB"))
}

/// Take the Convergence build, unlock boots, maximise EHP.
/// No hard constraints beyond what the solver already checks (SP validity).
#[test]
fn test_solve_single_slot_boots() {
    let db = match load_db() {
        Some(db) => db,
        None => return,
    };

    let url = "https://wynnbuilder.github.io/builder/#CN0O0VTctzAT4nzw4gvLU6eG05mwmLEunzobspBGs-Ud0";
    let mut base_build = wynn_encoding::decode_build(url, &db).expect("failed to decode");
    // Raise SP budget to account for tomes
    base_build.available_points = 250;

    let original_stats = wynn_core::calculate::calculate_build_stats(&base_build);
    println!("Original: boots={}, HP={}, EHP={:.0}",
        base_build.boots.as_ref().unwrap().name,
        original_stats.hp, original_stats.ehp);

    let locked = vec![
        Slot::Helmet, Slot::Chestplate, Slot::Leggings,
        Slot::Ring1, Slot::Ring2, Slot::Bracelet, Slot::Necklace, Slot::Weapon,
    ];

    let config = SolverConfig {
        constraints: Constraints::default(), // No extra constraints
        objectives: vec![Objective::Maximise(StatType::Ehp)],
        max_results: 5,
        min_item_level: 90,
    };

    let start = std::time::Instant::now();
    let results = solve(&base_build, &locked, &config, &db);
    let elapsed = start.elapsed();

    println!("\nSolver found {} results in {:?}:", results.len(), elapsed);
    for (i, r) in results.iter().enumerate() {
        let boots = r.build.boots.as_ref().map(|b| b.name.as_str()).unwrap_or("None");
        println!("  #{}: {} — HP={}, EHP={:.0}, tDef={}, SP={}",
            i + 1, boots, r.stats.hp, r.stats.ehp,
            r.stats.elemental_defence.thunder,
            r.stats.sp_assignment.total_assigned());
    }

    assert!(!results.is_empty(), "solver should find at least one result");
}

/// Solve with two flexible slots.
#[test]
fn test_solve_two_flexible_slots() {
    let db = match load_db() {
        Some(db) => db,
        None => return,
    };

    let url = "https://wynnbuilder.github.io/builder/#CN0O0VTctzAT4nzw4gvLU6eG05mwmLEunzobspBGs-Ud0";
    let mut base_build = wynn_encoding::decode_build(url, &db).expect("failed to decode");
    base_build.available_points = 250;

    // Unlock boots and ring1
    let locked = vec![
        Slot::Helmet, Slot::Chestplate, Slot::Leggings,
        Slot::Ring2, Slot::Bracelet, Slot::Necklace, Slot::Weapon,
    ];

    let config = SolverConfig {
        constraints: Constraints::default(),
        objectives: vec![Objective::Maximise(StatType::Ehp)],
        max_results: 3,
        min_item_level: 95,
    };

    let start = std::time::Instant::now();
    let results = solve(&base_build, &locked, &config, &db);
    let elapsed = start.elapsed();

    println!("Two-slot solve: {} results in {:?}", results.len(), elapsed);
    for (i, r) in results.iter().enumerate() {
        let boots = r.build.boots.as_ref().map(|b| b.name.as_str()).unwrap_or("None");
        let ring = r.build.ring1.as_ref().map(|r| r.name.as_str()).unwrap_or("None");
        println!("  #{}: boots={}, ring={} — HP={}, EHP={:.0}, SP={}",
            i + 1, boots, ring, r.stats.hp, r.stats.ehp,
            r.stats.sp_assignment.total_assigned());
    }

    assert!(!results.is_empty(), "solver should find results");
}

/// Use a less constrained build (Collapse) with lower SP requirements.
#[test]
fn test_solve_collapse_build() {
    let db = match load_db() {
        Some(db) => db,
        None => return,
    };

    let url = "https://wynnbuilder.github.io/builder/#CN00QW5Gyb0OrW6EqmH15OyWdcBkqRhd+tLq0";
    let base_build = wynn_encoding::decode_build(url, &db).expect("failed to decode");

    let original_stats = wynn_core::calculate::calculate_build_stats(&base_build);
    println!("Collapse build: HP={}, EHP={:.0}, SP={}",
        original_stats.hp, original_stats.ehp,
        original_stats.sp_assignment.total_assigned());

    // Unlock boots only
    let locked = vec![
        Slot::Helmet, Slot::Chestplate, Slot::Leggings,
        Slot::Ring1, Slot::Ring2, Slot::Bracelet, Slot::Necklace, Slot::Weapon,
    ];

    let config = SolverConfig {
        constraints: Constraints {
            min_hp: Some(5000),
            ..Default::default()
        },
        objectives: vec![Objective::Maximise(StatType::Ehp)],
        max_results: 5,
        min_item_level: 90,
    };

    let start = std::time::Instant::now();
    let results = solve(&base_build, &locked, &config, &db);
    let elapsed = start.elapsed();

    println!("\nCollapse solver: {} results in {:?}", results.len(), elapsed);
    for (i, r) in results.iter().enumerate() {
        let boots = r.build.boots.as_ref().map(|b| b.name.as_str()).unwrap_or("None");
        println!("  #{}: {} — HP={}, EHP={:.0}, SP={}",
            i + 1, boots, r.stats.hp, r.stats.ehp,
            r.stats.sp_assignment.total_assigned());
    }

    assert!(!results.is_empty(), "solver should find at least one result");
}
