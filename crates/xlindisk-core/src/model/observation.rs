//! Observation: what the core saw, and whether it can still be trusted.
//!
//! The model is not `Entry -> hash -> result`. It is:
//!
//! ```text
//! Entry -> Observation -> operation -> re-observation -> validation -> result
//! ```
//!
//! Everything that reads content (duplicate hash, cache, future delete,
//! PhotoKit materialization) shares this shape, so "the object may have changed
//! under us" is one concept in the core instead of a special case inside one
//! pipeline.
//!
//! Naming matters here: a stable observation means "the identity, size and
//! revision we can observe did not change", **not** "the content is proven
//! identical". See [`ObservationValidation::ObservationStable`].

use crate::model::fingerprint::{
    Fingerprint, VerificationStatus, FINGERPRINT_ALGORITHM, FINGERPRINT_ALGORITHM_VERSION,
};
use crate::model::ids::{LocatorId, ObjectId, RunId, ScanId, SessionId, SourceId};

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

/// Source-specific version information.
///
/// Whatever the platform offers beyond size and mtime: a generation counter, a
/// change counter, a version token. Filesystem v0.0.1 usually has nothing here,
/// which is exactly why it is optional rather than assumed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RevisionId(pub u64);

/// One look at an object.
///
/// Taken before an operation and again afterwards; the source then validates the
/// two against each other, because only the source knows how strong its own
/// notion of identity is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Observation {
    pub source: SourceId,
    pub locator: LocatorId,
    pub object: Option<ObjectId>,
    /// Logical size. Never physical size: sparse, compressed, cloned and CoW
    /// files make that inference invalid.
    pub logical_size: Option<u64>,
    pub modified: Option<Timestamp>,
    pub revision: Option<RevisionId>,
}

impl Observation {
    /// The conservative default comparison, for sources that have nothing
    /// better: object identity, size, timestamp and revision must all agree.
    ///
    /// Unknown fields (`None`) cannot prove a change and are ignored; a field
    /// known on both sides that differs does prove one. Sources may override it
    /// through `Source::validate_observation`.
    pub fn strict_compare(before: &Self, after: &Self) -> ObservationValidation {
        if let (Some(a), Some(b)) = (before.object, after.object) {
            if a != b {
                return ObservationValidation::ChangedDuringScan;
            }
        }
        if let (Some(a), Some(b)) = (before.logical_size, after.logical_size) {
            if a != b {
                return ObservationValidation::ChangedDuringScan;
            }
        }
        if let (Some(a), Some(b)) = (before.modified, after.modified) {
            if a != b {
                return ObservationValidation::ChangedDuringScan;
            }
        }
        if let (Some(a), Some(b)) = (before.revision, after.revision) {
            if a != b {
                return ObservationValidation::ChangedDuringScan;
            }
        }
        ObservationValidation::ObservationStable
    }
}

/// What a re-observation concluded.
///
/// `ObservationStable` — deliberately not `ContentUnchanged` — means the
/// observable identity did not move. It is an engineering-grade statement, not a
/// proof of byte equality.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ObservationValidation {
    /// Identity, size, timestamp and revision agree; safe to proceed.
    ObservationStable,
    /// Something observable changed between the two looks.
    ChangedDuringScan,
    /// The object is gone: deleted, renamed, or moved out of scope.
    Gone,
    /// It exists but can no longer be read (permission or ACL changed).
    PermissionChanged,
    /// Content could not be read at all.
    Unreadable,
}

impl ObservationValidation {
    /// Only a stable observation may produce a confirmed result.
    pub fn is_stable(self) -> bool {
        matches!(self, ObservationValidation::ObservationStable)
    }

    pub fn to_verification_status(self) -> VerificationStatus {
        match self {
            ObservationValidation::ObservationStable => VerificationStatus::HashMatched,
            ObservationValidation::ChangedDuringScan | ObservationValidation::Gone => {
                VerificationStatus::ChangedDuringScan
            }
            ObservationValidation::PermissionChanged | ObservationValidation::Unreadable => {
                VerificationStatus::Unreadable
            }
        }
    }

    /// Failed candidates are reported as excluded, never mixed into a confirmed
    /// duplicate group.
    pub fn exclusion_reason(self) -> Option<crate::duplicate::ExclusionReason> {
        match self {
            ObservationValidation::ObservationStable => None,
            ObservationValidation::ChangedDuringScan | ObservationValidation::Gone => {
                Some(crate::duplicate::ExclusionReason::ChangedDuringScan)
            }
            ObservationValidation::PermissionChanged => {
                Some(crate::duplicate::ExclusionReason::PermissionChanged)
            }
            ObservationValidation::Unreadable => {
                Some(crate::duplicate::ExclusionReason::Unreadable)
            }
        }
    }
}

/// One output row.
///
/// Analysis provenance, not an MCP payload: a future MCP layer adapts this, it
/// does not get to redefine it. Safe deletion will re-validate through the
/// locator and this observation before touching anything.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Record {
    pub run_id: RunId,
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
        run_id: RunId,
        scan_id: ScanId,
        session_id: SessionId,
        source: SourceId,
        locator: LocatorId,
    ) -> Self {
        Self {
            run_id,
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

    /// Fill the observation half of the record.
    pub fn with_observation(mut self, observation: &Observation) -> Self {
        self.source = observation.source;
        self.locator = observation.locator;
        self.object = observation.object;
        self.logical_size = observation.logical_size;
        self.modified = observation.modified;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::VolumeId;

    fn observation() -> Observation {
        Observation {
            source: SourceId(1),
            locator: LocatorId(1),
            object: Some(ObjectId::new(SourceId(1), VolumeId(7), 99)),
            logical_size: Some(42),
            modified: Some(Timestamp::new(1_700_000_000, 0)),
            revision: None,
        }
    }

    #[test]
    fn identical_observations_are_stable() {
        let a = observation();
        assert!(Observation::strict_compare(&a, &observation()).is_stable());
    }

    #[test]
    fn size_change_is_detected() {
        let a = observation();
        let mut b = observation();
        b.logical_size = Some(43);
        assert_eq!(
            Observation::strict_compare(&a, &b),
            ObservationValidation::ChangedDuringScan
        );
    }

    #[test]
    fn mtime_change_is_detected() {
        let a = observation();
        let mut b = observation();
        b.modified = Some(Timestamp::new(1_700_000_001, 0));
        assert_eq!(
            Observation::strict_compare(&a, &b),
            ObservationValidation::ChangedDuringScan
        );
    }

    #[test]
    fn revision_change_is_detected() {
        let a = observation();
        let mut b = observation();
        b.revision = Some(RevisionId(2));
        let mut c = observation();
        c.revision = Some(RevisionId(3));
        assert!(
            Observation::strict_compare(&a, &b).is_stable(),
            "unknown revision on one side cannot prove a change"
        );
        assert_eq!(
            Observation::strict_compare(&b, &c),
            ObservationValidation::ChangedDuringScan
        );
    }

    #[test]
    fn different_object_is_detected() {
        let a = observation();
        let mut b = observation();
        b.object = Some(ObjectId::new(SourceId(1), VolumeId(7), 100));
        assert_eq!(
            Observation::strict_compare(&a, &b),
            ObservationValidation::ChangedDuringScan
        );
    }

    #[test]
    fn unknown_fields_do_not_fake_a_change() {
        let a = Observation::default();
        assert!(Observation::strict_compare(&a, &observation()).is_stable());
    }

    #[test]
    fn only_stable_observations_confirm_a_result() {
        assert!(ObservationValidation::ObservationStable
            .to_verification_status()
            .is_confirmed());
        for unstable in [
            ObservationValidation::ChangedDuringScan,
            ObservationValidation::Gone,
            ObservationValidation::PermissionChanged,
            ObservationValidation::Unreadable,
        ] {
            assert!(!unstable.to_verification_status().is_confirmed());
            assert!(unstable.exclusion_reason().is_some());
        }
        assert!(ObservationValidation::ObservationStable
            .exclusion_reason()
            .is_none());
    }
}
