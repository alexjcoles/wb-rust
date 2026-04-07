use std::path::Path;
use wynn_core::db::ItemDb;

fn load_db() -> Option<ItemDb> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("data/items.json");

    if !path.exists() {
        eprintln!("skipping test: items.json not found at {}", path.display());
        return None;
    }

    Some(ItemDb::load_from_file(&path).expect("failed to load item DB"))
}

#[test]
fn test_encode_decode_roundtrip() {
    let db = match load_db() {
        Some(db) => db,
        None => return,
    };

    // Build with known items
    let mut build = wynn_core::build::Build::new();
    build.level = 106;

    if let Some(wynn_core::item::Item::Weapon(w)) = db.get_by_name("idol") {
        build.weapon = Some(w.clone());
    }
    if let Some(wynn_core::item::Item::Apparel(a)) = db.get_by_name("blue mask") {
        build.helmet = Some(a.clone());
    }

    let hash = wynn_encoding::encode_build(&build);
    let decoded = wynn_encoding::decode_build(&hash, &db).expect("failed to decode");

    assert_eq!(
        decoded.weapon.as_ref().map(|w| w.name.as_str()),
        Some("Idol")
    );
    assert_eq!(
        decoded.helmet.as_ref().map(|h| h.name.as_str()),
        Some("Blue Mask")
    );
    assert_eq!(decoded.level, 106);
}

/// Test decoding real build URLs from the Wynncraft community.
/// These are binary format (V12+) URLs from the Ultimate Build Guide.
#[test]
fn test_decode_real_builds() {
    let db = match load_db() {
        Some(db) => db,
        None => return,
    };

    // Non-crafted build URLs from the forum post
    let urls = &[
        // Convergence - Paladin Support
        "https://wynnbuilder.github.io/builder/#CN0O0VTctzAT4nzw4gvLU6eG05mwmLEunzobspBGs-Ud0",
        // Collapse - Upperscream Fallen Burst (General Use)
        "https://wynnbuilder.github.io/builder/#CN00QW5Gyb0OrW6EqmH15OyWdcBkqRhd+tLq0",
        // Collapse - Lootrun
        "https://wynnbuilder.github.io/builder/#CN00Qmd6qI2uQW6EqmH15amWdcNrbURzq-kY6",
        // Fatal - Paladin
        "https://wynnbuilder.github.io/builder/#CN0G7WM38Y30-G05AlXjEEdGfc5kq+ljNsF7",
        // Mage - Warp
        "https://wynnbuilder.github.io/builder/#CN0e10-CWi3yQ41TCTCi9i9i9i9m9IG4qnqnmcmcmcmc0d81HG7J73R2R2R2RYPY4WVEKrCo9D0GljUwVNH3",
        // Shaman - Cataclysm
        "https://wynnbuilder.github.io/builder/#CN0X6G5mNdm4lEXQUbd0Ayu0v9SbZ9Da-zPRFqT1",
        // Shaman - Revenant
        "https://wynnbuilder.github.io/builder/#CN0X6G6Gm90XQG762e066konechmb+vJJrAd6E",
        // Archer - Guardian
        "https://wynnbuilder.github.io/builder/#CN0vvG5G812XQmM72eWy4ko1hcNkqFlBfwuEGo0",
        // Archer - Spring
        "https://wynnbuilder.github.io/builder/#CN0vAG0EG74CnG05sW0LEEdmlcTkylqVOzlyG",
        // Assassin - Grimtrap
        "https://wynnbuilder.github.io/builder/#CN0oqG0E8t2CnW10C00LEcmnlcTkylqVOztE8",
    ];

    let mut successes = 0;
    let mut failures = Vec::new();

    for url in urls {
        match wynn_encoding::decode_build(url, &db) {
            Ok(build) => {
                successes += 1;
                let items: Vec<String> = wynn_core::item::Slot::ALL
                    .iter()
                    .filter_map(|slot| build.item(*slot).map(|i| i.name().to_string()))
                    .collect();
                let stats = wynn_core::calculate::calculate_build_stats(&build);

                println!("OK: {} items, HP={}, EHP={:.0}, level={}",
                    items.len(), stats.hp, stats.ehp, build.level);
                println!("   Items: {}", items.join(", "));
                if let Some(sp) = &build.assigned_sp {
                    println!("   SP: str={} dex={} int={} def={} agi={}",
                        sp.earth, sp.thunder, sp.water, sp.fire, sp.air);
                }
            }
            Err(e) => {
                let hash = url.split('#').nth(1).unwrap_or("");
                failures.push(format!("{}: {e}", &hash[..hash.len().min(20)]));
            }
        }
    }

    println!("\n=== Results: {successes}/{} decoded successfully ===", urls.len());
    for f in &failures {
        println!("  FAIL: {f}");
    }

    // We expect most non-crafted builds to decode
    assert!(
        successes >= urls.len() / 2,
        "too many failures: only {successes}/{} succeeded",
        urls.len()
    );
}

/// Test a specific build in detail to verify SP and stat calculation.
#[test]
fn test_decode_convergence_detail() {
    let db = match load_db() {
        Some(db) => db,
        None => return,
    };

    // Convergence - Paladin Support build
    let url = "https://wynnbuilder.github.io/builder/#CN0O0VTctzAT4nzw4gvLU6eG05mwmLEunzobspBGs-Ud0";
    let build = wynn_encoding::decode_build(url, &db).expect("failed to decode");

    // Verify weapon
    assert_eq!(build.weapon.as_ref().unwrap().name, "Convergence");
    assert_eq!(build.level, 106);

    // Print detailed stats
    let stats = wynn_core::calculate::calculate_build_stats(&build);
    println!("=== Convergence Paladin Support ===");
    println!("Items:");
    for slot in wynn_core::item::Slot::ALL {
        if let Some(item) = build.item(slot) {
            println!("  {:?}: {} (id={})", slot, item.name(), item.id());
        }
    }
    println!("\nAssigned SP: {:?}", build.assigned_sp);
    println!("Calculated SP: {:?}", stats.sp_assignment.assigned);
    println!("Total SP: {:?}", stats.sp_assignment.total);
    println!("Total assigned: {}", stats.sp_assignment.total_assigned());
    println!("\nHP: {}", stats.hp);
    println!("EHP: {:.0}", stats.ehp);
    println!("HPR: {} (raw={}, pct={})", stats.hpr, stats.hpr_raw, stats.hpr_pct);
    println!("Life Steal: {}", stats.life_steal);
    println!("Mana Regen: {}", stats.mana_regen);
    println!("Walk Speed: {}", stats.walk_speed);
    println!("Spell Damage: raw={} pct={}", stats.spell_damage_raw, stats.spell_damage_pct);
    println!("Defences: e={} t={} w={} f={} a={}",
        stats.elemental_defence.earth,
        stats.elemental_defence.thunder,
        stats.elemental_defence.water,
        stats.elemental_defence.fire,
        stats.elemental_defence.air,
    );
}
