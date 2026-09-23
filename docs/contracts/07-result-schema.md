# Result schema & versions

v0.0.1 不实现 MCP，也不实现删除，但结果字段现在就留全。以后接入只是适配，不是改模型。

## Record

| 字段 | 为什么现在就要有 |
| --- | --- |
| `scan_id` / `session_id` | 结果可追溯到一次具体观察 |
| `observed_at` | 观察时间戳：结果有时效性 |
| `source` | source scope：跨库 / 跨卷不能混谈 |
| `locator` | 未来重新定位、重新验证对象的唯一入口 |
| `object` | hardlink 与未来 cache key |
| `logical_size` / `modified` | 证据的一部分 |
| `fingerprint` | 内容身份 |
| `fingerprint_algorithm` / `fingerprint_algorithm_version` | 没有它，结果不可解释 |
| `verification` | 结论强度：`HashMatched` ≠ 字节相等 |

## 版本

```
CORE_CONTRACT_VERSION            = 1
PLAN_VERSION                     = 1
FINGERPRINT_ALGORITHM            = "blake3"
FINGERPRINT_ALGORITHM_VERSION    = 1
```

`FINGERPRINT_ALGORITHM_VERSION` 在喂给 hash 的字节发生变化时递增（range policy、
salt 等）。跨版本比较 fingerprint 是错误用法。

## 未来安全删除

* 删除不在 core：**独立 Action / Validation 层**负责。
* 执行前必须重新验证对象（re-stat + full hash，必要时 byte-exact），
  **不能**依据路径或 digest 直接删除。
* MCP 层负责路径 allowlist、分页、任务取消与显式确认。

## Partial 结果

`RunStatus::Partial` 的结果必须带同样的 schema，只是覆盖范围不完整；
consumer 不得把 partial 当成 complete 展示。
