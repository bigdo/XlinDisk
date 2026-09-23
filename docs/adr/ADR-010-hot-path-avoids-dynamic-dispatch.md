# ADR-010: Hot path avoids per-entry dynamic dispatch

- **Status:** Accepted **as a benchmark hypothesis** (2026-09-22)
- **Amended:** 2026-09-23 — 架构 review 后从「架构硬约束」降级为 benchmark 假设

## Decision

hot path 应尽量避免**每 entry** 的动态分发：优先 dense integer ID、arena、
shared string storage。但 Source 抽象边界使用泛型、trait object 还是 enum，
**由 benchmark 决定**，不再是架构前置约束。

## Consequences

* 当前骨架在**内容读取**边界使用 trait object（`Box<dyn ContentReader + Send>`），
  不在每 entry 的 hot path 上。
* Entry 用 dense `EntryId` + compact 字段；只有需要 hierarchy materialization 的
  Plan 才建树。
* 若 benchmark 表明 trait object 成为瓶颈，允许改为泛型 / enum 而不算违约。
