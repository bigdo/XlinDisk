# Object identity

ObjectId 回答「是不是同一个底层存储对象」，与「内容是否相同」是两个问题。
它是 hardlink 正确性与未来 cache key 的基础，不是优化项。

## 冻结语义

1. **仅在单次 scan session 内有效。** ObjectId 不作为跨进程 / 跨时间的持久身份，
   不作为缓存主键写进磁盘而不带 session 上下文。
2. **作用域 = `(SourceId, VolumeId)`。** 两个卷上的同一个 inode 号是两个对象，
   因此 `ObjectId` 三个字段都参与相等比较。
3. **取不到就是取不到。** 用 `Option<ObjectId>` + `ObjectIdUnavailable` 原因，
   不退化成「各自唯一」的假身份。

## 缺失策略

| 场景 | 处理 |
| --- | --- |
| 平台无稳定身份 API | `UnsupportedPlatform` |
| 网络盘 / SMB / NFS | `NetworkFileSystem`（跨挂载不可信） |
| FAT / exFAT 等无 inode 概念 | `FileSystemWithoutIdentity` |
| device / socket / fifo | `SpecialFile` |
| metadata 调用失败 | `MetadataFailed`（视为 unknown，不是 unique） |

**ObjectId 不可用时禁止计算可释放空间。** 唯一允许的结论是「这些文件内容相同」，
不允许输出「删掉可以省 N 字节」。

## Hardlink

* 同一 `ObjectId` 的多个路径在重复组内**只去重一次**，不当作 N 份独立存储。
* v0.0.1 额外输出 `EntryFlags::HARDLINKED`，**不**输出 link count。
  link count 是平台能力，等出现真实需求再进 contract。

## 代码

```rust
pub struct ObjectId { pub source: SourceId, pub volume: VolumeId, pub value: u128 }
pub enum ObjectIdUnavailable { UnsupportedPlatform, NetworkFileSystem, FileSystemWithoutIdentity, SpecialFile, MetadataFailed }
```
