//! Determinism rules.
//!
//! Scan order is allowed to be nondeterministic; the output is not. Tests,
//! CLI diffs, MCP responses and regression runs all depend on byte-identical
//! output for the same input, at any worker count.

use std::cmp::Ordering;

use crate::model::fingerprint::Fingerprint;

/// Byte-level comparison for canonical keys.
///
/// Never use a locale-aware or Unicode-collation comparison here: the same
/// fixture must produce the same order on macOS, Windows and Linux.
pub fn canonical_cmp(a: &[u8], b: &[u8]) -> Ordering {
    a.cmp(b)
}

/// Sort key of a duplicate group.
///
/// Frozen order: size descending, then fingerprint ascending, then the
/// canonical locator key ascending. Member order never depends on which worker
/// discovered a file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GroupOrderKey {
    pub size: u64,
    pub fingerprint: Option<Fingerprint>,
    pub locator_key: Vec<u8>,
}

impl Ord for GroupOrderKey {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .size
            .cmp(&self.size)
            .then_with(|| match (self.fingerprint, other.fingerprint) {
                (Some(a), Some(b)) => a.cmp_bytes(&b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            })
            .then_with(|| canonical_cmp(&self.locator_key, &other.locator_key))
    }
}

impl PartialOrd for GroupOrderKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Sort a slice into the canonical order.
///
/// `sort_unstable` would be fine for a single pass, but stability matters when
/// two groups share a key and a caller re-sorts an already sorted stream.
pub fn sort_canonical(keys: &mut [GroupOrderKey]) {
    keys.sort();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(size: u64, fp: Option<u8>, locator: &[u8]) -> GroupOrderKey {
        GroupOrderKey {
            size,
            fingerprint: fp.map(|b| Fingerprint { bytes: [b; 32] }),
            locator_key: locator.to_vec(),
        }
    }

    #[test]
    fn larger_groups_come_first() {
        let mut keys = vec![key(10, None, b"a"), key(100, None, b"b")];
        sort_canonical(&mut keys);
        assert_eq!(keys[0].size, 100);
    }

    #[test]
    fn ties_break_on_fingerprint_then_locator() {
        let mut keys = vec![
            key(10, Some(2), b"a"),
            key(10, Some(1), b"z"),
            key(10, Some(1), b"a"),
        ];
        sort_canonical(&mut keys);
        assert_eq!(keys[0].fingerprint.unwrap().bytes[0], 1);
        assert_eq!(keys[0].locator_key, b"a".to_vec());
        assert_eq!(keys[1].locator_key, b"z".to_vec());
    }

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn order_is_independent_of_arrival_order() {
        // The same three groups discovered by 1, 2 or 8 workers, i.e. in any
        // arrival order, must normalize to the same sequence.
        let expected = {
            let mut k = vec![
                key(5, Some(9), b"b"),
                key(5, Some(1), b"a"),
                key(7, None, b"c"),
            ];
            sort_canonical(&mut k);
            k
        };
        for permutation in [
            [0usize, 1, 2],
            [1, 0, 2],
            [2, 1, 0],
            [2, 0, 1],
            [1, 2, 0],
            [0, 2, 1],
        ] {
            let all = vec![
                key(5, Some(9), b"b"),
                key(5, Some(1), b"a"),
                key(7, None, b"c"),
            ];
            let mut shuffled = vec![
                all[permutation[0]].clone(),
                all[permutation[1]].clone(),
                all[permutation[2]].clone(),
            ];
            sort_canonical(&mut shuffled);
            assert_eq!(shuffled, expected, "permutation {permutation:?}");
        }
    }

    #[test]
    fn canonical_cmp_is_byte_level() {
        assert_eq!(canonical_cmp(b"Z", b"a"), Ordering::Less); // 0x5A < 0x61
    }
}
