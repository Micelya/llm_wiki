//! The signature of a document and the buckets that find its neighbours.
//!
//! The signature is a MinHash of its shingles: `PERMUTATIONS` numbers,
//! each the smallest hash of the shingles under one hash function. Two
//! documents agree on each number with a probability equal to the share
//! of shingles they have in common, so comparing signatures estimates
//! that share without the texts.
//!
//! Candidates come from locality-sensitive hashing: the signature is cut
//! in `BANDS` bands of `ROWS` numbers, and two documents are candidates
//! when they agree on a whole band. With 64 bands of 2 rows, a pair that
//! shares 20 % of its shingles is a candidate 92 % of the times, and one
//! that shares 50 % almost always. The threshold is that low on purpose:
//! a short document inside a long one (an annex inside a contract) shares
//! little of the union, and the exact measures discard the rest.

use super::text::fnv1a;

pub const PERMUTATIONS: usize = 128;
pub const BANDS: usize = 64;
pub const ROWS: usize = PERMUTATIONS / BANDS;

/// SplitMix64's finalizer: a fast, well-mixed hash of a 64-bit number.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn seeds() -> [u64; PERMUTATIONS] {
    let mut seeds = [0u64; PERMUTATIONS];
    for (index, seed) in seeds.iter_mut().enumerate() {
        *seed = mix(index as u64 ^ 0x5eed_0f_d0c5);
    }
    seeds
}

/// The MinHash of a set of shingles; empty for an empty set.
pub fn signature(shingles: &[u64]) -> Vec<u64> {
    if shingles.is_empty() {
        return Vec::new();
    }
    let seeds = seeds();
    let mut minimum = vec![u64::MAX; PERMUTATIONS];
    for shingle in shingles {
        for (slot, seed) in minimum.iter_mut().zip(seeds.iter()) {
            let value = mix(shingle ^ seed);
            if value < *slot {
                *slot = value;
            }
        }
    }
    minimum
}

/// The bucket of each band, as SQLite integers.
pub fn buckets(signature: &[u64]) -> Vec<i64> {
    signature
        .chunks(ROWS)
        .enumerate()
        .map(|(band, rows)| {
            let mut bytes = Vec::with_capacity(8 * (ROWS + 1));
            bytes.extend_from_slice(&(band as u64).to_le_bytes());
            for row in rows {
                bytes.extend_from_slice(&row.to_le_bytes());
            }
            fnv1a(&bytes) as i64
        })
        .collect()
}

/// Share of the signature two documents agree on: an estimate of the
/// share of shingles they have in common.
pub fn estimate(a: &[u64], b: &[u64]) -> f64 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b).filter(|(x, y)| x == y).count() as f64 / a.len() as f64
}

pub fn to_blob(values: &[u64]) -> Vec<u8> {
    values.iter().flat_map(|value| value.to_le_bytes()).collect()
}

pub fn from_blob(blob: &[u8]) -> Vec<u64> {
    blob.chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("chunks of eight bytes")))
        .collect()
}

/// Of two sorted sets, how many elements they share and how many each
/// has, leaving out the elements in `skip` (the archive's routine text).
pub fn overlap(a: &[u64], b: &[u64], skip: &dyn Fn(u64) -> bool) -> (usize, usize, usize) {
    let (mut i, mut j) = (0, 0);
    let (mut both, mut only_a, mut only_b) = (0, 0, 0);
    while i < a.len() || j < b.len() {
        let take_a = j >= b.len() || (i < a.len() && a[i] < b[j]);
        let take_b = i >= a.len() || (j < b.len() && b[j] < a[i]);
        if take_a {
            if !skip(a[i]) {
                only_a += 1;
            }
            i += 1;
        } else if take_b {
            if !skip(b[j]) {
                only_b += 1;
            }
            j += 1;
        } else {
            if !skip(a[i]) {
                both += 1;
            }
            i += 1;
            j += 1;
        }
    }
    (both, both + only_a, both + only_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(range: std::ops::Range<u64>) -> Vec<u64> {
        range.map(|x| fnv1a(&x.to_le_bytes())).collect::<std::collections::BTreeSet<_>>().into_iter().collect()
    }

    #[test]
    fn the_estimate_follows_the_shared_share() {
        let a = set(0..1000);
        let b = set(200..1200);
        // 800 shared out of 1200: two thirds.
        let estimate = estimate(&signature(&a), &signature(&b));
        assert!((estimate - 0.667).abs() < 0.12, "{estimate}");
        assert_eq!(super::estimate(&signature(&a), &signature(&a)), 1.0);
    }

    #[test]
    fn similar_documents_share_a_bucket_and_unrelated_ones_do_not() {
        let a = buckets(&signature(&set(0..500)));
        let b = buckets(&signature(&set(100..600)));
        let c = buckets(&signature(&set(5000..5500)));
        assert_eq!(a.len(), BANDS);
        assert!(a.iter().zip(&b).any(|(x, y)| x == y));
        assert!(!a.iter().zip(&c).any(|(x, y)| x == y));
    }

    #[test]
    fn overlap_counts_without_the_routine() {
        let a = vec![1, 2, 3, 4];
        let b = vec![2, 3, 5];
        assert_eq!(overlap(&a, &b, &|_| false), (2, 4, 3));
        assert_eq!(overlap(&a, &b, &|x| x == 2), (1, 3, 2));
        assert_eq!(from_blob(&to_blob(&a)), a);
    }
}
