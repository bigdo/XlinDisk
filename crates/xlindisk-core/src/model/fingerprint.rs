//! Content identity.
//!
//! Object identity != content identity. Two hardlinks share an
//! [`crate::model::ids::ObjectId`] and a fingerprint; two copies share only the
//! fingerprint.

/// Algorithm tag recorded with every result. A result is never interpretable
/// without it, because a future version may change the hash.
pub const FINGERPRINT_ALGORITHM: &str = "blake3";

/// Bumped whenever the bytes fed to the hash change (range policy, salt, ...).
pub const FINGERPRINT_ALGORITHM_VERSION: u32 = 1;

/// A full-content digest.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    pub bytes: [u8; 32],
}

impl Fingerprint {
    pub const ZERO: Fingerprint = Fingerprint { bytes: [0u8; 32] };

    /// Byte-level ordering. This is the only ordering used for deterministic
    /// output; never sort fingerprints with a locale-aware comparison.
    pub fn cmp_bytes(&self, other: &Self) -> std::cmp::Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl Ord for Fingerprint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp_bytes(other)
    }
}

impl PartialOrd for Fingerprint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fingerprint({}…)", hex_prefix(&self.bytes, 8))
    }
}

/// What a `Fingerprint` node actually computes.
///
/// `Partial` parameters are tunable strategy, **not** API contract: they may
/// change after any benchmark without a contract revision.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FingerprintSpec {
    /// Cheap pre-filter, e.g. first + last 4 KiB.
    Partial {
        prefix_bytes: u32,
        suffix_bytes: u32,
    },
    /// Whole content digest.
    Full,
}

impl Default for FingerprintSpec {
    fn default() -> Self {
        FingerprintSpec::Partial {
            prefix_bytes: 4096,
            suffix_bytes: 4096,
        }
    }
}

/// How strict the safety policy is. v0.0.1 is read-only, so only `Hash` runs;
/// `ByteExact` exists so that a future destructive action has somewhere to go.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum VerificationMode {
    #[default]
    Hash,
    ByteExact,
}

/// How a match was established.
///
/// A BLAKE3 match is an engineering-grade fingerprint, **not** a proof of byte
/// equality. Callers must read this field before claiming "identical".
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum VerificationStatus {
    /// Same size, same full digest, metadata stable across both passes.
    HashMatched,
    /// Additionally compared byte-for-byte.
    ByteExactVerified,
    /// Metadata changed between passes: never a confirmed duplicate.
    ChangedDuringScan,
    /// Content could not be read.
    Unreadable,
    /// Not (yet) verified, e.g. cancelled mid-pass.
    Unverified,
}

impl VerificationStatus {
    /// True only when the result may be presented as a confirmed duplicate.
    pub fn is_confirmed(&self) -> bool {
        matches!(
            self,
            VerificationStatus::HashMatched | VerificationStatus::ByteExactVerified
        )
    }
}

fn hex_prefix(bytes: &[u8], n: usize) -> String {
    bytes[..n.min(bytes.len())]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_byte_level() {
        let mut a = Fingerprint::ZERO;
        a.bytes[31] = 1;
        let mut b = Fingerprint::ZERO;
        b.bytes[0] = 1;
        assert!(b > a, "high-order bytes must dominate");
    }

    #[test]
    fn only_confirmed_statuses_count() {
        assert!(VerificationStatus::HashMatched.is_confirmed());
        assert!(VerificationStatus::ByteExactVerified.is_confirmed());
        assert!(!VerificationStatus::ChangedDuringScan.is_confirmed());
        assert!(!VerificationStatus::Unreadable.is_confirmed());
        assert!(!VerificationStatus::Unverified.is_confirmed());
    }
}
