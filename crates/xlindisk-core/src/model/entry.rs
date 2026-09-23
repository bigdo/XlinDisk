//! Entry: the smallest thing the core is willing to say about a storage object.

use crate::model::ids::{EntryId, LocatorId, ObjectId, SourceId};
use crate::model::observation::{Snapshot, Timestamp};

/// What the object is, in the most general sense.
///
/// `Asset` means "provider-specific resource", e.g. a PhotoKit resource or a
/// MediaStore item. It deliberately does **not** mean image / video / audio:
/// v0.0.1 performs no media classification.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    /// Provider-specific resource (iOS / Android), *not* a media category.
    Asset,
    /// Device, socket, fifo, and other Unix special files.
    Special,
    Other,
}

bitflags::bitflags! {
    /// Per-entry observations that are cheap to record and expensive to recompute.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct EntryFlags: u32 {
        /// Several paths point at the same [`ObjectId`].
        const HARDLINKED = 1 << 0;
        /// Symlink / junction / reparse point; see the symlink policy contract.
        const LINK = 1 << 1;
        /// The link target could not be resolved.
        const BROKEN_LINK = 1 << 2;
        /// Crossing it would leave the requested filesystem(s).
        const MOUNT_BOUNDARY = 1 << 3;
        /// Size on disk is smaller than `logical_size`.
        const SPARSE = 1 << 4;
        /// Content could not be read (permission, IO, decryption, ...).
        const UNREADABLE = 1 << 5;
        /// Metadata changed between the scan pass and the fingerprint pass.
        const CHANGED_DURING_SCAN = 1 << 6;
        /// Object identity could not be obtained.
        const OBJECT_ID_UNKNOWN = 1 << 7;
    }
}

/// One storage object as seen by the core.
///
/// Keep this small: at tens of millions of entries, every extra field is
/// hundreds of megabytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    pub id: EntryId,
    pub source: SourceId,
    pub locator: LocatorId,
    /// `None` when identity is unavailable; `Err(reason)` style information is
    /// carried by [`EntryFlags::OBJECT_ID_UNKNOWN`] plus an entry-level error.
    pub object: Option<ObjectId>,
    /// Present only when the plan materializes hierarchy.
    pub parent: Option<EntryId>,
    pub kind: EntryKind,
    pub flags: EntryFlags,
    /// Logical size, i.e. what the content claims. It is **not** allocated size:
    /// hardlinks, sparse files, compression and CoW all break that assumption.
    pub logical_size: Option<u64>,
    pub modified: Option<Timestamp>,
    /// Metadata captured at scan time; the fingerprint pass re-stats and compares.
    pub snapshot: Snapshot,
}

impl Entry {
    /// True when the entry can be a duplicate candidate at all.
    pub fn is_duplicate_candidate(&self) -> bool {
        matches!(self.kind, EntryKind::File | EntryKind::Asset)
            && !self.flags.contains(EntryFlags::UNREADABLE)
            && !self.flags.contains(EntryFlags::CHANGED_DURING_SCAN)
    }
}
