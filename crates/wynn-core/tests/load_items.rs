use std::path::Path;
use wynn_core::db::ItemDb;
use wynn_core::item::{Item, ItemCategory};

#[test]
fn test_load_item_database() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("data/items.json");

    if !path.exists() {
        eprintln!("skipping test: items.json not found at {}", path.display());
        return;
    }

    let db = ItemDb::load_from_file(&path).expect("failed to load item DB");

    // Should have loaded many items
    assert!(
        db.items_by_id.len() > 1000,
        "expected >1000 items, got {}",
        db.items_by_id.len()
    );

    // Check a known mythic weapon: Idol (spear)
    let idol = db.get_by_name("idol");
    assert!(idol.is_some(), "Idol should exist in database");
    if let Some(Item::Weapon(w)) = idol {
        assert_eq!(w.weapon_type, wynn_core::stats::WeaponType::Spear);
        assert_eq!(w.tier, wynn_core::item::ItemTier::Mythic);
        println!("Idol: level={}, atkSpd={:?}, sp_req={:?}", w.level, w.attack_speed, w.requirements);
    } else {
        panic!("Idol should be a weapon");
    }

    // Check a known armour: Blue Mask (helmet)
    let blue_mask = db.get_by_name("blue mask");
    assert!(blue_mask.is_some(), "Blue Mask should exist in database");
    if let Some(Item::Apparel(a)) = blue_mask {
        assert_eq!(a.category, ItemCategory::Helmet);
        println!("Blue Mask: level={}, hp={}, sp_req={:?}", a.level, a.hp, a.requirements);
    } else {
        panic!("Blue Mask should be apparel");
    }

    // Check categories have items
    assert!(
        !db.apparels_in_category(ItemCategory::Helmet).is_empty(),
        "should have helmets"
    );
    assert!(!db.weapon_ids.is_empty(), "should have weapons");

    println!(
        "Total: {} items, {} weapons, {} helmets, {} rings",
        db.items_by_id.len(),
        db.weapon_ids.len(),
        db.apparels_in_category(ItemCategory::Helmet).len(),
        db.apparels_in_category(ItemCategory::Ring).len(),
    );
}
