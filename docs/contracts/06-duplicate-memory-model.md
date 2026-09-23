# Duplicate resource model

「流式 scanner + 不建完整树」**不**意味着 duplicate 查询低内存。这是错误直觉：
重复检测至少要保留候选元数据直到最后一个阶段。

## 结论先行

> **duplicate execution is memory-bounded, not memory-constant.**

明确说清楚这一点，比假装它是 O(1) 有用：达到预算就报 `ResourceLimitExceeded`，
不偷偷 OOM。

## v0.0.1 不做 spill-to-disk

spill 会立刻引入：临时数据库、序列化、崩溃清理、磁盘配额、临时目录策略、resume 语义。
复杂度远超收益。v0.0.1 的选择是：

> **Explicit bounded in-memory candidate store.**

以后可以演进为：

```text
InMemoryCandidateStore
DiskBackedCandidateStore
```

但**现在不为它先做 trait**——等真实第二实现再抽象（与 `ComputeBackend` 同一条原则）。

## Budget

```rust
pub struct DuplicateBudget {
    pub max_candidate_bytes: Option<u64>,
    pub max_candidates: Option<u64>,
    pub max_group_members: Option<u32>,
    pub re_read_instead_of_cache: bool,
}
```

* 超预算 → `ResourceLimitExceeded` (209)，run 降级为 `RunStatus::Partial`。
* 不静默截断，不静默变慢。

## Candidate 是 compact record，不是 Entry 副本

```rust
pub struct Candidate {
    pub entry: EntryId,
    pub locator: LocatorId,
    pub object: Option<ObjectId>,
    pub logical_size: Option<u64>,
    pub observation: Observation,
}
```

绝不复制：完整 path、display name、完整 metadata object。这样才能做到百万级候选。
`Candidate` 有大小守卫测试（预算 256 字节）。

## 分阶段释放

```text
size groups
      ↓ singleton drop
partial hash
      ↓ non-candidate drop
full hash
      ↓
result
```

每一阶段之后，不再需要的 candidate 必须尽早释放。不允许 stage1 / stage2 / stage3 的
数据全程同时存在。

## 极端 large size group

例如 500 万个 4 KiB 文件全部同尺寸：size grouping 本身无法有效 prune。此时
`max_group_members` 是唯一防线，超限的组标记 `GroupTooLarge` 进 diagnostics。

## 输出结构：ContentGroup + ObjectGroups

```text
DuplicateGroup
  ├── content: [A, B, C]          # 同一 fingerprint
  └── object_groups: [[A, B], [C]]  # 同一 ObjectId 的硬链接折叠
```

A、B 共享 `ObjectId`，C 不共享 ⇒ 一个内容组三个成员、两个存储对象。
GUI 可以展示 hardlink，cleanup planner 不会误删，benchmark 不丢信息。

## 没有 reclaimable_bytes

`logical size ≠ allocated size ≠ exclusive physical size`。hardlink、sparse、
compression、APFS clone、Btrfs reflink、CoW、dedupe filesystem 都会破坏简单关系。
因此结果里最多有 `total_logical_bytes`，**没有** `space_can_be_freed`。

## Benchmark 义务（PR8）

* 记录 duplicate 中间态 peak RSS / bytes per retained entry；
* 覆盖 zero / sparse / dense duplicates、large duplicate files、
  many small duplicate files；
* 验证候选上限、组上限与取消策略生效。
