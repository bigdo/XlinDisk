//! Entry: the smallest thing the core is willing to say about a storage object.

use crate::model::ids::{EntryId, LocatorId, ObjectId, SourceId};
use crate::model::observation::Timestamp;

/// What the object is, in the most general sense.
///
/// No `Asset` variant: whether a PhotoKit `PHAsset` is a new kind or just a
/// provider object is a question to answer when PhotoKit actually arrives, not
/// now (same rule as `ComputeBackend`: no abstraction without a second real
/// implementation).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    /// Device, socket, fifo and friends; enumerated, never opened for content.
    Special,
    Other,
}

/// Refinement of [`EntryKind::Special`], encoded in flags so it costs nothing
/// per entry.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SpecialSubtype {
    Socket,
    Fifo,
    BlockDevice,
    CharDevice,
    /// Something platform-specific the core does not model further.
    Unknown,
}

bitflags::bitflags! {
    /// Per-entry observations that are cheap to record and expensive to recompute.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct EntryFlags: u32 {
        /// Several paths point at the same [`ObjectId`].
        const HARDLINKED = 1 << 0;
        /// Symlink / junction / reparse point; see the symlink policy contract.
        const LINK = 1 << 1;
        /// The link target could not be resolved (broken symlink).
        const TARGET_UNAVAILABLE = 1 << 2;
        /// Crossing it would leave the requested filesystem(s).
        const MOUNT_BOUNDARY = 1 << 3;
        /// Windows reparse point / junction.
        const REPARSE_POINT = 1 << 4;
        /// Size on disk is smaller than `logical_size`.
        const SPARSE = 1 << 5;
        /// Content could not be read (permission, IO, decryption, ...).
        const UNREADABLE = 1 << 6;
        /// Metadata changed between the scan pass and the fingerprint pass.
        const CHANGED_DURING_SCAN = 1 << 7;
        /// Object identity could not be obtained.
        const OBJECT_ID_UNKNOWN = 1 << 8;
        /// `Special`: socket.
        const SOCKET = 1 << 9;
        /// `Special`: fifo / named pipe.
        const FIFO = 1 << 10;
        /// `Special`: block device.
        const BLOCK_DEVICE = 1 << 11;
        /// `Special`: character device.
        const CHAR_DEVICE = 1 << 12;
    }
}

/// One storage object as seen by the core.
///
/// Keep this small: at tens of millions of entries, every extra field is
/// hundreds of megabytes. The metadata snapshot is therefore **not** in here —
/// it is recorded per candidate in [`crate::model::store::EntryStore`] when the
/// fingerprint pass starts (a `Snapshot` alone is 112 bytes).
///
/// `model::store::tests::entry_stays_small_enough_for_tens_of_millions` fails the
/// build if this struct grows past its budget.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    pub id: EntryId,
    pub source: SourceId,
    pub locator: LocatorId,
    /// `None` when identity is unavailable; the reason is carried by
    /// [`EntryFlags::OBJECT_ID_UNKNOWN`] plus an entry-level error.
    ///
    /// 48 bytes — `u128` (Windows 128-bit file ids) forces 16-byte alignment.
    /// It is the most expensive field here and the first one to revisit if the
    /// per-entry budget has to shrink further.
    pub object: Option<ObjectId>,
    /// Present only when the plan materializes hierarchy.
    pub parent: Option<EntryId>,
    pub kind: EntryKind,
    pub flags: EntryFlags,
    /// Logical size, i.e. what the content claims. It is **not** allocated size:
    /// hardlinks, sparse files, compression and CoW all break that assumption.
    pub logical_size: Option<u64>,
    pub modified: Option<Timestamp>,
}

impl Entry {
    /// True when the entry can be a duplicate candidate at all.
    pub fn is_duplicate_candidate(&self) -> bool {
        matches!(self.kind, EntryKind::File)
            && !self.flags.contains(EntryFlags::UNREADABLE)
            && !self.flags.contains(EntryFlags::CHANGED_DURING_SCAN)
    }

    /// Special files are enumerated but never opened, and never hashed.
    pub fn special_subtype(&self) -> Option<SpecialSubtype> {
        if !matches!(self.kind, EntryKind::Special) {
            return None;
        }
        Some(if self.flags.contains(EntryFlags::SOCKET) {
            SpecialSubtype::Socket
        } else if self.flags.contains(EntryFlags::FIFO) {
            SpecialSubtype::Fifo
        } else if self.flags.contains(EntryFlags::BLOCK_DEVICE) {
            SpecialSubtype::BlockDevice
        } else if self.flags.contains(EntryFlags::CHAR_DEVICE) {
            SpecialSubtype::CharDevice
        } else {
            SpecialSubtype::Unknown
        })
    }
}
