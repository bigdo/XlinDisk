//! PLAN layer: applications compose nodes, the runtime decides how to run them.
//!
//! There is no `DuplicateFinder` and no `LargestFilesFinder`. Those are presets
//! over these nodes, so a new capability is a new composition rather than a new
//! engine.

use crate::error::{Error, ErrorCode};
use crate::model::fingerprint::FingerprintSpec;
use crate::model::ids::SourceId;
use crate::source::ScanRequest;

/// Version of the plan IR. Bumped through a contract revision, not per release.
pub const PLAN_VERSION: u32 = 1;

/// What a node consumes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NodeInput {
    Nothing,
    Entries,
    Groups,
    Ranked,
    /// Sort accepts either shape: ranking flat entries (largest files) and
    /// ranking groups (duplicate groups) are the same operation.
    EntriesOrGroups,
}

impl NodeInput {
    pub fn accepts(self, produced: NodeOutput) -> bool {
        match (self, produced) {
            (NodeInput::Nothing, NodeOutput::Nothing) => true,
            (NodeInput::Entries, NodeOutput::Entries) => true,
            (NodeInput::Groups, NodeOutput::Groups) => true,
            (NodeInput::Ranked, NodeOutput::Ranked) => true,
            (NodeInput::EntriesOrGroups, NodeOutput::Entries | NodeOutput::Groups) => true,
            _ => false,
        }
    }
}

/// What a node produces.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NodeOutput {
    Nothing,
    Entries,
    Groups,
    Ranked,
}

#[derive(Clone, Debug)]
pub struct ScanSpec {
    pub source: SourceId,
    pub request: ScanRequest,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AggregateOp {
    Count,
    SumLogicalSize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AggregateSpec {
    /// Summarize each group, e.g. directory sizes for a treemap.
    PerGroup(AggregateOp),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GroupKey {
    Size,
    Fingerprint,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GroupSpec {
    pub key: GroupKey,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortKey {
    Size,
    Fingerprint,
    /// Source-defined canonical key; byte-level comparison, never locale-aware.
    Locator,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SortSpec {
    pub key: SortKey,
    pub descending: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LimitSpec {
    pub count: u64,
}

/// Minimal filter language. Deliberately not an expression VM: v0.0.1 proves
/// the boundary, it does not build a query engine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterExpr {
    IsFile,
    SizeEq(u64),
    SizeGt(u64),
    /// Keep only groups with at least this many members (duplicate pruning).
    GroupCountAtLeast(u32),
}

impl FilterExpr {
    /// Entry-level filters keep the entry stream; group-level ones keep the
    /// group stream. The frozen type is what stops a filter being spliced into
    /// the wrong stage.
    pub fn io(&self) -> (NodeInput, NodeOutput) {
        match self {
            FilterExpr::GroupCountAtLeast(_) => (NodeInput::Groups, NodeOutput::Groups),
            FilterExpr::IsFile | FilterExpr::SizeEq(_) | FilterExpr::SizeGt(_) => {
                (NodeInput::Entries, NodeOutput::Entries)
            }
        }
    }
}

/// The plan IR.
#[derive(Clone, Debug)]
pub enum PlanNode {
    Scan(ScanSpec),
    Filter(FilterExpr),
    Group(GroupSpec),
    Fingerprint(FingerprintSpec),
    Sort(SortSpec),
    Aggregate(AggregateSpec),
    Limit(LimitSpec),
}

impl PlanNode {
    /// Frozen input/output types: a node cannot be spliced anywhere, and the
    /// executor can reject a plan before touching a disk.
    pub fn io(&self) -> (NodeInput, NodeOutput) {
        match self {
            PlanNode::Scan(_) => (NodeInput::Nothing, NodeOutput::Entries),
            PlanNode::Filter(expr) => expr.io(),
            PlanNode::Group(_) => (NodeInput::Entries, NodeOutput::Groups),
            PlanNode::Fingerprint(_) => (NodeInput::Groups, NodeOutput::Groups),
            PlanNode::Sort(_) => (NodeInput::EntriesOrGroups, NodeOutput::Ranked),
            PlanNode::Aggregate(_) => (NodeInput::Groups, NodeOutput::Groups),
            PlanNode::Limit(_) => (NodeInput::Ranked, NodeOutput::Ranked),
        }
    }
}

/// An ordered list of nodes.
#[derive(Clone, Debug)]
pub struct Plan {
    pub version: u32,
    pub nodes: Vec<PlanNode>,
}

impl Plan {
    pub fn new() -> Self {
        Self {
            version: PLAN_VERSION,
            nodes: Vec::new(),
        }
    }

    pub fn push(&mut self, node: PlanNode) {
        self.nodes.push(node);
    }

    /// Static validation: the chain must start with a scan and every node must
    /// accept what the previous one emits.
    pub fn validate(&self) -> Result<(), Error> {
        if self.version != PLAN_VERSION {
            return Err(Error::with_detail(
                ErrorCode::PlanInvalid,
                "plan version mismatch",
            ));
        }
        if self.nodes.is_empty() {
            return Err(Error::with_detail(ErrorCode::PlanInvalid, "empty plan"));
        }
        if !matches!(self.nodes.first(), Some(PlanNode::Scan(_))) {
            return Err(Error::with_detail(
                ErrorCode::PlanInvalid,
                "first node must be Scan",
            ));
        }
        let mut produced = NodeOutput::Nothing;
        for node in &self.nodes {
            let (input, output) = node.io();
            if !input.accepts(produced) {
                return Err(Error::with_detail(
                    ErrorCode::PlanInvalid,
                    "node input does not match previous output",
                ));
            }
            produced = output;
        }
        Ok(())
    }
}

impl Default for Plan {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::SessionId;

    fn scan_node() -> PlanNode {
        PlanNode::Scan(ScanSpec {
            source: SourceId(0),
            request: ScanRequest::new(SourceId(0), SessionId(0), vec![]),
        })
    }

    #[test]
    fn duplicate_chain_type_checks() {
        let mut plan = Plan::new();
        plan.push(scan_node());
        plan.push(PlanNode::Filter(FilterExpr::IsFile));
        plan.push(PlanNode::Group(GroupSpec {
            key: GroupKey::Size,
        }));
        plan.push(PlanNode::Filter(FilterExpr::GroupCountAtLeast(2)));
        plan.push(PlanNode::Fingerprint(FingerprintSpec::default()));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn largest_files_chain_type_checks() {
        let mut plan = Plan::new();
        plan.push(scan_node());
        plan.push(PlanNode::Filter(FilterExpr::IsFile));
        plan.push(PlanNode::Sort(SortSpec {
            key: SortKey::Size,
            descending: true,
        }));
        plan.push(PlanNode::Limit(LimitSpec { count: 100 }));
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn entry_filters_cannot_run_on_groups() {
        let mut plan = Plan::new();
        plan.push(scan_node());
        plan.push(PlanNode::Group(GroupSpec {
            key: GroupKey::Size,
        }));
        plan.push(PlanNode::Filter(FilterExpr::IsFile));
        assert_eq!(plan.validate().unwrap_err().code, ErrorCode::PlanInvalid);
    }

    #[test]
    fn misordered_nodes_are_rejected_before_any_io() {
        let mut plan = Plan::new();
        plan.push(scan_node());
        plan.push(PlanNode::Limit(LimitSpec { count: 10 }));
        assert_eq!(plan.validate().unwrap_err().code, ErrorCode::PlanInvalid);
    }

    #[test]
    fn a_plan_must_start_with_a_scan() {
        let mut plan = Plan::new();
        plan.push(PlanNode::Filter(FilterExpr::IsFile));
        assert_eq!(plan.validate().unwrap_err().code, ErrorCode::PlanInvalid);
    }
}
