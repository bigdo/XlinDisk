//! SOURCE contract.
//!
//! A source answers "what objects exist" and "how do I read one". It is the
//! only place that knows about paths, content URIs or PhotoKit resources; the
//! core only ever holds an opaque [`crate::model::ids::LocatorId`].

use crate::error::Error;
use crate::model::entry::Entry;
use crate::model::ids::{LocatorId, SessionId, SourceId};
use crate::model::observation::{Observation, ObservationValidation};
use crate::runtime::CancellationToken;

bitflags::bitflags! {
    /// What a source supports. Capabilities are not authorization: a source may
    /// be able to delete and still be unauthorized right now.
    ///
    /// Concurrency is a capability, not an assumption. The runtime must not
    /// presume every source is thread-safe: mobile providers routinely are not.
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
        /// `stat` may be called from several threads at once.
        const CONCURRENT_STAT = 1 << 6;
        /// `open_content` reads may run in parallel.
        const CONCURRENT_READ = 1 << 7;
    }
}

/// Default capability set of a local filesystem source.
pub const FILESYSTEM_CAPABILITIES: SourceCapabilities = SourceCapabilities::HIERARCHICAL
    .union(SourceCapabilities::STABLE_OBJECT_ID)
    .union(SourceCapabilities::SEQUENTIAL_READ)
    .union(SourceCapabilities::SEEK)
    .union(SourceCapabilities::RANGE_READ)
    .union(SourceCapabilities::LOCAL_ONLY)
    .union(SourceCapabilities::CONCURRENT_STAT)
    .union(SourceCapabilities::CONCURRENT_READ);

/// How symlinks are treated.
///
/// A name in a document is not a contract: this is carried by every scan
/// request, so two runs with different policies are visibly different runs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum LinkPolicy {
    /// Never traverse a symlink; report it as an entry with the `LINK` flag.
    #[default]
    DoNotFollow,
    /// Follow symlinks whose target stays inside the requested roots. Cycles are
    /// detected by directory identity and reported, never traversed twice.
    FollowWithinRoots,
}

/// Windows reparse points (junctions, mount points, symlinkd).
///
/// Separate from [`LinkPolicy`] because the safe default is different: not
/// traversing a reparse point is the only way to avoid directory cycles on
/// Windows without doing identity work first.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum ReparsePolicy {
    #[default]
    DoNotTraverseReparsePoint,
    TraverseWithinRoots,
}

/// Whether traversal may leave the filesystem it started on.
///
/// There is no implicit default inside the core: every preset states which one
/// it wants.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum MountPolicy {
    #[default]
    StayOnInitialFilesystem,
    CrossFilesystems,
}

/// What to enumerate.
#[derive(Clone, Debug)]
pub struct ScanRequest {
    pub source: SourceId,
    pub session: SessionId,
    /// Roots are locators, not paths: the core does not know what a path is.
    pub roots: Vec<LocatorId>,
    pub link_policy: LinkPolicy,
    pub reparse_policy: ReparsePolicy,
    pub mount_policy: MountPolicy,
}

impl ScanRequest {
    pub fn new(source: SourceId, session: SessionId, roots: Vec<LocatorId>) -> Self {
        Self {
            source,
            session,
            roots,
            link_policy: LinkPolicy::DoNotFollow,
            reparse_policy: ReparsePolicy::DoNotTraverseReparsePoint,
            mount_policy: MountPolicy::StayOnInitialFilesystem,
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

/// Receives batches. Implementations must tolerate batches arriving in any order
/// and from any worker: deterministic ordering is applied at output time, not
/// during the scan.
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
/// Locator contract (frozen in PR0 r1, see
/// `docs/contracts/03-locator-and-source-contract.md`):
///
/// * a `LocatorId` is an opaque, immutable, `Copy + Send + Sync` integer owned
///   by the source, valid **only** inside the session that produced it —
///   `LocatorId(35)` in session A and in session B are unrelated; using it after
///   the session ends is `LocatorExpired`, using it with the wrong source is
///   `LocatorInvalid`;
/// * `display_locator` is the only way to obtain a human-readable path, and it
///   is **lazy** — called for results that will actually be shown, never per
///   discovered file, otherwise the path allocations we avoided come straight
///   back;
/// * a displayed path is never an identity;
/// * `sort_key` is the source's canonical ordering key, which is what makes
///   output reproducible without case folding or Unicode normalization.
pub trait Source: Send + Sync {
    fn id(&self) -> SourceId;

    fn capabilities(&self) -> SourceCapabilities;

    fn scan(
        &self,
        request: &ScanRequest,
        sink: &mut dyn EntrySink,
        cancel: &CancellationToken,
    ) -> Result<ScanOutcome, Error>;

    /// Observe an object: the re-validation primitive used before hashing, and
    /// later before any destructive action.
    fn stat(&self, locator: LocatorId) -> Result<Observation, Error>;

    /// Decide whether two observations describe the same unchanged object.
    ///
    /// This belongs to the source, not to the runtime: the filesystem compares
    /// object id + size + mtime, while PhotoKit will compare asset id + resource
    /// version. The core only defines [`Observation`] and
    /// [`ObservationValidation`].
    fn validate_observation(
        &self,
        locator: LocatorId,
        before: &Observation,
        after: &Observation,
    ) -> Result<ObservationValidation, Error>;

    /// Open content for reading.
    fn open_content(
        &self,
        locator: LocatorId,
        request: ContentRequest,
    ) -> Result<Box<dyn ContentReader + Send + '_>, Error>;

    /// Human-readable rendering of a locator, for CLI / UI / logs only. Never
    /// feed the result back into the core as an identity.
    fn display_locator(&self, locator: LocatorId) -> Result<String, Error>;

    /// Canonical ordering key for deterministic output.
    ///
    /// Filesystem rules from the spec: raw filename/path bytes on Unix, lossless
    /// UTF-16 code units on Windows, lexicographic in both cases — no case
    /// folding, no NFC/NFD normalization. Sorting is for reproducibility, not
    /// for deciding whether two paths are the same file.
    fn sort_key(&self, locator: LocatorId) -> Result<Vec<u8>, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_declares_concurrency_explicitly() {
        let caps = FILESYSTEM_CAPABILITIES;
        assert!(caps.contains(SourceCapabilities::CONCURRENT_STAT));
        assert!(caps.contains(SourceCapabilities::CONCURRENT_READ));
        assert!(caps.contains(SourceCapabilities::STABLE_OBJECT_ID));
    }

    #[test]
    fn safe_defaults_are_do_not_follow_and_stay_put() {
        let request = ScanRequest::new(SourceId(0), SessionId(0), vec![]);
        assert_eq!(request.link_policy, LinkPolicy::DoNotFollow);
        assert_eq!(
            request.reparse_policy,
            ReparsePolicy::DoNotTraverseReparsePoint
        );
        assert_eq!(request.mount_policy, MountPolicy::StayOnInitialFilesystem);
    }
}
