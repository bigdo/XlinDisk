//! SOURCE contract.
//!
//! A source answers "what objects exist" and "how do I read one". It is the
//! only place that knows about paths, content URIs or PhotoKit resources; the
//! core only ever holds an opaque [`crate::model::ids::LocatorId`].

use crate::error::Error;
use crate::model::entry::Entry;
use crate::model::ids::{LocatorId, SessionId, SourceId};
use crate::model::observation::Snapshot;
use crate::runtime::CancellationToken;

bitflags::bitflags! {
    /// What a source supports. Capabilities are not authorization: a source may
    /// be able to delete and still be unauthorized right now.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct SourceCapabilities: u32 {
        const HIERARCHICAL = 1 << 0;
        /// Can produce stable [`crate::model::ids::ObjectId`]s.
        const STABLE_OBJECT_ID = 1 << 1;
        const SEQUENTIAL_READ = 1 << 2;
        const SEEK = 1 << 3;
        const RANGE_READ = 1 << 4;
        /// Content lives on this machine; no network round-trip per read.
        const LOCAL_ONLY = 1 << 5;
        /// PR0 addition: content reads must be serialized by the runtime.
        /// Unset means concurrent reads are allowed (the common case).
        const SERIALIZE_READS = 1 << 6;
    }
}

/// Default capability set of a local filesystem source.
pub const FILESYSTEM_CAPABILITIES: SourceCapabilities = SourceCapabilities::HIERARCHICAL
    .union(SourceCapabilities::STABLE_OBJECT_ID)
    .union(SourceCapabilities::SEQUENTIAL_READ)
    .union(SourceCapabilities::SEEK)
    .union(SourceCapabilities::RANGE_READ)
    .union(SourceCapabilities::LOCAL_ONLY);

/// How symlinks / junctions / reparse points are treated.
///
/// Before PR0 these were just names; from now on they are part of the request,
/// so two runs with different policies are visibly different runs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum LinkPolicy {
    /// Never traverse a link; report it as an entry with the `LINK` flag.
    #[default]
    DoNotFollow,
    /// Follow links whose target stays inside the requested roots. Cycles are
    /// reported, never traversed twice.
    FollowWithinRoots,
}

/// Whether traversal may leave the filesystem it started on.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum BoundaryPolicy {
    /// Stop at mount points / volume boundaries; report them with the
    /// `MOUNT_BOUNDARY` flag.
    #[default]
    StayWithinRoots,
    /// Cross boundaries (explicit opt-in only).
    Cross,
}

/// What to enumerate.
#[derive(Clone, Debug)]
pub struct ScanRequest {
    pub source: SourceId,
    pub session: SessionId,
    /// Roots are locators, not paths: the core does not know what a path is.
    pub roots: Vec<LocatorId>,
    pub link_policy: LinkPolicy,
    pub boundary_policy: BoundaryPolicy,
}

impl ScanRequest {
    pub fn new(source: SourceId, session: SessionId, roots: Vec<LocatorId>) -> Self {
        Self {
            source,
            session,
            roots,
            link_policy: LinkPolicy::DoNotFollow,
            boundary_policy: BoundaryPolicy::StayWithinRoots,
        }
    }
}

/// A chunk of entries.
///
/// Batching keeps allocation, channel and synchronization cost off the hot path:
/// one cross-thread message per file would dominate everything else.
#[derive(Clone, Copy, Debug)]
pub struct EntryBatch<'a> {
    pub source: SourceId,
    pub entries: &'a [Entry],
}

/// Receives batches. Implementations must tolerate batches arriving in any
/// order and from any worker: deterministic ordering is applied at output time,
/// not during the scan.
pub trait EntrySink {
    fn push(&mut self, batch: EntryBatch<'_>) -> Result<(), Error>;
}

/// What to read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContentRequest {
    Whole,
    Range { offset: u64, len: u64 },
}

/// Stream of object content.
///
/// Not `std::fs::File`: the same pipeline must later read a PhotoKit resource
/// or a SAF document without changing a single line of the fingerprint code.
pub trait ContentReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    /// Optional hint; `None` when the length is unknown.
    fn remaining(&self) -> Option<u64> {
        None
    }
}

/// Counters returned by a scan.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ScanOutcome {
    pub entries: u64,
    pub errors: u64,
}

/// The boundary between the core and a storage world.
///
/// Locator contract (frozen in PR0, see `docs/contracts/03-locator-and-source-contract.md`):
///
/// * a `LocatorId` is issued by a source and belongs to that source **and** to
///   the session that produced it — reusing it elsewhere is `LocatorInvalid`;
/// * locators are `Copy`, so they may travel between threads and pipeline
///   stages; re-opening later is allowed unless the session ended
///   (`LocatorExpired`);
/// * `stat` is the re-validation primitive: the fingerprint pass re-stats and
///   compares against the snapshot taken during the scan;
/// * `display_locator` is the *only* way to obtain a human-readable path, and
///   it lives here precisely so the core never stores one.
pub trait Source: Send + Sync {
    fn id(&self) -> SourceId;

    fn capabilities(&self) -> SourceCapabilities;

    fn scan(
        &self,
        request: &ScanRequest,
        sink: &mut dyn EntrySink,
        cancel: &CancellationToken,
    ) -> Result<ScanOutcome, Error>;

    /// Open content for reading.
    fn open_content(
        &self,
        locator: LocatorId,
        request: ContentRequest,
    ) -> Result<Box<dyn ContentReader + Send + '_>, Error>;

    /// Re-stat an object: re-validation before fingerprinting, and later before
    /// any destructive action.
    fn stat(&self, locator: LocatorId) -> Result<Snapshot, Error>;

    /// Human-readable rendering of a locator, for CLI / UI / logs only. Never
    /// feed the result back into the core as an identity.
    fn display_locator(&self, locator: LocatorId) -> Result<String, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_reports_identity_and_seeking() {
        let caps = FILESYSTEM_CAPABILITIES;
        assert!(caps.contains(SourceCapabilities::STABLE_OBJECT_ID));
        assert!(caps.contains(SourceCapabilities::SEEK));
        assert!(caps.contains(SourceCapabilities::RANGE_READ));
        assert!(!caps.contains(SourceCapabilities::SERIALIZE_READS));
    }

    #[test]
    fn safe_defaults_are_do_not_follow_and_stay_within_roots() {
        let request = ScanRequest::new(SourceId(0), SessionId(0), vec![]);
        assert_eq!(request.link_policy, LinkPolicy::DoNotFollow);
        assert_eq!(request.boundary_policy, BoundaryPolicy::StayWithinRoots);
    }
}
