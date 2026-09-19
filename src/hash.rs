//! Full-key hashing. IDs are never reduced with `(a * u + c) % b`.

#[must_use]
pub fn mix(seed: u64, bytes: &[u8]) -> u64 {
    let mut hash = seed ^ 0x9E37_79B9_7F4A_7C15;
    for chunk in bytes.chunks(8) {
        let mut lane = 0_u64;
        for (index, byte) in chunk.iter().enumerate() {
            lane |= u64::from(*byte) << (index * 8);
        }
        hash = hash.wrapping_mul(0xBF58_476D_1CE4_E5B9) ^ lane.rotate_left(17);
        hash = hash.rotate_left(31);
    }
    hash ^= u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    hash ^ (hash >> 33)
}

/// Length-prefixed fields so a NUL inside a label cannot split the tuple.
#[must_use]
pub fn encode_fields(parts: &[&str]) -> alloc::vec::Vec<u8> {
    let mut out = alloc::vec::Vec::new();
    for part in parts {
        let bytes = part.as_bytes();
        let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(bytes);
    }
    out
}

#[must_use]
pub fn bucket(hash: u64, width: usize) -> usize {
    let width = u128::from(u64::try_from(width.max(1)).unwrap_or(1));
    usize::try_from((u128::from(hash) * width) >> 64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{bucket, mix};

    #[test]
    fn ids_offset_by_width_do_not_collide_in_every_replica() {
        let width = 64_usize;
        let left = 7_u64.to_le_bytes();
        let right = (7_u64 + width as u64).to_le_bytes();
        let collisions = (0..8)
            .filter(|replica| {
                let seed = 0xA5A5_A5A5_A5A5_A5A5 ^ (*replica * 0x1111_1111_1111_1111);
                bucket(mix(seed, &left), width) == bucket(mix(seed, &right), width)
            })
            .count();
        assert!(
            collisions < 8,
            "full-key hashes must not share the AnoGraph (u+b) collision"
        );
    }
}
