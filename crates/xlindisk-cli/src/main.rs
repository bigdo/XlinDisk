//! `xlindisk` CLI — integration harness, not a product.
//!
//! The CLI may only build plans and call the engine; every scan, grouping and
//! hashing decision lives in the core. Real commands arrive with the executor
//! (PR5) and the duplicate preset (PR7).

use xlindisk_core::model::fingerprint::FingerprintSpec;
use xlindisk_core::model::ids::{SessionId, SourceId};
use xlindisk_core::plan::{FilterExpr, GroupKey, GroupSpec, Plan, PlanNode, PLAN_VERSION};
use xlindisk_core::source::ScanRequest;
use xlindisk_core::CORE_CONTRACT_VERSION;

fn duplicate_plan(source: SourceId, session: SessionId) -> Plan {
    let mut plan = Plan::new();
    plan.push(PlanNode::Scan(xlindisk_core::plan::ScanSpec {
        source,
        request: ScanRequest::new(source, session, vec![]),
    }));
    plan.push(PlanNode::Filter(FilterExpr::IsFile));
    plan.push(PlanNode::Group(GroupSpec {
        key: GroupKey::Size,
    }));
    plan.push(PlanNode::Filter(FilterExpr::GroupCountAtLeast(2)));
    plan.push(PlanNode::Fingerprint(FingerprintSpec::default()));
    plan
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("xlindisk-cli (skeleton)");
    println!("core contract v{CORE_CONTRACT_VERSION} / plan v{PLAN_VERSION}");
    println!("commands land with the executor (PR5) and duplicate preset (PR7)");

    if args.iter().any(|a| a == "--print-duplicate-plan") {
        let plan = duplicate_plan(SourceId(0), SessionId(0));
        match plan.validate() {
            Ok(()) => {
                for node in &plan.nodes {
                    let (input, output) = node.io();
                    println!("  {node:?}: {input:?} -> {output:?}");
                }
            }
            Err(err) => println!("  invalid plan: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_duplicate_preset_type_checks() {
        assert!(duplicate_plan(SourceId(0), SessionId(0)).validate().is_ok());
    }
}
