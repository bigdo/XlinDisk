//! Observation: what the core saw, and whether it can still be trusted.
//!
//! Files move and change while a scan runs. A duplicate conclusion is only
//! valid if the observation was stable across both passes, so the snapshot
//! taken before hashing is part of the result, not an implementation detail.

use crate::model::fingerprint::{
    Fingerprint, VerificationStatus, FINGERPRINT_ALGORITHM, FINGERPRINT_ALGORITHM_VERSION,
};
use crate::model::ids::{LocatorId, ObjectId, ScanId, SessionId, SourceId};

/// Wall-clock timestamp with nanoseconds. Source-provided; the core never
/// invents one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Timestamp {
    pub secs: i64,
    pub nanos: u32,
}

impl Timestamp {
    pub const fn new(secs: i64, nanos: u32) -> Self {
        Self { secs, nanos }
    }
}

/// The metadata of an object at one instant.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Snapshot {
    pub logical_size: Option<u64>,
    pub modified: Option<Timestamp>,
    pub object: Option<ObjectId>,
    /// Platform change counter (Windows change time, generation number) when
    /// available; stronger than mtime alone.
    pub generation: Option<u64>,
}

impl Snapshot {
    /// Compare two snapshots of the same locator.
    ///
    /// Unknown fields (`None`) cannot prove a change and are ignored; a field
    /// that is known on both sides and differs does prove one.
    pub fn is_consistent_with(&self, later: &Self) -> bool {
        if let (Some(a), Some(b)) = (self.object, later.object) {
            if a != b {
                return false;
            }
        }
        if let (Some(a), Some(b)) = (self.logical_size, later.logical_size) {
            if a != b {
                return false;
            }
        }
        if let (Some(a), Some(b)) = (self.modified, later.modified) {
            if a != b {
                return false;
            }
        }
        if let (Some(a), Some(b)) = (self.generation, later.generation) {
            if a != b {
                return false;
            }
        }
        true
    }

    /// Classify a re-stat performed after hashing.
    ///
    /// `None` means the object is gone: it was deleted, renamed, or moved out
    /// of scope during the scan.
    pub fn classify(&self, after: Option<&Self>) -> ObservationClass {
        match after {
            None => ObservationClass::Gone,
            Some(later) if !self.is_consistent_with(later) => ObservationClass::ChangedDuringScan,
            Some(_) => ObservationClass::Stable,
        }
    }
}

/// Outcome of the observation-consistency check.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ObservationClass {
    /// Both passes agree; the duplicate conclusion may stand.
    Stable,
    /// The object changed between passes: never report as a duplicate.
    ChangedDuringScan,
    /// The object disappeared; the entry is dropped from results.
    Gone,
    /// Content could not be read (permission, IO, ...).
    Unreadable,
}

impl ObservationClass {
    /// Map to the verification status recorded on the result.
    pub fn to_verification_status(self) -> VerificationStatus {
        match self {
            ObservationClass::Stable => VerificationStatus::HashMatched,
            ObservationClass::ChangedDuringScan => VerificationStatus::ChangedDuringScan,
            ObservationClass::Gone => VerificationStatus::ChangedDuringScan,
            ObservationClass::Unreadable => VerificationStatus::Unreadable,
        }
    }
}

/// One output row.
///
/// The field set is frozen: it already carries everything a future MCP / safe
/// deletion layer needs (session, observation time, scope, locator, object
/// identity, size/mtime, algorithm version, verification status). Deletion must
/// re-verify through the locator — never from a path or a digest alone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Record {
    pub scan_id: ScanId,
    pub session_id: SessionId,
    /// When the observation was made.
    pub observed_at: Option<Timestamp>,
    pub source: SourceId,
    pub locator: LocatorId,
    pub object: Option<ObjectId>,
    pub logical_size: Option<u64>,
    pub modified: Option<Timestamp>,
    pub fingerprint: Option<Fingerprint>,
    pub fingerprint_algorithm: &'static str,
    pub fingerprint_algorithm_version: u32,
    pub verification: VerificationStatus,
}

impl Record {
    pub fn new(
        scan_id: ScanId,
        session_id: SessionId,
        source: SourceId,
        locator: LocatorId,
    ) -> Self {
        Self {
            scan_id,
            session_id,
            observed_at: None,
            source,
            locator,
            object: None,
            logical_size: None,
            modified: None,
            fingerprint: None,
            fingerprint_algorithm: FINGERPRINT_ALGORITHM,
            fingerprint_algorithm_version: FINGERPRINT_ALGORITHM_VERSION,
            verification: VerificationStatus::Unverified,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::{SourceId, VolumeId};

    fn snap() -> Snapshot {
        Snapshot {
            logical_size: Some(42),
            modified: Some(Timestamp::new(1_700_000_000, 0)),
            object: Some(ObjectId::new(SourceId(1), VolumeId(7), 99)),
            generation: None,
        }
    }

    #[test]
    fn identical_snapshots_are_stable() {
        let a = snap();
        assert_eq!(a.classify(Some(&snap())), ObservationClass::Stable);
    }

    #[test]
    fn size_change_is_detected() {
        let a = snap();
        let mut b = snap();
        b.logical_size = Some(43);
        assert_eq!(a.classify(Some(&b)), ObservationClass::ChangedDuringScan);
    }

    #[test]
    fn mtime_change_is_detected() {
        let a = snap();
        let mut b = snap();
        b.modified = Some(Timestamp::new(1_700_000_001, 0));
        assert_eq!(a.classify(Some(&b)), ObservationClass::ChangedDuringScan);
    }

    #[test]
    fn different_inode_is_detected() {
        let a = snap();
        let mut b = snap();
        b.object = Some(ObjectId::new(SourceId(1), VolumeId(7), 100));
        assert_eq!(a.classify(Some(&b)), ObservationClass::ChangedDuringScan);
    }

    #[test]
    fn missing_object_is_gone() {
        let a = snap();
        assert_eq!(a.classify(None), ObservationClass::Gone);
    }

    #[test]
    fn unknown_fields_do_not_fake_a_change() {
        let a = Snapshot::default();
        let b = snap();
        assert!(a.is_consistent_with(&b));
        assert_eq!(a.classify(Some(&b)), ObservationClass::Stable);
    }

    #[test]
    fn gone_and_changed_are_never_confirmed() {
        assert!(!ObservationClass::Gone
            .to_verification_status()
            .is_confirmed());
        assert!(!ObservationClass::ChangedDuringScan
            .to_verification_status()
            .is_confirmed());
        assert!(ObservationClass::Stable
            .to_verification_status()
            .is_confirmed());
    }
}
