# PR0 · Contract Freeze

**Gate: PR0 未合并，不开工 PR3（ReferenceWalker）/ PR4（ParallelWalker）。**

PR0 只冻结语义与类型，不实现 filesystem 逻辑。它的存在只有一个理由：并发 walker
一旦按错误的语义实现，之后所有 benchmark、fixture 与结果 schema 都会围着错误结论重建。

## 冻结项 → 文档 → 代码

| 冻结项 | 文档 | 代码 |
| --- | --- | --- |
| ObjectId 作用域与缺失策略 | [01](01-object-identity.md) | `model::ids::{ObjectId, ObjectIdUnavailable}` |
| Snapshot / 观察一致性 | [02](02-observation-consistency.md) | `model::observation::{Snapshot, ObservationClass}` |
| Locator contract | [03](03-locator-and-source-contract.md) | `source::Source::{open_content, stat, display_locator}` |
| Source trait / EntryBatch | [03](03-locator-and-source-contract.md) | `source::{Source, EntryBatch, EntrySink}` |
| RunStatus / 错误 / 取消 / Progress | [04](04-run-status-errors-cancellation.md) | `error::ErrorCode`、`runtime::*` |
| Determinism | [05](05-determinism.md) | `determinism::GroupOrderKey` |
| Duplicate 中间状态与内存模型 | [06](06-duplicate-memory-model.md) | `duplicate::CandidateBudget` |
| 结果 schema / 算法版本 | [07](07-result-schema.md) | `model::observation::Record` |
| Plan node 输入 / 输出类型 | — | `plan::PlanNode::io` |
| 跨平台 correctness fixture 清单 | [08](08-cross-platform-fixtures.md) | PR3 落地 |
| API 与实现解耦 | [../ARCHITECTURE.md](../ARCHITECTURE.md#6-api-与实现解耦) | ADR-010（已降为 benchmark 假设） |

## v0.0.1 范围冻结

* 只输出 **逻辑重复文件**，不输出「可释放物理空间」。
  `logical_size` 不等于真实磁盘占用：hardlink、sparse file、压缩与 CoW 都会打破这个等式。
* read-only：core 内不存在 unlink / move / trash 能力。
* 未来 MCP 与安全删除只**预留字段**，不实现。删除必须由独立 Action / Validation 层
  重新验证对象后执行，不能依据路径或 digest 直接删除。

## 变更规则

改这里的任何一项，都是 contract revision：

1. 更新对应文档与本页表格；
2. 更新类型 / 常量（含 `CORE_CONTRACT_VERSION`、`PLAN_VERSION`、
   `FINGERPRINT_ALGORITHM_VERSION`）；
3. 更新受影响的 fixture 与 release gate 条目。

不接受「先改代码，文档以后补」。
