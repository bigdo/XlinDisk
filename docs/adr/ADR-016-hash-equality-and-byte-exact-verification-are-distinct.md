# ADR-016: Hash equality and byte-exact verification are distinct states

- **Status:** Accepted (2026-09-23, r1)

## Decision

`VerificationStatus` 明确区分：

| 状态 | 含义 |
| --- | --- |
| `HashMatched` | same logical size + same full BLAKE3 fingerprint **under stable observations** |
| `ByteExactVerified` | 另外做了逐字节比较 |
| `ChangedDuringScan` | 两次观察不一致 |
| `Unreadable` | 内容读不出来 |
| `Unverified` | 未验证（例如中途取消） |

只有前两者可以进入 confirmed duplicate group。

## Consequences

* 不允许把 `HashMatched` 说成 byte identical：它是工程级确认。
* 未来 destructive action 的默认安全策略至少是 re-stat + full hash，
  必要时再 byte-exact。
* `ByteExactVerified` 也不宣称「数学上绝对」——从软件 contract 看，两者的区别已经足够
  让调用方知道自己拿到的是什么。
