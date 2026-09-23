# PR0 · Semantic Contract Freeze

**状态：Architecture Draft r1（2026-09-23）——架构方向通过，进入 Semantic Contract Freeze。**

**Gate：PR0 未合并，不开工 PR3（ReferenceWalker）/ PR4（ParallelWalker）。**

## 为什么先做 PR0

ReferenceWalker 要成为后面所有平台实现、并行实现和 accelerator 的 correctness
oracle，前提是「正确」这件事已经被定义。否则 oracle 只是「第一个实现」，之后每个平台
都会各自解释 hardlink、链接、竞态和「算完了没」。

PR0 不写高性能代码。它冻结语义、类型、版本号和 fixture 清单。

## 核心模型升级

不是：

```text
Entry → Hash → Result
```

而是：

```text
Entry
  ↓
Observation
  ↓
Operation
  ↓
Re-observation
  ↓
Validation
  ↓
Result
```

filesystem scan、duplicate hash、cache、MCP、delete、PhotoKit、MediaStore 共用同一套
「观察到的对象可能发生变化」的语义。这比在某条 pipeline 里加一个 mtime 检查重要得多。

## 四组必须先冻结的东西

| 组 | 文档 | 代码 |
| --- | --- | --- |
| Observation consistency | [02](02-observation-consistency.md) | `model::observation::{Observation, ObservationValidation}` |
| Object / Locator semantics | [01](01-object-identity.md) · [03](03-locator-and-source-contract.md) | `model::ids::{ObjectId, LocatorId}`、`source::Source` |
| Run / Error semantics | [04](04-run-status-errors-cancellation.md) | `error::{ErrorCode, ErrorBudget}`、`runtime::*` |
| Duplicate resource model | [06](06-duplicate-memory-model.md) | `duplicate::{DuplicateBudget, Candidate, DuplicateGroup}` |

其余冻结项：

| 冻结项 | 文档 | 代码 |
| --- | --- | --- |
| Filesystem semantics（symlink / reparse / mount / special / sparse / 排序） | [spec/filesystem-semantics.md](../spec/filesystem-semantics.md) | `source::{LinkPolicy, ReparsePolicy, MountPolicy}` |
| Determinism | [05](05-determinism.md) | `determinism::GroupOrderKey`、`Source::sort_key` |
| 结果 schema / 版本 | [07](07-result-schema.md) | `model::observation::Record`、`RESULT_SCHEMA_VERSION` |
| Plan node 输入 / 输出 | — | `plan::PlanNode::io` |
| 跨平台 correctness fixture 清单 | [08](08-cross-platform-fixtures.md) | PR3 落地 |
| API 与实现解耦 | [../ARCHITECTURE.md](../ARCHITECTURE.md) | ADR-017 |

## v0.0.1 范围冻结

* 只输出**逻辑重复内容**，不输出可释放物理空间（`reclaimable_bytes` 不存在）；
  最多 `total_logical_bytes`。logical / allocated / exclusive physical 是三个不同的数。
* read-only：core 内不存在 unlink / move / trash 能力。
* duplicate 候选**只在内存中**，且是**有界**的：超预算报
  `ResourceLimitExceeded`，不静默 OOM；v0.0.1 不做 spill-to-disk。
* MCP 与安全删除只预留字段，不实现。删除走
  `DuplicateResult → Selection → ActionPlan → Revalidate → Confirmation → Apply`，
  绝不允许 `DuplicateResult → unlink(path)`。

## 变更规则

改这里任何一项都是 contract revision：

1. 更新对应文档与本页表格；
2. 更新类型与常量（`CORE_CONTRACT_VERSION`、`PLAN_VERSION`、
   `RESULT_SCHEMA_VERSION`、`FINGERPRINT_ALGORITHM_VERSION`）；
3. 更新受影响的 fixture 与 release gate 条目。

不接受「先改代码，文档以后补」。

## r1 相对 r0 的修订

| r0 | r1 |
| --- | --- |
| `Snapshot` + `ObservationClass` | `Observation`（含 `RevisionId`）+ `ObservationValidation` |
| 稳定性判定写在 runtime 里比字段 | `Source::validate_observation`，各 Source 决定强度 |
| 状态名 `Stable` | `ObservationStable`（不是 `ContentUnchanged`） |
| `LinkPolicy` / `BoundaryPolicy` | `LinkPolicy` / `ReparsePolicy` / `MountPolicy` |
| `SERIALIZE_READS` | `CONCURRENT_STAT` / `CONCURRENT_READ`（正面能力位） |
| `CandidateBudget` + `SpillPolicy` | `DuplicateBudget`（有界内存，无 spill） |
| `EntryKind::Asset` | 删除（等 PhotoKit 真正接入再回答） |
| 旧错误码表 | 新表：`NotFound` / `PermissionChanged` / `ChangedDuringScan` / `IoError` / `UnsupportedOperation` / `ResourceLimitExceeded` |
| ADR-010 = 架构硬约束 | ADR-010 = benchmark 假设；新增 ADR-011…017 |
