# ADR-017: Public APIs do not expose runtime implementation primitives

- **Status:** Accepted (2026-09-23, r1)

## Decision

公共 API 不暴露 runtime 实现原语：

```text
不出现 crossbeam / Rayon / Tokio 类型
不出现具体 scheduler / worker pool 类型
```

内部实现可以随时替换（work stealing、单 scheduler、未来的别的调度器），
而 front-end 与移动端 adapter 不需要知道。

## Consequences

* 公共面只有：模型、Plan、`Source` trait、Runtime 的**可观察行为**
  （`RunStatus` / `ProgressEvent` / `CancellationToken` / `ErrorCode`）。
* `SourceCapabilities` 用正面能力位表达（`CONCURRENT_STAT` / `CONCURRENT_READ`），
  Runtime 据此决定并行度，而不是假定所有 Source 线程安全。
* 这条才是真正的 architecture contract；ADR-010 描述的 dispatch 形式不是。
