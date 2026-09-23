# ADR-006: A single scheduler owns concurrency

- **Status:** Accepted (2026-09-22)

## Decision

并发由 Core 自己的一个 scheduler 拥有，不混用 Rayon、Tokio 与 BLAKE3 Rayon，
避免 oversubscription。并发能力由 Source 自己声明：
`SourceCapabilities::CONCURRENT_STAT` / `CONCURRENT_READ`；未声明者由 Runtime 串行化
（见 ADR-017）。

## Consequences

* v0.0.1 不引入 Tokio（Core 不绑定 async runtime）与 Rayon；
  BLAKE3 不启用 Rayon feature，单文件 hash 用它自带的 SIMD runtime dispatch。
* 实现选型（crossbeam-deque / channel / 自研）属于实现细节，**不进公共 API**。
* 若 benchmark 证明单超大文件内部并行 hash 有价值，由 Runtime 针对特定 workload
  分配资源，而不是引入第二套 scheduler。
