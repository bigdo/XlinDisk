# Observation consistency

文件会在扫描和 hash 期间被修改。没有观察一致性，就会出现「扫描时看起来重复，
实际内容不同」的结论——对只读引擎是错误结果，对未来删除能力是数据丢失。

## 流程

1. **scan 阶段**取 metadata snapshot：`logical_size`、`modified`、`object`、`generation`。
2. **fingerprint 阶段**先 `stat`（revalidate）再读内容，hash 后再 `stat` 一次。
3. 任一已知字段不一致 → `ChangedDuringScan`。
4. `ChangedDuringScan` 与 `Unreadable` 的文件**不得进入 confirmed duplicate**。

## 判定规则

* 只在**两侧都已知**时才比较：缺失字段不能证明变化（避免误报）。
* `object`（ObjectId）变化 → changed。
* `logical_size` 变化 → changed。
* `modified` 变化 → changed。
* `generation`（Windows change time / 平台 generation）变化 → changed。
* 对象消失 → `Gone`：从结果中移除，记 `GoneDuringScan`，不中止扫描。

## 稳定错误码

| 情况 | 错误码 |
| --- | --- |
| 文件消失 / 被改名 / 移出范围 | `GoneDuringScan` (201) |
| 权限变化 | `PermissionDenied` (200) |
| 读取失败 | `ReadFailed` (202) |

三者都是 entry-level：记录后继续扫描。

## 与 VerificationStatus 的关系

| ObservationClass | VerificationStatus | confirmed? |
| --- | --- | --- |
| `Stable` | `HashMatched` | 是 |
| `ChangedDuringScan` | `ChangedDuringScan` | 否 |
| `Gone` | `ChangedDuringScan` | 否 |
| `Unreadable` | `Unreadable` | 否 |

**BLAKE3 相同 ≠ 字节相等。** `HashMatched` 是工程级 fingerprint 结论，
不是数学证明。需要更强保证时走 `VerificationMode::ByteExact`，产出
`ByteExactVerified`。未来 destructive action 的默认安全策略至少是
re-stat + full hash，必要时再 byte-exact。
