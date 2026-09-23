# Observation consistency

文件会在扫描和 hash 期间被修改。没有观察一致性，就会出现「扫描时看起来重复、实际内容
不同」的结论——对只读引擎是错误结果，对未来删除能力是数据丢失。

## Observation 是 Core 的一等模型

不是把这段逻辑散落在 duplicate 实现里，而是进入 Core Model：

```rust
pub struct Observation {
    pub source: SourceId,
    pub locator: LocatorId,
    pub object: Option<ObjectId>,
    pub logical_size: Option<u64>,
    pub modified: Option<Timestamp>,
    pub revision: Option<RevisionId>,
}
```

`revision` 是 **Source-specific** 的版本信息：filesystem v0.0.1 通常只有
object id / size / mtime；未来某个平台有 generation、change counter、version token
就填这里。没有就不填，而不是假装有。

## 标准过程

```text
stat → Observation A
        ↓
   open / read / hash
        ↓
stat → Observation B
        ↓
validate(A, B)
```

结果（`ObservationValidation`）：

| 状态 | 含义 |
| --- | --- |
| `ObservationStable` | 可观察的身份、大小、时间戳、revision 一致 |
| `ChangedDuringScan` | 两次观察之间可观察的东西变了 |
| `Gone` | 对象消失：删除、改名、移出范围 |
| `PermissionChanged` | 对象还在，但已无法读取（权限 / ACL 变化） |
| `Unreadable` | 内容读不出来 |

**只有 `ObservationStable` 能进入 confirmed result。**

## 稳定性判定属于 Source，不属于 Runtime

Runtime 不写死：

```rust
before.object_id == after.object_id && before.size == after.size && before.mtime == after.mtime
```

而是：

```rust
source.validate_observation(locator, &before, &after)
```

原因：不同 Source 的强度不同。filesystem 用 object id + size + mtime；PhotoKit 用
asset id + resource version；别的 provider 有自己的 token。Core 只定义
`Observation` 与 `ObservationValidation`，判断交给 Source。

filesystem 的默认实现是 `Observation::strict_compare`（保守版本），但它是一个
**可选默认**，不是 Core 的语义。

## mtime 不是内容版本号

这一点写进 contract：

> v0.0.1 filesystem source 使用当前平台可获得的 identity / size / timestamp 等信息，
> 尽力判断读取前后的对象是否保持一致。

而不是声称：

```text
same mtime ⇒ same content
```

因此状态名是 `ObservationStable`，不是 `ContentUnchanged`。这个区别以后很重要：它决定了
「稳定」是工程判断而不是内容证明。

## 判定规则（filesystem 默认）

* 只在**两侧都已知**时才比较：缺失字段不能证明变化（避免误报）。
* `object` / `logical_size` / `modified` / `revision` 任一已知且不一致 → `ChangedDuringScan`。
* 对象消失 → `Gone`：从结果移除，记 `NotFound`，不中止扫描。

## 稳定错误码

| 情况 | 错误码 |
| --- | --- |
| 文件消失 / 改名 / 移出范围 | `NotFound` (201) |
| 权限变化 | `PermissionChanged` (202) |
| 重新观察不一致 | `ChangedDuringScan` (203) |
| 读取失败 | `IoError` (204) |
| 无法读取（其他原因） | `PermissionDenied` (200) |

全部是 entry-level：记录后继续扫描。

## 与 VerificationStatus 的关系

| ObservationValidation | VerificationStatus | confirmed? |
| --- | --- | --- |
| `ObservationStable` | `HashMatched` | 是 |
| `ChangedDuringScan` | `ChangedDuringScan` | 否 |
| `Gone` | `ChangedDuringScan` | 否 |
| `PermissionChanged` | `Unreadable` | 否 |
| `Unreadable` | `Unreadable` | 否 |

**`HashMatched` 的准确含义**是：

> same logical size and same full BLAKE3 fingerprint under stable observations

即工程级确认。**不是** byte identical。只有 `ByteExactVerified` 表示执行了逐字节验证。

不能进入 confirmed group 的候选以 `ExcludedCandidate` 挂在 group / run diagnostics 上，
不允许混进 `DuplicateGroup`：一个组里同时有「确认重复」和「不确定」，API 就模糊了。
