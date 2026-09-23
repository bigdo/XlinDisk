# ADR-004: Plans describe work; Runtime executes work

- **Status:** Accepted (2026-09-22)

## Decision

上层用 Plan 描述**分析目标**（Scan / Filter / Group / Aggregate / Sort /
Fingerprint / Limit），Runtime 依据 source capability、storage profile 与硬件
条件决定**执行方式**。

## Consequences

* 不做 SQL、Join、subquery、expression VM 或 cost-based optimizer；
  SQL 若出现，是 frontend（SQL → Parser → Plan）。
* 每个 node 有冻结的输入 / 输出类型（`PlanNode::io`），plan 可在触碰磁盘前被拒绝。
* Duplicate 是 `duplicate_plan(options)` preset，不是独立引擎。
