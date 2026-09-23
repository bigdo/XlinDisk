# ADR-001: Core and applications are separate

- **Status:** Accepted (2026-09-22)
- **Scope:** xlindisk-core / Desktop / CLI / MCP / Mobile

## Decision

Core 拥有模型、Plan、Runtime、Source contract 与执行性能相关的能力；
前端（Desktop、CLI、MCP、Mobile、第三方）只描述目标并消费结果。
前端不得拥有自己的扫描、分组或 hash 实现。

## Consequences

* 任何新能力都应是 Plan composition，而不是新的 finder 模块（见 ADR 的另一面：
  Duplicate 是 preset，不是 DuplicateFinder）。
* CLI 是 integration harness：只能建 Plan、调 Engine。
* core 的可移植性成为硬约束（iOS / Android compile smoke）。
