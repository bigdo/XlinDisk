# Result schema & versions

v0.0.1 不实现 MCP，也不实现删除，但结果字段现在就留全。以后接入只是适配，不是改模型。

这些字段是 **analysis provenance**，不是为了现在实现 MCP。

## Record

```rust
pub struct Record {
    pub run_id: RunId,
    pub scan_id: ScanId,
    pub session_id: SessionId,
    pub observed_at: Option<Timestamp>,
    pub source: SourceId,
    pub locator: LocatorId,
    pub object: Option<ObjectId>,
    pub logical_size: Option<u64>,
    pub modified: Option<Timestamp>,
    pub fingerprint: Option<Fingerprint>,
    pub fingerprint_algorithm: &'static str,
    pub fingerprint_algorithm_version: u32,
    pub verification: VerificationStatus,
}
```

| 字段 | 为什么现在就要有 |
| --- | --- |
| `run_id` | 一次执行的结果必须可追溯到具体那次运行 |
| `scan_id` / `session_id` | locator 与 ObjectId 的语义依赖 session |
| `observed_at` | 观察时间戳：结果有时效性 |
| `source` | source scope：跨库 / 跨卷不能混谈 |
| `locator` | 未来重新定位、重新验证对象的唯一入口 |
| `object` | hardlink 与未来 cache key |
| `logical_size` / `modified` | 证据的一部分 |
| `fingerprint` | 内容身份 |
| `fingerprint_algorithm` / `_version` | 没有它，结果不可解释 |
| `verification` | 结论强度：`HashMatched` ≠ 字节相等 |

## 版本

```
CORE_CONTRACT_VERSION            = 2
PLAN_VERSION                     = 1
RESULT_SCHEMA_VERSION            = 1
FINGERPRINT_ALGORITHM            = "blake3"
FINGERPRINT_ALGORITHM_VERSION    = 1
```

* consumer 读 `Record` 前必须检查 `RESULT_SCHEMA_VERSION`；
* `FINGERPRINT_ALGORITHM_VERSION` 在喂给 hash 的字节发生变化时递增
  （range policy、salt 等）。跨版本比较 fingerprint 是错误用法。

## Partial 结果

`RunStatus::Partial` 的结果必须带同样的 schema，只是覆盖范围不完整；
consumer 不得把 partial 当成 complete 展示。`RunStatus::Cancelled` 的结果
`is_final() == false`。

## 未来安全删除

```text
DuplicateResult → Selection → ActionPlan → Revalidate → Confirmation → Apply
```

绝对不能：

```text
DuplicateResult → unlink(path)
```

执行前必须重新验证对象（re-stat + full hash，必要时 byte-exact）。
`Locator` / `ObjectId` / `Observation` / `Fingerprint` 这些今天定义的模型，
未来正好全部参与 TOCTOU 防护。

MCP 层负责路径 allowlist、分页、任务取消与显式确认；它**不进入 Core**。
