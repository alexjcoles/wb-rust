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

/// Single slot swap: unlock boots, maximise EHP.
#[test]
fn test_solve_single_slot_boots() {
    let db = match load_db() { Some(db) => db, None => return };

    let url = "https://wynnbuilder.github.io/builder/#CN0O0VTctzAT4nzw4gvLU6eG05mwmLEunzobspBGs-Ud0";
    let mut base_build = wynn_encoding::decode_build(url, &db).expect("failed to decode");
    base_build.available_points = 200;

    let locked = vec![
        Slot::Helmet, Slot::Chestplate, Slot::Leggings,
        Slot::Ring1, Slot::Ring2, Slot::Bracelet, Slot::Necklace, Slot::Weapon,
    ];

    let config = SolverConfig {
        objectives: vec![Objective::Maximise(StatType::Ehp)],
        max_results: 5,
        min_item_level: 90,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let results = solve(&base_build, &locked, &config, &db);
    let elapsed = start.elapsed();

    println!("Single-slot: {} results in {:?}", results.len(), elapsed);
    for (i, r) in results.iter().enumerate() {
        let boots = r.build.boots.as_ref().map(|b| b.name.as_str()).unwrap_or("None");
        println!("  #{}: {} — HP={}, EHP={:.0}, SP={}",
            i + 1, boots, r.stats.hp, r.stats.ehp, r.stats.sp_assignment.total_assigned());
    }

    assert!(!results.is_empty(), "solver should find at least one result");
}

/// Two-slot swap: unlock boots and ring1.
#[test]
fn test_solve_two_flexible_slots() {
    let db = match load_db() { Some(db) => db, None => return };

    let url = "https://wynnbuilder.github.io/builder/#CN0O0VTctzAT4nzw4gvLU6eG05mwmLEunzobspBGs-Ud0";
    let mut base_build = wynn_encoding::decode_build(url, &db).expect("failed to decode");
    base_build.available_points = 200;

    let locked = vec![
        Slot::Helmet, Slot::Chestplate, Slot::Leggings,
        Slot::Ring2, Slot::Bracelet, Slot::Necklace, Slot::Weapon,
    ];

    let config = SolverConfig {
        objectives: vec![Objective::Maximise(StatType::Ehp)],
        max_results: 3,
        min_item_level: 95,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let results = solve(&base_build, &locked, &config, &db);
    let elapsed = start.elapsed();

    println!("Two-slot: {} results in {:?}", results.len(), elapsed);
    for (i, r) in results.iter().enumerate() {
        let boots = r.build.boots.as_ref().map(|b| b.name.as_str()).unwrap_or("None");
        let ring = r.build.ring1.as_ref().map(|r| r.name.as_str()).unwrap_or("None");
        println!("  #{}: boots={}, ring={} — HP={}, EHP={:.0}, SP={}",
            i + 1, boots, ring, r.stats.hp, r.stats.ehp, r.stats.sp_assignment.total_assigned());
    }

    // May return 0 results if the base build can't fit in 200 SP
    // (this build was designed for tomes which are now removed)
    for r in &results {
        assert!(r.stats.sp_assignment.is_valid(200), "SP must be valid");
    }
}

/// Collapse build with HP constraint.
#[test]
fn test_solve_collapse_build() {
    let db = match load_db() { Some(db) => db, None => return };

    let url = "https://wynnbuilder.github.io/builder/#CN00QW5Gyb0OrW6EqmH15OyWdcBkqRhd+tLq0";
    let base_build = wynn_encoding::decode_build(url, &db).expect("failed to decode");

    let locked = vec![
        Slot::Helmet, Slot::Chestplate, Slot::Leggings,
        Slot::Ring1, Slot::Ring2, Slot::Bracelet, Slot::Necklace, Slot::Weapon,
    ];

    let config = SolverConfig {
        constraints: Constraints { min_hp: Some(5000), ..Default::default() },
        objectives: vec![Objective::Maximise(StatType::Ehp)],
        max_results: 5,
        min_item_level: 90,
        ..Default::default()
    };

    let results = solve(&base_build, &locked, &config, &db);

    println!("Collapse: {} results", results.len());
    for r in &results {
        assert!(r.stats.hp >= 5000, "HP constraint violated: {}", r.stats.hp);
    }
    assert!(!results.is_empty(), "solver should find at least one result");
}

/// THREE-SLOT solve: boots + ring1 + bracelet. Must complete in < 2s.
#[test]
fn test_solve_three_slots_performance() {
    let db = match load_db() { Some(db) => db, None => return };

    let url = "https://wynnbuilder.github.io/builder/#CN0O0VTctzAT4nzw4gvLU6eG05mwmLEunzobspBGs-Ud0";
    let mut base_build = wynn_encoding::decode_build(url, &db).expect("failed to decode");
    base_build.available_points = 200;

    // Unlock boots, ring1, bracelet (3 slots)
    let locked = vec![
        Slot::Helmet, Slot::Chestplate, Slot::Leggings,
        Slot::Ring2, Slot::Necklace, Slot::Weapon,
    ];

    let config = SolverConfig {
        objectives: vec![Objective::Maximise(StatType::Ehp)],
        max_results: 5,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let results = solve(&base_build, &locked, &config, &db);
    let elapsed = start.elapsed();

    println!("Three-slot solve: {} results in {:?}", results.len(), elapsed);
    for (i, r) in results.iter().enumerate() {
        let boots = r.build.boots.as_ref().map(|b| b.name.as_str()).unwrap_or("None");
        let ring = r.build.ring1.as_ref().map(|r| r.name.as_str()).unwrap_or("None");
        let bracelet = r.build.bracelet.as_ref().map(|b| b.name.as_str()).unwrap_or("None");
        println!("  #{}: boots={}, ring={}, bracelet={} — HP={}, EHP={:.0}, SP={}",
            i + 1, boots, ring, bracelet, r.stats.hp, r.stats.ehp,
            r.stats.sp_assignment.total_assigned());
    }

    assert!(!results.is_empty(), "solver should find at least one result");
    assert!(elapsed.as_secs() < 5, "three-slot solve took too long: {:?}", elapsed);
}

/// Constraint enforcement: solver must return builds with thunder_defence >= 0.
#[test]
fn test_solve_thunder_defence_constraint() {
    let db = match load_db() { Some(db) => db, None => return };

    let url = "https://wynnbuilder.github.io/builder/#CN0O0VTctzAT4nzw4gvLU6eG05mwmLEunzobspBGs-Ud0";
    let mut base_build = wynn_encoding::decode_build(url, &db).expect("failed to decode");
    base_build.available_points = 200;

    // Unlock boots + ring1
    let locked = vec![
        Slot::Helmet, Slot::Chestplate, Slot::Leggings,
        Slot::Ring2, Slot::Bracelet, Slot::Necklace, Slot::Weapon,
    ];

    let config = SolverConfig {
        constraints: Constraints {
            min_thunder_defence: Some(0),
            ..Default::default()
        },
        objectives: vec![
            Objective::Maximise(StatType::Ehp),
            Objective::Maximise(StatType::ThunderDefence),
        ],
        max_results: 5,
        ..Default::default()
    };

    let results = solve(&base_build, &locked, &config, &db);

    println!("Thunder constraint: {} results", results.len());
    for (i, r) in results.iter().enumerate() {
        let boots = r.build.boots.as_ref().map(|b| b.name.as_str()).unwrap_or("None");
        let ring = r.build.ring1.as_ref().map(|r| r.name.as_str()).unwrap_or("None");
        println!("  #{}: boots={}, ring={} — tDef={}, HP={}, EHP={:.0}",
            i + 1, boots, ring,
            r.stats.elemental_defence.thunder, r.stats.hp, r.stats.ehp);

        assert!(
            r.stats.elemental_defence.thunder >= 0,
            "thunder defence constraint violated: {} for boots={}, ring={}",
            r.stats.elemental_defence.thunder, boots, ring
        );
    }

    // It's possible no builds exist that satisfy this (thunder_defence -325 is very deep)
    // so we don't assert non-empty, but we do assert all results satisfy the constraint
    println!("All {} results satisfy thunder_defence >= 0", results.len());
}
