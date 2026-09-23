# ADR-012: Object identity is source-scoped and session-scoped

- **Status:** Accepted (2026-09-23, r1)

## Decision

`ObjectId` 不能脱离 `SourceId` 解释：

```text
ObjectKey = (SourceId, ObjectId)
```

且生命周期是保守的：

> ObjectId 只保证在当前 SourceSession / ScanSession 内用于 identity comparison。

不承诺「今天 scan 得到的 ObjectId，下个月还能拿来恢复同一个对象」。

## Consequences

* `ObjectId` 携带 `source` / `volume`，三者共同参与相等比较。
* 跨 session 比较 ObjectId 是错误用法；用错 session 的 locator 报 `LocatorInvalid` /
  `LocatorExpired`。
* 将来 cache 需要持久身份时，单独定义 `PersistentObjectIdentity`，
  不悄悄扩大 `ObjectId` 的语义。
* ObjectId 缺失（网络盘 / FAT / special file / 平台不支持）时显式记录原因，
  并禁止推导可释放空间（见 ADR-013）。
