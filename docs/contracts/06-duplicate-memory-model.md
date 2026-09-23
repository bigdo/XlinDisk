# Duplicate memory model

「流式 scanner + 不建完整树」**不**意味着 duplicate 查询低内存：重复检测天然要
保留候选集直到最后一阶段。这一点必须进 benchmark 与内存预算，而不是等 OOM 才发现。

## Pipeline

```
Group(Size) → Filter(count >= 2) → PartialFingerprint → Group → FullFingerprint → Group
```

`DuplicateStage::ORDER` 冻结为 `[Size, PartialFingerprint, FullFingerprint]`。

## 中间状态放哪（无 persistent cache）

| 问题 | 冻结结论 |
| --- | --- |
| 是否保留所有候选 Entry | 是，但受 `CandidateBudget.max_candidates` 约束 |
| 超大 size group | 受 `max_group_members` 约束，超限不整体 hash |
| 是否允许 spill | 由 `SpillPolicy` 决定；默认 `Forbid`（超限是错误，不是变慢） |
| 能否重开 locator 二次读取 | 可以，`re_read_instead_of_cache` 默认 true（本地便宜，网络/云昂贵） |
| 超出预算 | `BudgetExceeded` (207)，结果降级为 `Partial`，不静默截断 |

## Spill 为什么不写路径

`SpillTarget::{SystemTemp, Named}` —— core 没有 path 概念，具体位置由 host 解析。
把 `PathBuf` 放进 `SpillPolicy` 会把 filesystem 语义重新拖回 core，等于推翻 ADR-002。

## Benchmark 义务（PR8）

* 记录 duplicate 中间态 peak RSS / bytes per retained entry；
* 必须覆盖：zero / sparse / dense duplicates、large duplicate files、
  many small duplicate files；
* 必须验证候选上限与取消策略生效。

## 输出范围

v0.0.1 只输出**逻辑重复文件**，不输出「可释放物理空间」。
