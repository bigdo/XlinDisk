//! Entry storage: dense ids in contiguous memory, hierarchy only on demand.
//!
//! At tens of millions of entries, a per-entry `PathBuf`, `Box<Node>` or
//! `Rc<Node>` turns into hundreds of megabytes. The store therefore keeps
//! entries in one `Vec`, models hierarchy with sibling/child indexes that are
//! built **only** when a plan asks for materialization, and keeps the metadata
//! snapshot out of `Entry` entirely.

use std::collections::HashMap;

use crate::model::entry::{Entry, EntryFlags, EntryKind};
use crate::model::ids::{EntryId, LocatorId, ObjectId, ScanId, SessionId, SourceId};
use crate::model::observation::{Observation, Timestamp};

/// A scan session: the scope in which ids, locators and object identities mean
/// something.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Session {
    pub id: SessionId,
    pub source: SourceId,
    next_scan: u64,
}

impl Session {
    pub fn new(id: SessionId, source: SourceId) -> Self {
        Self {
            id,
            source,
            next_scan: 0,
        }
    }

    pub fn next_scan_id(&mut self) -> ScanId {
        self.next_scan += 1;
        ScanId(self.next_scan)
    }
}

/// Contiguous entry storage with optional hierarchy links.
#[derive(Clone, Debug, Default)]
pub struct EntryStore {
    entries: Vec<Entry>,
    /// `first_child` / `next_sibling` are compact indexes, not pointers: 8 bytes
    /// each, and the arena stays relocatable.
    first_child: Vec<Option<EntryId>>,
    next_sibling: Vec<Option<EntryId>>,
    /// Observations, recorded only for entries the fingerprint pass will
    /// actually look at. An `Observation` is ~120 bytes, so it never lives in
    /// `Entry`.
    observations: HashMap<EntryId, Observation>,
    /// Set once the caller asks for hierarchy; before that the link vectors are
    /// empty and cost nothing.
    materialized: bool,
}

impl EntryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            first_child: Vec::new(),
            next_sibling: Vec::new(),
            observations: HashMap::new(),
            materialized: false,
        }
    }

    /// Append an entry and return its dense id.
    ///
    /// The id is positional, so a scan can append from many workers into
    /// per-worker stores and merge them afterwards; deterministic ordering is
    /// applied at output time (see `determinism`).
    pub fn push(&mut self, entry: Entry) -> EntryId {
        let id = EntryId(self.entries.len() as u32);
        if self.materialized {
            self.first_child.push(None);
            self.next_sibling.push(None);
            if let Some(parent) = entry.parent {
                self.link_child(parent, id);
            }
        }
        self.entries.push(entry);
        id
    }

    /// Convenience constructor for sources: fills in the dense id itself.
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        source: SourceId,
        locator: LocatorId,
        kind: EntryKind,
        flags: EntryFlags,
        parent: Option<EntryId>,
        logical_size: Option<u64>,
        modified: Option<Timestamp>,
        object: Option<ObjectId>,
    ) -> EntryId {
        let entry = Entry {
            id: EntryId(self.entries.len() as u32),
            source,
            locator,
            object,
            parent,
            kind,
            flags,
            logical_size,
            modified,
        };
        self.push(entry)
    }

    pub fn get(&self, id: EntryId) -> Option<&Entry> {
        self.entries.get(id.0 as usize)
    }

    pub fn get_mut(&mut self, id: EntryId) -> Option<&mut Entry> {
        self.entries.get_mut(id.0 as usize)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }

    /// Entries that may enter the duplicate pipeline at all.
    pub fn duplicate_candidates(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(|e| e.is_duplicate_candidate())
    }

    /// Record the pre-operation observation. The fingerprint pass re-observes
    /// and validates against this; without it there is no way to detect
    /// `ChangedDuringScan`.
    pub fn record_observation(&mut self, id: EntryId, observation: Observation) {
        self.observations.insert(id, observation);
    }

    pub fn observation(&self, id: EntryId) -> Option<&Observation> {
        self.observations.get(&id)
    }

    /// Build the child/sibling indexes. Cheap to call once, wasteful to keep for
    /// plans that only need a flat stream.
    pub fn materialize_hierarchy(&mut self) {
        if self.materialized {
            return;
        }
        self.first_child = vec![None; self.entries.len()];
        self.next_sibling = vec![None; self.entries.len()];
        self.materialized = true;
        let parents: Vec<Option<EntryId>> = self.entries.iter().map(|e| e.parent).collect();
        for (child, parent) in parents.iter().enumerate() {
            if let Some(parent) = parent {
                self.link_child(*parent, EntryId(child as u32));
            }
        }
    }

    pub fn is_materialized(&self) -> bool {
        self.materialized
    }

    /// Children in insertion order.
    ///
    /// Insertion order is *not* output order: canonical, byte-level ordering is
    /// applied when results are produced, so worker count never changes output.
    pub fn children(&self, parent: EntryId) -> ChildIter<'_> {
        let next = if self.materialized {
            self.first_child.get(parent.0 as usize).copied().flatten()
        } else {
            None
        };
        ChildIter { store: self, next }
    }

    fn link_child(&mut self, parent: EntryId, child: EntryId) {
        let parent_index = parent.0 as usize;
        if parent_index >= self.first_child.len() {
            return;
        }
        match self.first_child[parent_index] {
            None => self.first_child[parent_index] = Some(child),
            Some(first) => {
                // Append to the end of the sibling chain to preserve insertion
                // order; hot paths build whole subtrees at once, so this stays
                // linear in the number of siblings, not in the store size.
                let mut current = first;
                while let Some(next) = self.next_sibling.get(current.0 as usize).copied().flatten()
                {
                    current = next;
                }
                if let Some(slot) = self.next_sibling.get_mut(current.0 as usize) {
                    *slot = Some(child);
                }
            }
        }
    }

    /// Group the given entries by object identity, i.e. hardlinks.
    ///
    /// Scoped to a candidate list on purpose: a global `ObjectId -> entries`
    /// map over ten million mostly-unique objects would cost more memory than
    /// the entries themselves. Hardlinks only matter inside a duplicate group,
    /// which is exactly what PR7 will pass here.
    ///
    /// Groups with a single member are dropped; each returned group is one
    /// stored object, never N reclaimable copies.
    pub fn hardlink_groups(&self, candidates: &[EntryId]) -> Vec<Vec<EntryId>> {
        let mut groups: Vec<(ObjectId, Vec<EntryId>)> = Vec::new();
        let mut index: HashMap<ObjectId, usize> = HashMap::new();
        for id in candidates {
            let Some(entry) = self.get(*id) else { continue };
            let Some(object) = entry.object else { continue };
            match index.get(&object) {
                Some(slot) => groups[*slot].1.push(*id),
                None => {
                    index.insert(object, groups.len());
                    groups.push((object, vec![*id]));
                }
            }
        }
        groups
            .into_iter()
            .filter(|(_, members)| members.len() > 1)
            .map(|(_, members)| members)
            .collect()
    }

    /// Mark an entry as changed during the scan; it then drops out of the
    /// duplicate pipeline.
    pub fn mark_changed_during_scan(&mut self, id: EntryId) -> bool {
        match self.get_mut(id) {
            Some(entry) => {
                entry.flags |= EntryFlags::CHANGED_DURING_SCAN;
                true
            }
            None => false,
        }
    }
}

/// Iterator over the children of one entry.
pub struct ChildIter<'a> {
    store: &'a EntryStore,
    next: Option<EntryId>,
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next?;
        self.next = self
            .store
            .next_sibling
            .get(id.0 as usize)
            .copied()
            .flatten();
        self.store.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::VolumeId;

    fn object_id(value: u128) -> ObjectId {
        ObjectId::new(SourceId(1), VolumeId(1), value)
    }

    fn entry(parent: Option<EntryId>, object: Option<u128>, kind: EntryKind) -> Entry {
        Entry {
            id: EntryId::INVALID,
            source: SourceId(1),
            locator: LocatorId(0),
            object: object.map(object_id),
            parent,
            kind,
            flags: EntryFlags::empty(),
            logical_size: Some(10),
            modified: None,
        }
    }

    #[test]
    fn ids_are_dense_and_positional() {
        let mut store = EntryStore::new();
        let a = store.push(entry(None, None, EntryKind::Directory));
        let b = store.push(entry(Some(a), None, EntryKind::File));
        assert_eq!(a, EntryId(0));
        assert_eq!(b, EntryId(1));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn add_builds_a_complete_entry() {
        let mut store = EntryStore::new();
        let id = store.add(
            SourceId(1),
            LocatorId(5),
            EntryKind::File,
            EntryFlags::empty(),
            None,
            Some(42),
            Some(Timestamp::new(1_700_000_000, 0)),
            Some(object_id(7)),
        );
        let stored = store.get(id).unwrap();
        assert_eq!(stored.locator, LocatorId(5));
        assert_eq!(stored.object, Some(object_id(7)));
        assert_eq!(stored.logical_size, Some(42));
    }

    #[test]
    fn hierarchy_is_only_built_when_asked_for() {
        let mut store = EntryStore::new();
        let root = store.push(entry(None, None, EntryKind::Directory));
        store.push(entry(Some(root), None, EntryKind::File));
        assert!(!store.is_materialized());
        assert_eq!(store.children(root).count(), 0, "no links yet");
        store.materialize_hierarchy();
        assert!(store.is_materialized());
        assert_eq!(store.children(root).count(), 1);
    }

    #[test]
    fn hardlinks_are_one_object_not_many_copies() {
        let mut store = EntryStore::new();
        let a = store.push(entry(None, Some(7), EntryKind::File));
        let b = store.push(entry(None, Some(7), EntryKind::File));
        let c = store.push(entry(None, Some(8), EntryKind::File));
        let groups = store.hardlink_groups(&[a, b, c]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec![a, b]);
    }

    #[test]
    fn hardlink_grouping_is_scoped_to_candidates() {
        let mut store = EntryStore::new();
        let a = store.push(entry(None, Some(7), EntryKind::File));
        let b = store.push(entry(None, Some(7), EntryKind::File));
        // Only entries the caller asks about are grouped: no global map.
        assert!(store.hardlink_groups(&[a]).is_empty());
        assert!(store.hardlink_groups(&[a, b]).len() == 1);
    }

    #[test]
    fn observations_are_recorded_per_candidate() {
        let mut store = EntryStore::new();
        let id = store.push(entry(None, None, EntryKind::File));
        assert!(store.observation(id).is_none());
        let observation = Observation {
            source: SourceId(1),
            locator: LocatorId(0),
            object: Some(object_id(3)),
            logical_size: Some(10),
            modified: Some(Timestamp::new(1_700_000_000, 0)),
            revision: None,
        };
        store.record_observation(id, observation);
        assert_eq!(store.observation(id), Some(&observation));
    }

    #[test]
    fn changed_entries_drop_out_of_the_duplicate_pipeline() {
        let mut store = EntryStore::new();
        let id = store.push(entry(None, None, EntryKind::File));
        assert_eq!(store.duplicate_candidates().count(), 1);
        assert!(store.mark_changed_during_scan(id));
        assert_eq!(store.duplicate_candidates().count(), 0);
    }

    #[test]
    fn directories_and_unreadable_files_are_not_candidates() {
        let mut store = EntryStore::new();
        store.push(entry(None, None, EntryKind::Directory));
        let mut unreadable = entry(None, None, EntryKind::File);
        unreadable.flags |= EntryFlags::UNREADABLE;
        store.push(unreadable);
        assert_eq!(store.duplicate_candidates().count(), 0);
    }

    #[test]
    fn entry_stays_small_enough_for_tens_of_millions() {
        // Guard rail from the memory principle: at 10M entries every byte costs
        // 10 MB, so a regression fails the build instead of showing up as an OOM
        // on somebody's disk.
        const BUDGET: usize = 128;
        assert!(
            std::mem::size_of::<Entry>() <= BUDGET,
            "Entry grew to {} bytes, budget is {}",
            std::mem::size_of::<Entry>(),
            BUDGET
        );
    }

    #[test]
    fn session_issues_scan_ids() {
        let mut session = Session::new(SessionId(1), SourceId(1));
        assert_eq!(session.next_scan_id(), ScanId(1));
        assert_eq!(session.next_scan_id(), ScanId(2));
    }
}
