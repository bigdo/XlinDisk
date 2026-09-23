# ADR-005: Core v0.0.1 is read-only

- **Status:** Accepted (2026-09-22)

## Decision

v0.0.1 的 Core 不提供 delete / trash / move / hardlink 等 mutation 能力。
结果 schema 预留未来安全删除所需的全部字段，但不实现删除。

## Consequences

* 第一版把复杂度集中在 correctness、architecture、memory、performance、portability。
* 删除属于独立 Action / Validation 层，执行前必须**重新验证对象**
  （re-stat + full hash，必要时 byte-exact），不能依据路径或 digest 直接删除。
* MCP 层负责路径 allowlist、分页、任务取消与显式确认。
