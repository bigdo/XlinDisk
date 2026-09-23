# Object identity

ObjectId 回答「是不是同一个底层存储对象」，与「内容是否相同」是两个问题。
它是 hardlink 正确性与未来 cache key 的基础，不是优化项。

## Source-scoped：ObjectId 不能脱离 SourceId 解释

```text
ObjectKey = (SourceId, ObjectId)
```

为了 hot path 不必真的存一个 `ObjectKey` 结构：`Entry` 已经带 `SourceId`，
而 `ObjectId` 自身也携带 `source` / `volume`，两者参与相等比较。

## 生命周期：保守定义

> **ObjectId 只保证在当前 SourceSession / ScanSession 内用于 identity comparison。**

不承诺：

```text
今天 scan 得到的 ObjectId = 下个月还能拿来恢复同一个对象
```

即使 Unix inode 很诱人也不形成这个 API contract。将来 cache 需要持久身份时，另定义：

```text
PersistentObjectIdentity
```

而不是悄悄扩大 `ObjectId` 的语义。

## 缺失策略

| 场景 | 处理 |
| --- | --- |
| 平台无稳定身份 API | `UnsupportedPlatform` |
| 网络盘 / SMB / NFS | `NetworkFileSystem`（跨挂载不可信） |
| FAT / exFAT 等无 inode 概念 | `FileSystemWithoutIdentity` |
| device / socket / fifo | `SpecialFile` |
| metadata 调用失败 | `MetadataFailed`（视为 unknown，不是 unique） |

## ObjectId 缺失不阻止 duplicate detection

必须区分两件事：

```text
content duplicate      → 可以判断
storage reclaim        → 不可以判断
```

ObjectId 不可用时，`size + BLAKE3` 仍然可以报告：

> 这些逻辑对象的内容 fingerprint 相同。

只是不能可靠判断「是否已经通过 hardlink 共用物理数据」。因此：

* **允许**：输出 `DuplicateGroup`；
* **不输出**：`reclaimable_bytes`。

## 即使有 ObjectId，也不输出物理可释放空间

```text
logical size ≠ allocated size ≠ exclusive physical size
```

hardlink、sparse、compression、APFS clone、Btrfs reflink、CoW、dedupe filesystem
都会破坏简单关系。v0.0.1 的 `DuplicateGroup` 最多带 `total_logical_bytes`。

## Hardlink：两个层次的输出

```text
DuplicateGroup
  ├── content: [A, B, C]           # 同一 BLAKE3
  └── object_groups: [[A, B], [C]] # 同一 ObjectId 折叠
```

A、B 共享 ObjectId，C 不共享 ⇒ 内容成员 3 个，存储对象 2 个。
这样 GUI 可以展示 hardlink、cleanup planner 不会误删、benchmark 不丢信息。

v0.0.1 额外输出 `EntryFlags::HARDLINKED`，**不**计算「真正能释放多少块」，
也**不**输出 link count（等出现真实需求再进 contract）。

## 代码

```rust
pub struct ObjectId { pub source: SourceId, pub volume: VolumeId, pub value: u128 }
pub enum ObjectIdUnavailable { UnsupportedPlatform, NetworkFileSystem, FileSystemWithoutIdentity, SpecialFile, MetadataFailed }
```
