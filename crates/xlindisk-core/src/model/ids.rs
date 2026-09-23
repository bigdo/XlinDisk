//! Dense, session-scoped identifiers.
//!
//! All ids are small copyable integers so that a scan can hold millions of
//! entries without per-entry allocations. None of them is a permanent, global
//! object identity: they are indexes into arenas owned by a scan session.

/// Identifies *which* storage source an object came from.
///
/// The core never assumes there is only one source in a process.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SourceId(pub u32);

/// A group of scans that share locators, e.g. one library open / one mount.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SessionId(pub u64);

/// One scan invocation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ScanId(pub u64);

/// One plan execution. A single scan may feed several runs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RunId(pub u64);

/// Dense index of an entry inside a scan session. Hot-path friendly.
///
/// Not stable across runs; use [`ObjectId`] when you need "same object".
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EntryId(pub u32);

impl EntryId {
    pub const INVALID: EntryId = EntryId(u32::MAX);
}

/// Opaque handle owned by a [`crate::source::Source`].
///
/// The core stores it and can hand it back, but it never interprets it: on the
/// filesystem it may be a (parent, name) pair, on iOS a `PHAssetResource`
/// identifier, on Android a content URI. See `docs/contracts/03-locator-and-source-contract.md`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct LocatorId(pub u64);

impl LocatorId {
    pub const INVALID: LocatorId = LocatorId(u64::MAX);
}

/// Device / volume / library partition inside a source.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct VolumeId(pub u64);

impl VolumeId {
    /// Used when a source does not model volumes (e.g. a photo library).
    pub const UNKNOWN: VolumeId = VolumeId(0);
}

/// Identity of the underlying storage object (not of its content).
///
/// Frozen semantics (PR0):
///
/// * valid **only within a single scan session** — never persisted as a global
///   identity, never compared across sessions;
/// * scoped by `(SourceId, VolumeId)`, so the same inode number on two volumes
///   is two different objects;
/// * `None` when the platform cannot provide identity (see
///   [`ObjectIdUnavailable`]); when it is `None`, reclaimable space must not be
///   computed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ObjectId {
    pub source: SourceId,
    pub volume: VolumeId,
    /// inode, file index, or platform equivalent.
    pub value: u128,
}

impl ObjectId {
    pub fn new(source: SourceId, volume: VolumeId, value: u128) -> Self {
        Self {
            source,
            volume,
            value,
        }
    }
}

/// Why an object identity could not be obtained.
///
/// Recorded on the entry instead of silently falling back to "not a hardlink".
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ObjectIdUnavailable {
    /// No stable identity API on this platform/target.
    UnsupportedPlatform,
    /// Network / SMB / NFS mount: identity is not trustworthy across mounts.
    NetworkFileSystem,
    /// Filesystem has no identity at all (FAT / exFAT and friends).
    FileSystemWithoutIdentity,
    /// Special file (device, socket, fifo, ...).
    SpecialFile,
    /// The metadata call failed; treat as unknown rather than as unique.
    MetadataFailed,
}
