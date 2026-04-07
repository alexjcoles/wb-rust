/// Version-dependent encoding constants.
/// These control bit widths for different sections of the hash.
#[derive(Debug, Clone)]
pub struct EncodingConsts {
    pub item_id_bitlen: usize,
    pub tome_id_bitlen: usize,
    pub aspect_id_bitlen: usize,
    pub aspect_tier_bitlen: usize,
    pub powder_id_bitlen: usize,
    pub powder_wrapper_bitlen: usize,
    pub powder_tiers: u8,
    pub powder_elements: usize,
    pub max_sp_bitlen: usize,
    pub level_bitlen: usize,
    pub max_level: u32,
    pub equipment_num: usize,
    pub tome_num: usize,
    pub num_aspects: usize,
}

/// Wynncraft version names, indexed by version ID in the hash.
pub const VERSION_NAMES: &[&str] = &[
    "2.0.1.1", "2.0.1.2", "2.0.2.1", "2.0.2.3",
    "2.0.3.1", "2.0.4.1", "2.0.4.3", "2.0.4.4",
    "2.1.0.0", "2.1.0.1", "2.1.1.0", "2.1.1.1",
    "2.1.1.2", "2.1.1.3", "2.1.1.4", "2.1.1.5",
    "2.1.1.6", "2.1.1.7", "2.1.2.0", "2.1.3.0",
    "2.1.3.4", "2.1.4.0", "2.1.5.0", "2.1.6.0",
];

/// Get encoding constants for a given version index.
/// Returns the latest constants for recent versions, with adjustments for older ones.
pub fn consts_for_version(version: usize) -> EncodingConsts {
    // Base: latest version constants
    let mut c = EncodingConsts {
        item_id_bitlen: 13,
        tome_id_bitlen: 8,
        aspect_id_bitlen: 5,
        aspect_tier_bitlen: 2,
        powder_id_bitlen: 5,
        powder_wrapper_bitlen: 2,
        powder_tiers: 6,
        powder_elements: 5,
        max_sp_bitlen: 12,
        level_bitlen: 7,
        max_level: 106,
        equipment_num: 9,
        tome_num: 14,
        num_aspects: 5,
    };

    // Adjust for older versions
    if version <= 8 {
        c.item_id_bitlen = 12;
        c.aspect_id_bitlen = 0; // Aspects didn't exist
        c.num_aspects = 0;
    }

    if version == 0 {
        c.tome_id_bitlen = 6;
    }

    c
}

/// The legacy header value that indicates the V12 binary format.
pub const BINARY_FORMAT_FLAG: u64 = 12; // 0xC
