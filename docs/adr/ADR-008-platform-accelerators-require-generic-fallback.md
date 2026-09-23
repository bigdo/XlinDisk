# ADR-008: Platform accelerators require generic fallback

- **Status:** Accepted (2026-09-22)

## Decision

NTFS MFT、APFS、Everything / plocate 之类的加速器只能是 optional fast path：
失败时必须回退 generic source，**正确性优先**。

## Consequences

* v0.0.1 不实现任何平台加速器（属于明确非目标）。
* 引入加速器时，必须有与 ReferenceWalker 一致的规范化结果作为验收标准。
* 第三方库已解决的部分（例如 BLAKE3 的 SIMD runtime dispatch）不自己重写。
