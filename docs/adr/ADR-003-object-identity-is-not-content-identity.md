# ADR-003: Object identity != content identity

- **Status:** Accepted (2026-09-22)
- **冻结补充:** 2026-09-23（PR0）

## Decision

`ObjectId`（是不是同一个底层对象）与 `Fingerprint`（内容是否相同）是两个类型、
两个问题，从第一版就分开。

## Consequences

* hardlink 的正确性依赖 ObjectId：同一 ObjectId 的多个路径在重复组内只去重一次。
* ObjectId 只在单次 scan session 内有效，作用域为 `(SourceId, VolumeId)`（ADR-012）。
* ObjectId 不可用时（网络盘 / FAT / 特殊文件 / 平台不支持）必须显式记录原因，
  并**禁止推导可释放空间**（ADR-013）。详见 `docs/contracts/01-object-identity.md`。
