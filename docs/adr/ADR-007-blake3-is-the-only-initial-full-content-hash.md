# ADR-007: BLAKE3 is the only initial full-content hash

- **Status:** Accepted (2026-09-22)
- **冻结补充:** 2026-09-23（PR0）

## Decision

v0.0.1 只支持 BLAKE3 作为 full-content fingerprint，不引入 CRC32 / XXH3 / SHA256 /
SHA512。结果必须携带 algorithm 与 version。

## Consequences

* **same BLAKE3 ≠ 字节级相等**：它是工程级 fingerprint，不是数学证明。
  结果必须带 `VerificationStatus`（`HashMatched` / `ByteExactVerified` /
  `ChangedDuringScan` / `Unreadable` / `Unverified`），只有前两者是 confirmed。
* Partial fingerprint 的 range 参数是**可调策略，不是 API contract**，
  可随 benchmark 改变而不算破坏。
* `FINGERPRINT_ALGORITHM_VERSION` 在喂给 hash 的字节变化时递增；
  跨版本比较 fingerprint 是错误用法。
