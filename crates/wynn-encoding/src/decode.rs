use wynn_core::build::Build;
use wynn_core::db::ItemDb;
use wynn_core::item::{Apparel, Item};
use wynn_core::stats::{Element, Powder, SkillPoints};

use crate::charset::{BitVec, DecodeError};
use crate::versioned_consts::{consts_for_version, BINARY_FORMAT_FLAG};

/// Decode a WynnBuilder URL into a Build.
///
/// Supports both URL formats:
/// - `https://hppeng-wynn.github.io/builder/#<hash>`
/// - `https://wynnbuilder.github.io/builder/#<hash>`
pub fn decode_build(url: &str, db: &ItemDb) -> Result<Build, DecodeError> {
    let hash = extract_hash(url)?;

    // Check if this is the binary format (V12+)
    let first_char = hash.as_bytes().first().ok_or(DecodeError::UnexpectedEnd)?;
    let first_val = crate::charset::char_to_index(*first_char)
        .ok_or(DecodeError::InvalidChar(*first_char as char))?;

    if first_val as u64 >= BINARY_FORMAT_FLAG {
        decode_binary(hash, db)
    } else {
        decode_legacy(hash, db)
    }
}

/// Extract the hash fragment from a WynnBuilder URL.
fn extract_hash(url: &str) -> Result<&str, DecodeError> {
    // Handle both full URLs and bare hashes
    if let Some(hash) = url.strip_prefix("https://hppeng-wynn.github.io/builder/#") {
        Ok(hash)
    } else if let Some(hash) = url.strip_prefix("https://wynnbuilder.github.io/builder/#") {
        Ok(hash)
    } else if let Some(hash) = url.strip_prefix("http://hppeng-wynn.github.io/builder/#") {
        Ok(hash)
    } else if let Some(hash) = url.strip_prefix("http://wynnbuilder.github.io/builder/#") {
        Ok(hash)
    } else if url.contains('#') {
        // Try to extract hash from any URL
        Ok(url.split('#').nth(1).unwrap_or(""))
    } else {
        // Assume bare hash
        Ok(url)
    }
}

/// Decode a V12 binary format hash.
fn decode_binary(hash: &str, db: &ItemDb) -> Result<Build, DecodeError> {
    let mut bv = BitVec::from_hash(hash)?;

    // Header: 6-bit legacy flag + 10-bit version
    let _legacy = bv.read_bits(6)?;
    let version = bv.read_bits(10)? as usize;
    let consts = consts_for_version(version);

    let mut build = Build::new();

    // Equipment: 9 items
    let mut powders_by_slot: Vec<Vec<Powder>> = Vec::new();

    for slot_idx in 0..consts.equipment_num {
        let kind = bv.read_bits(2)?;

        match kind {
            0 => {
                // NORMAL item
                let item_id = bv.read_bits(consts.item_id_bitlen)? as u32;
                if item_id > 0 {
                    let actual_id = item_id - 1;
                    match db.get_by_id(actual_id) {
                        Some(Item::Apparel(apparel)) => {
                            set_apparel(&mut build, slot_idx, apparel.clone());
                        }
                        Some(Item::Weapon(weapon)) => {
                            build.weapon = Some(weapon.clone());
                        }
                        None => {
                            tracing::warn!("item ID {actual_id} not found in database, skipping slot {slot_idx}");
                        }
                    }
                }
                // else: empty slot (id == 0)
            }
            1 => {
                // CRAFTED - skip for now (complex sub-encoding)
                tracing::warn!("crafted items not yet supported, skipping slot {slot_idx}");
                // TODO: implement crafted item decoding
                // For now we'd need to know the sub-format length to skip it
                return Err(DecodeError::UnsupportedEquipmentKind(1));
            }
            2 => {
                // CUSTOM - read length then skip
                let len_chars = bv.read_bits(12)? as usize;
                let _skip = bv.read_bits(len_chars * 6)?;
                tracing::warn!("custom items not yet supported, skipping slot {slot_idx}");
            }
            other => return Err(DecodeError::UnsupportedEquipmentKind(other)),
        }

        // Powders (only for powderable slots: 0-3 = armour, 8 = weapon)
        let is_powderable = matches!(slot_idx, 0..=3 | 8);
        if is_powderable {
            let has_powders = bv.read_bit()?;
            if has_powders {
                let powders = decode_powders(&mut bv, &consts)?;
                powders_by_slot.push(powders);
            } else {
                powders_by_slot.push(Vec::new());
            }
        }
    }

    // Store powders in build (order: helmet, chest, legs, boots, weapon)
    for (i, powders) in powders_by_slot.into_iter().enumerate() {
        if i < 5 {
            build.powders[i] = powders;
        }
    }

    // Tomes
    let has_tomes = bv.read_bit()?;
    if has_tomes {
        for _tome_idx in 0..consts.tome_num {
            let used = bv.read_bit()?;
            if used {
                let _tome_id = bv.read_bits(consts.tome_id_bitlen)?;
                // TODO: store tome data
            }
        }
    }

    // Skill Points
    let automatic_sp = bv.read_bit()?;
    if !automatic_sp {
        let mut sp = SkillPoints::default();
        for elem in Element::ALL.iter() {
            let assigned = bv.read_bit()?;
            if assigned {
                let val = bv.read_signed(consts.max_sp_bitlen)? as i32;
                sp.set(*elem, val);
            }
        }
        build.assigned_sp = Some(sp);
    }

    // Level
    let is_max_level = !bv.read_bit()?;
    if is_max_level {
        build.level = consts.max_level;
    } else {
        build.level = bv.read_bits(consts.level_bitlen)? as u32;
    }

    // Aspects (skip for now if present)
    if consts.num_aspects > 0 && bv.remaining() > 0 {
        let has_aspects = bv.read_bit().unwrap_or(false);
        if has_aspects {
            for _ in 0..consts.num_aspects {
                let used = bv.read_bit().unwrap_or(false);
                if used {
                    let _aspect_id = bv.read_bits(consts.aspect_id_bitlen).unwrap_or(0);
                    let _aspect_tier = bv.read_bits(consts.aspect_tier_bitlen).unwrap_or(0);
                }
            }
        }
    }

    // Remaining bits are ability tree (skip for now)

    Ok(build)
}

/// Decode a legacy format hash (version <= 11).
fn decode_legacy(hash: &str, db: &ItemDb) -> Result<Build, DecodeError> {
    // Legacy format: version_items:27_sp:10_level:2_powder:var_tomes:16_ability:rest
    // Split on underscore to handle version prefix
    let parts: Vec<&str> = hash.splitn(2, '_').collect();
    let (_version_str, data) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        // No underscore - try the whole string
        ("", hash)
    };

    // Decode data characters using the old from_int_n / to_int scheme
    let chars: Vec<u8> = data.as_bytes().to_vec();

    let mut build = Build::new();

    // Items: 9 x 3 chars each = 27 chars
    if chars.len() < 27 {
        return Err(DecodeError::UnexpectedEnd);
    }

    for slot_idx in 0..9 {
        let start = slot_idx * 3;
        let item_id = legacy_to_int(&chars[start..start + 3])? as u32;
        if item_id > 0 {
            match db.get_by_id(item_id) {
                Some(Item::Apparel(apparel)) => {
                    set_apparel(&mut build, slot_idx, apparel.clone());
                }
                Some(Item::Weapon(weapon)) => {
                    build.weapon = Some(weapon.clone());
                }
                None => {
                    tracing::warn!("legacy: item ID {item_id} not found");
                }
            }
        }
    }

    // Skill points: 5 x 2 chars = 10 chars
    let sp_start = 27;
    if chars.len() >= sp_start + 10 {
        let mut sp = SkillPoints::default();
        for (i, elem) in Element::ALL.iter().enumerate() {
            let start = sp_start + i * 2;
            let val = legacy_to_int(&chars[start..start + 2])? as i32;
            sp.set(*elem, val);
        }
        build.assigned_sp = Some(sp);
    }

    // Level: 2 chars
    let level_start = sp_start + 10;
    if chars.len() >= level_start + 2 {
        build.level = legacy_to_int(&chars[level_start..level_start + 2])? as u32;
    }

    // Powders and tomes are in the remaining data - skip for now
    // TODO: decode legacy powders and tomes

    Ok(build)
}

/// Convert legacy base-64 chars to integer (MSB first).
fn legacy_to_int(chars: &[u8]) -> Result<u64, DecodeError> {
    let mut result = 0u64;
    for &c in chars {
        let idx = crate::charset::char_to_index(c)
            .ok_or(DecodeError::InvalidChar(c as char))?;
        result = (result << 6) | idx as u64;
    }
    Ok(result)
}

/// Decode powders from the binary format.
fn decode_powders(
    bv: &mut BitVec,
    consts: &crate::versioned_consts::EncodingConsts,
) -> Result<Vec<Powder>, DecodeError> {
    let mut powders = Vec::new();

    // Read first powder ID
    let first_id = bv.read_bits(consts.powder_id_bitlen)? as u32;
    let first_powder = version_decode_powder(first_id, consts.powder_tiers);
    powders.push(Powder::from_id(first_powder, consts.powder_tiers));

    loop {
        let op = bv.read_bit()?;
        if !op {
            // REPEAT: copy previous powder
            let last = *powders.last().unwrap();
            powders.push(last);
        } else {
            let tier_op = bv.read_bit()?;
            if !tier_op {
                // REPEAT_TIER: same tier, different element (wrapped)
                let last = powders.last().unwrap();
                let last_elem = last.element.index() as u32;
                let last_tier = last.tier;
                let wrap = bv.read_bits(consts.powder_wrapper_bitlen)? as u32;
                let new_elem = ((last_elem + wrap + 1) % consts.powder_elements as u32) as usize;
                powders.push(Powder {
                    element: Element::ALL[new_elem],
                    tier: last_tier,
                });
            } else {
                let change_op = bv.read_bit()?;
                if !change_op {
                    // NEW_POWDER: completely new powder
                    let new_id = bv.read_bits(consts.powder_id_bitlen)? as u32;
                    let decoded = version_decode_powder(new_id, consts.powder_tiers);
                    powders.push(Powder::from_id(decoded, consts.powder_tiers));
                } else {
                    // NEW_ITEM: stop decoding powders for this item
                    break;
                }
            }
        }
    }

    Ok(powders)
}

/// Version-correct a powder ID (when decoding from a version with different tier count).
fn version_decode_powder(encoded: u32, _current_tiers: u8) -> u32 {
    // For now, assume same tier count (no version mismatch)
    // Full formula: decoded = encoded + (encoded / old_tiers) * (current_tiers - old_tiers)
    encoded
}

fn set_apparel(build: &mut Build, slot_idx: usize, apparel: Apparel) {
    match slot_idx {
        0 => build.helmet = Some(apparel),
        1 => build.chestplate = Some(apparel),
        2 => build.leggings = Some(apparel),
        3 => build.boots = Some(apparel),
        4 => build.ring1 = Some(apparel),
        5 => build.ring2 = Some(apparel),
        6 => build.bracelet = Some(apparel),
        7 => build.necklace = Some(apparel),
        _ => {} // slot 8 is weapon, handled separately
    }
}
