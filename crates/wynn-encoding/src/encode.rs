use wynn_core::build::Build;
use wynn_core::item::Slot;
use wynn_core::stats::Element;

use crate::charset::BitVec;
use crate::versioned_consts::{consts_for_version, BINARY_FORMAT_FLAG, VERSION_NAMES};

/// Encode a Build into a WynnBuilder URL hash.
///
/// Uses the latest version binary format.
pub fn encode_build(build: &Build) -> String {
    let version = VERSION_NAMES.len() - 1;
    let consts = consts_for_version(version);
    let mut bv = BitVec::new();

    // Header
    bv.write_bits(BINARY_FORMAT_FLAG, 6);
    bv.write_bits(version as u64, 10);

    // Equipment: 9 slots
    for (slot_idx, slot) in Slot::ALL.iter().enumerate() {
        let item = build.item(*slot);
        match &item {
            Some(item) => {
                bv.write_bits(0, 2); // NORMAL kind
                bv.write_bits((item.id() + 1) as u64, consts.item_id_bitlen);
            }
            None => {
                bv.write_bits(0, 2); // NORMAL kind
                bv.write_bits(0, consts.item_id_bitlen); // empty slot
            }
        }

        // Powders
        if slot.is_powderable() {
            let powder_idx = match slot_idx {
                0..=3 => slot_idx,
                8 => 4,
                _ => continue,
            };
            let powders = &build.powders[powder_idx];
            if powders.is_empty() {
                bv.write_bit(false); // NO_POWDERS
            } else {
                bv.write_bit(true); // HAS_POWDERS
                encode_powders(&mut bv, powders, &consts);
            }
        }
    }

    // Tomes: none for now
    bv.write_bit(false); // NO_TOMES

    // Skill Points
    if let Some(sp) = &build.assigned_sp {
        bv.write_bit(false); // ASSIGNED (not automatic)
        for elem in Element::ALL {
            let val = sp.get(elem);
            if val != 0 {
                bv.write_bit(true); // ELEMENT_ASSIGNED
                bv.write_signed(val as i64, consts.max_sp_bitlen);
            } else {
                bv.write_bit(false); // ELEMENT_UNASSIGNED
            }
        }
    } else {
        bv.write_bit(true); // AUTOMATIC
    }

    // Level
    if build.level == consts.max_level {
        bv.write_bit(false); // MAX level
    } else {
        bv.write_bit(true); // OTHER
        bv.write_bits(build.level as u64, consts.level_bitlen);
    }

    // Aspects: none for now
    if consts.num_aspects > 0 {
        bv.write_bit(false); // NO_ASPECTS
    }

    // No ability tree for now

    bv.to_hash()
}

fn encode_powders(
    bv: &mut BitVec,
    powders: &[wynn_core::stats::Powder],
    consts: &crate::versioned_consts::EncodingConsts,
) {
    if powders.is_empty() {
        return;
    }

    // Write first powder
    let first_id = powders[0].to_id(consts.powder_tiers);
    bv.write_bits(first_id as u64, consts.powder_id_bitlen);

    for i in 1..powders.len() {
        let prev = &powders[i - 1];
        let curr = &powders[i];

        if curr == prev {
            // REPEAT
            bv.write_bit(false);
        } else if curr.tier == prev.tier {
            // REPEAT_TIER
            bv.write_bit(true);
            bv.write_bit(false);
            let wrap = ((curr.element.index() as u32 + consts.powder_elements as u32
                - prev.element.index() as u32
                - 1)
                % consts.powder_elements as u32) as u64;
            bv.write_bits(wrap, consts.powder_wrapper_bitlen);
        } else {
            // NEW_POWDER
            bv.write_bit(true);
            bv.write_bit(true);
            bv.write_bit(false);
            let pid = curr.to_id(consts.powder_tiers);
            bv.write_bits(pid as u64, consts.powder_id_bitlen);
        }
    }

    // NEW_ITEM terminator
    bv.write_bit(true);
    bv.write_bit(true);
    bv.write_bit(true);
}
