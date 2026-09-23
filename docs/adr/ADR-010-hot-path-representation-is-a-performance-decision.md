# ADR-010: Hot-path representation and dispatch are performance decisions validated by benchmarks

- **Status:** Accepted as an engineering guideline (2026-09-22)
- **Amended:** 2026-09-23 — r1：从「架构硬约束」正式降级为 **performance hypothesis**

## Context

原始表述是：

> Hot path avoids per-entry dynamic dispatch.

它读起来像 architecture law，但实际上 `trait object` / `enum dispatch` / generic /
monomorphization 哪个更快取决于 workload 和平台，属于 benchmark 才能回答的问题。

## Decision

hot path 的表示方式与分发方式**是性能决策**，由 benchmark 验证：

* 公共 API 不得暴露 crossbeam / Rayon / Tokio 类型（这是 architecture contract，
  见 ADR-017）；
* 内部 scheduler、dispatch 形式、arena 布局都可以替换；
* `trait object` / `enum` / 泛型的选择留给数据。

## Consequences

* 当前骨架在**内容读取**边界使用 `Box<dyn ContentReader + Send>`，不在每 entry 的
  hot path 上；若 benchmark 表明它是瓶颈，可以改为泛型或 enum 而不算违约。
* `Entry` 保持 dense id + compact 字段，理由是可测量的（有 size guard 测试），
  不是因为「避免 dynamic dispatch」。
* 这条 ADR 不再是拒绝某个实现的理由；它只是提醒要量。
