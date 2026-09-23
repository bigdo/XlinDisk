//! Duplicate pipeline: staging, the candidate record and the memory contract.
//!
//! A streaming scanner does **not** imply a low-memory duplicate query.
//! Duplicate detection has to hold candidates until the last stage, so the cost
//! is an explicit, bounded budget rather than a hope.

use crate::model::fingerprint::Fingerprint;
use crate::model::ids::{EntryId, LocatorId, ObjectId};
use crate::model::observation::Observation;

/// The three stages of v0.0.1.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DuplicateStage {
    /// Exact logical size grouping.
    Size,
    /// Cheap pre-filter (prefix + suffix).
    PartialFingerprint,
    /// Whole-content BLAKE3.
    FullFingerprint,
}

impl DuplicateStage {
    /// The frozen order of the pipeline.
    pub const ORDER: [DuplicateStage; 3] = [
        DuplicateStage::Size,
        DuplicateStage::PartialFingerprint,
        DuplicateStage::FullFingerprint,
    ];
}

/// The duplicate memory budget.
///
/// v0.0.1 keeps candidates **in memory only**. Spilling to disk would drag in
/// serialization, crash cleanup, temp-directory policy, quotas and resume
/// semantics — that is a future `DiskBackedCandidateStore`, and there is no
/// trait for it until a second implementation actually exists (same rule as
/// `ComputeBackend`).
///
/// Being bounded is the point: exceeding the budget is
/// `ErrorCode::ResourceLimitExceeded` and the run degrades to
/// `RunStatus::Partial`, instead of quietly OOM-ing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DuplicateBudget {
    /// Hard ceiling on candidate memory. `None` means "host default".
    pub max_candidate_bytes: Option<u64>,
    /// Maximum number of candidates kept between stages.
    pub max_candidates: Option<u64>,
    /// A size group larger than this is not fully hashed.
    pub max_group_members: Option<u32>,
    /// Re-open the locator and read again instead of caching content. Cheap on
    /// local disks, expensive on network and cloud sources.
    pub re_read_instead_of_cache: bool,
}

impl Default for DuplicateBudget {
    fn default() -> Self {
        Self {
            max_candidate_bytes: None,
            max_candidates: None,
            max_group_members: None,
            re_read_instead_of_cache: true,
        }
    }
}

impl DuplicateBudget {
    pub fn allows_group(&self, members: u32) -> bool {
        match self.max_group_members {
            Some(max) => members <= max,
            None => true,
        }
    }

    pub fn allows_more_candidates(&self, current: u64) -> bool {
        match self.max_candidates {
            Some(max) => current < max,
            None => true,
        }
    }

    /// Bytes one candidate costs, for budget accounting.
    pub const fn candidate_cost() -> u64 {
        std::mem::size_of::<Candidate>() as u64
    }
}

/// The compact per-candidate record.
///
/// Deliberately **not** a copy of `Entry`: no display name, no path, no full
/// metadata object. Millions of these have to fit in the budget.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Candidate {
    pub entry: EntryId,
    pub locator: LocatorId,
    pub object: Option<ObjectId>,
    pub logical_size: Option<u64>,
    /// Observation taken before hashing; re-validated afterwards.
    pub observation: Observation,
}

/// Why a candidate is not part of a confirmed group.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExclusionReason {
    ChangedDuringScan,
    PermissionChanged,
    Unreadable,
    /// Group exceeded `max_group_members`.
    GroupTooLarge,
    /// The run ran out of budget.
    BudgetExceeded,
    /// Not enough members to be a duplicate at all.
    Singleton,
}

/// A candidate that did not make it into a group.
///
/// Kept in run diagnostics instead of being silently dropped: "we did not check
/// this" and "this is not a duplicate" are different statements.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExcludedCandidate {
    pub entry: EntryId,
    pub locator: LocatorId,
    pub reason: ExclusionReason,
}

/// One confirmed duplicate group.
///
/// Two levels on purpose:
///
/// * `content` — everything with the same fingerprint;
/// * `object_groups` — how those paths map onto distinct stored objects.
///
/// `[A, B]` sharing an `ObjectId` and `C` not sharing it is one content group of
/// three members but two objects. That is what a GUI needs to show hardlinks and
/// what a cleanup planner needs to avoid deleting the wrong thing.
///
/// There is no `reclaimable_bytes`: logical size, allocated size and exclusive
/// physical size are three different numbers once hardlinks, sparse files,
/// compression, clones and CoW are involved. At most `total_logical_bytes`.
#[derive(Clone, Debug, Default)]
pub struct DuplicateGroup {
    pub fingerprint: Option<Fingerprint>,
    /// Members with the same content, in canonical order.
    pub content: Vec<EntryId>,
    /// Distinct stored objects among `content`; each inner list are hardlinks.
    pub object_groups: Vec<Vec<EntryId>>,
    /// Sum of logical sizes across `content`. Not reclaimable space.
    pub total_logical_bytes: u64,
    /// Candidates that were considered but excluded.
    pub excluded: Vec<ExcludedCandidate>,
}

impl DuplicateGroup {
    pub fn member_count(&self) -> usize {
        self.content.len()
    }

    pub fn object_count(&self) -> usize {
        if self.object_groups.is_empty() && !self.content.is_empty() {
            1
        } else {
            self.object_groups.len()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_order_is_size_then_partial_then_full() {
        assert_eq!(
            DuplicateStage::ORDER,
            [
                DuplicateStage::Size,
                DuplicateStage::PartialFingerprint,
                DuplicateStage::FullFingerprint
            ]
        );
    }

    #[test]
    fn group_cap_is_enforced() {
        let budget = DuplicateBudget {
            max_group_members: Some(64),
            ..DuplicateBudget::default()
        };
        assert!(budget.allows_group(64));
        assert!(!budget.allows_group(65));
    }

    #[test]
    fn budget_stops_adding_candidates() {
        let budget = DuplicateBudget {
            max_candidates: Some(10),
            ..DuplicateBudget::default()
        };
        assert!(budget.allows_more_candidates(9));
        assert!(!budget.allows_more_candidates(10));
    }

    #[test]
    fn hardlinks_collapse_to_one_object() {
        let group = DuplicateGroup {
            fingerprint: Some(Fingerprint::ZERO),
            content: vec![EntryId(0), EntryId(1), EntryId(2)],
            object_groups: vec![vec![EntryId(0), EntryId(1)], vec![EntryId(2)]],
            total_logical_bytes: 300,
            excluded: Vec::new(),
        };
        assert_eq!(group.member_count(), 3);
        assert_eq!(group.object_count(), 2, "A and B are one object, C another");
    }

    #[test]
    fn no_object_information_means_one_assumed_object() {
        let group = DuplicateGroup {
            content: vec![EntryId(0), EntryId(1)],
            ..DuplicateGroup::default()
        };
        assert_eq!(group.object_count(), 1);
    }

    #[test]
    fn candidate_stays_compact() {
        // Candidates are the memory-critical type in this pipeline.
        const BUDGET: usize = 256;
        assert!(
            std::mem::size_of::<Candidate>() <= BUDGET,
            "Candidate grew to {} bytes, budget is {}",
            std::mem::size_of::<Candidate>(),
            BUDGET
        );
    }
}
