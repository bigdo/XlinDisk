# ADR-015: Duplicate detection is memory-bounded, not memory-constant

- **Status:** Accepted (2026-09-23, r1)

## Context

「流式 scanner」容易让人误以为 duplicate 查询是 O(1) 内存。它不是：候选元数据必须
保留到最后阶段。500 万个同尺寸文件就是活生生的反例。

## Decision

* v0.0.1 使用 **explicit bounded in-memory candidate store**；
* `DuplicateBudget { max_candidate_bytes, max_candidates, max_group_members, re_read_instead_of_cache }`；
* 超预算 → `ResourceLimitExceeded` (209)，run 降级为 `RunStatus::Partial`，不静默 OOM；
* `Candidate` 是 compact record（entry / locator / object / logical_size / observation），
  不是 `Entry` 副本，不复制 path、display name、完整 metadata；
* 每阶段结束尽早释放不再需要的候选；
* **v0.0.1 不做 spill-to-disk**，也不为 spill 先做 trait：等真实第二实现再抽象。

## Consequences

* 「memory-bounded」是 contract 的一部分，必须进 benchmark 与 release gate。
* 未来可以有 `DiskBackedCandidateStore`，但那是另一份 ADR。
