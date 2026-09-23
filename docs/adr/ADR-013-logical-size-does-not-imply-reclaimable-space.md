# ADR-013: Logical size does not imply physical reclaimable space

- **Status:** Accepted (2026-09-23, r1)

## Decision

v0.0.1 的 duplicate 结果只报告**逻辑重复内容**，最多带 `total_logical_bytes`。
结果模型里**没有** `reclaimable_bytes` / `space_can_be_freed`。

## Why

```text
logical size ≠ allocated size ≠ exclusive physical size
```

hardlink、sparse file、compression、APFS clone、Btrfs reflink、CoW、
dedupe filesystem 都会破坏这个简单关系。

## Consequences

* 即使 ObjectId 可用，也不输出物理可释放空间。
* ObjectId 不可用时，`size + BLAKE3` 仍然可以报告「内容 fingerprint 相同」，
  只是不能判断是否已通过 hardlink 共用物理数据。
* hardlink 用两层结构表达（content + object_groups），信息不丢，但不下结论。
* 这是一个**产品语义边界**，不是暂时没实现：写错比不写更糟。
