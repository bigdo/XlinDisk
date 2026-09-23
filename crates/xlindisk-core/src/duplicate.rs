//! Duplicate pipeline: staging and the memory model.
//!
//! A streaming scanner does **not** imply a low-memory duplicate query:
//! duplicate detection has to hold on to candidates until the last stage. That
//! cost is frozen here as an explicit budget instead of being discovered in
//! production.

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

/// Where intermediate candidate state may go when memory runs out.
///
/// A path is deliberately *not* modelled here: the core has no notion of a
/// path, so the host names a target and resolves it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpillTarget {
    /// Whatever the platform calls a temporary directory.
    SystemTemp,
    /// Host-resolved location, interpreted outside the core.
    Named,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpillPolicy {
    /// Refuse to spill: exceeding the budget is an error, not a slow scan.
    Forbid,
    /// Spill intermediate state to the given target.
    TempFile(SpillTarget),
}

/// The duplicate memory budget.
///
/// Without a persistent cache (v0.0.1 has none), these numbers are the only
/// thing standing between a dense-duplicate volume and an OOM kill, so they are
/// part of the contract and are exercised by the benchmark in PR8.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CandidateBudget {
    /// Maximum number of candidate entries kept between stages.
    pub max_candidates: Option<u64>,
    /// A single size group larger than this is not fully hashed.
    pub max_group_members: Option<u32>,
    /// What to do when a limit is hit.
    pub spill: SpillPolicy,
    /// Re-open the locator and read a second time instead of caching content.
    /// Cheap on local disks, expensive on network and cloud sources.
    pub re_read_instead_of_cache: bool,
}

impl Default for CandidateBudget {
    fn default() -> Self {
        Self {
            max_candidates: None,
            max_group_members: None,
            spill: SpillPolicy::Forbid,
            re_read_instead_of_cache: true,
        }
    }
}

impl CandidateBudget {
    /// True when a group of `members` candidates fits the budget.
    pub fn allows_group(&self, members: u32) -> bool {
        match self.max_group_members {
            Some(max) => members <= max,
            None => true,
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
    fn default_budget_does_not_spill() {
        let budget = CandidateBudget::default();
        assert_eq!(budget.spill, SpillPolicy::Forbid);
        assert!(budget.allows_group(u32::MAX));
    }

    #[test]
    fn group_cap_is_enforced() {
        let budget = CandidateBudget {
            max_group_members: Some(64),
            ..CandidateBudget::default()
        };
        assert!(budget.allows_group(64));
        assert!(!budget.allows_group(65));
    }
}
