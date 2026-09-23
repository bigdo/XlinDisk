# XlinDisk Core · Architecture

完整的方案与 review 记录在 Notion：**Architecture｜XlinDisk-core v0.0.1 实施方案**。
本文件是仓库内的可执行摘要，只保留影响代码结构的内容。

## 1｜定位

XlinDisk 的长期核心不是某个去重 / 清理 / 可视化 App，而是一个高性能、跨平台、
可被多种前端复用的 **storage analysis engine**。

> Applications describe WHAT they want. XlinDisk Core decides HOW to execute it.

* Core：Model、Plan、Runtime、Source contract，以及与执行性能直接相关的能力。
* Desktop / CLI / MCP / Mobile / 第三方 App：前端，不拥有扫描、分组、hash 算法。

v0.0.1 是 **read-only engine**：优先验证 correctness、architecture、memory、
performance、portability，不让 mutation safety 干扰核心架构。

## 2｜分层

| 层 | 内容 | 代码 |
| --- | --- | --- |
| MODEL | SourceId / EntryId / LocatorId / ObjectId、Entry、Snapshot、Fingerprint、Record、EntryStore | `src/model/` |
| PLAN | Scan / Filter / Group / Aggregate / Sort / Fingerprint / Limit | `src/plan.rs` |
| RUNTIME | RunStatus、Progress、Cancellation、ErrorBudget | `src/runtime.rs`、`src/error.rs` |
| SOURCE | Source contract（唯一知道 path / URI / PhotoKit 的地方） | `src/source.rs` |

关键分离：**对象身份（ObjectId）≠ 位置（LocatorId）≠ 内容（Fingerprint）**。

## 3｜Workspace 与依赖方向

```
crates/xlindisk-core        依赖：bitflags
crates/xlindisk-source-fs   依赖：xlindisk-core
crates/xlindisk-cli         依赖：xlindisk-core
crates/xlindisk-bench       依赖：xlindisk-core
```

单向依赖，由 review 与 CI 保证：

* core 不依赖任何 App，也不依赖 source-fs；
* source-fs → core；
* CLI / bench → core（+ 具体 source）。

`xlindisk-source-fs` 从第一版就独立，是因为 PhotoKit / MediaStore 不提供
「可直接打开的路径」语义；core 一旦把 `PathBuf` 写进公共模型，移动端接入就是结构性重构。

## 4｜当前状态

* **PR0（Contract Freeze）**：语义契约已冻结在 `docs/contracts/`，类型已落地在
  `crates/xlindisk-core`。**PR0 未合并不开工 PR3。**
* **PR1（skeleton）**：workspace、四个 crate、ADR-001…010、本文档。
* **PR2（Core Model）**：`EntryStore` —— 连续内存 + dense id，hierarchy 按需
  materialize，snapshot 按候选记录（`Entry` 预算 ≤ 128 字节，有测试守住），
  hardlink 分组只在候选范围内做。

Core 不含 executor / scheduler / walker：它们在 PR5 之后进入，且不得改变已冻结的类型。

### 内存原则的执行方式

`Entry` 里**没有** Snapshot：一个 `Snapshot` 是 112 字节，放进去会让 `Entry` 从
~120 涨到 240 字节（1000 万条目就是 2.4 GB）。snapshot 由 `EntryStore` 按候选
单独记录，只有真正要 hash 的条目才付这份钱。`model::store` 里的
`entry_stays_small_enough_for_tens_of_millions` 会在 `Entry` 超出预算时直接让测试失败。

## 5｜PR 顺序

| PR | 内容 | 状态 |
| --- | --- | --- |
| PR0 | Contract Freeze（语义 + 类型 + fixture 清单） | ✅ 本分支 |
| PR1 | Architecture skeleton（workspace、crates、ADR） | ✅ 本分支 |
| PR2 | Core Model（`EntryStore`、snapshot 侧表、hardlink 分组） | ✅ 分支 `feat/pr2-core-model` |
| PR3 | Reference Filesystem Source（单线程 oracle） | 阻塞于 PR0 |
| PR4 | Parallel Filesystem Source | 阻塞于 PR3 |
| PR5 | Plan + Executor | 待办 |
| PR6 | Fingerprinting（含观察一致性） | 待办 |
| PR7 | Duplicate Plan preset（含内存模型） | 待办 |
| PR8 | Benchmark + CI Release Gate | 待办 |

Release gate 清单见 Notion 第 28 节；本仓库在 PR8 同步为可执行的 CI 检查。

## 6｜API 与实现解耦

* `crossbeam-deque`、`crossbeam-channel`、单 scheduler 是**实现细节**，不进公共 API。
* **ADR-010 已从架构硬约束降级为 benchmark 假设**：Source 抽象用泛型、trait object
  还是 enum，由 benchmark 决定。当前骨架用 trait object（`Box<dyn ContentReader>`），
  出现在**内容读取**边界，而不是每 entry 的 hot path。
* 第三方依赖取舍见 ADR 与 Notion 第 22 节：`thiserror` 等实现性依赖在进入对应 PR 时
  才引入，当前错误模型只有稳定码，不需要 proc-macro。

## 7｜参考

* Czkawka / Krokiet、dupeGuru、jdupes、rmlint、rdfind：功能与 pipeline 参考。
* dua-core、eDirStat：walker、work stealing、compact arena 参考。
* fclones：storage-aware scheduler 与 ObjectId-as-cache-key 参考。
* fselect：查询化 filesystem analysis 的证明（但 SQL 属于 frontend）。
* Spacedrive：read-only Query 与 Action / validation 分离的参考。
