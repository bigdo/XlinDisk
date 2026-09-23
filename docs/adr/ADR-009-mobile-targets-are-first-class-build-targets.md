# ADR-009: Mobile targets are first-class build targets

- **Status:** Accepted (2026-09-22)

## Decision

`aarch64-apple-ios` 与 `aarch64-linux-android` 从第一版就是 Core 的 compile smoke
目标。目的不是 v0.0.1 在手机上有功能，而是**防止 desktop-only 依赖渗进 core**。

## Consequences

* iOS / Android adapter 不在 v0.0.1 范围，但模型必须能在没有 path 的情况下工作。
* 任何只在 desktop 可用的依赖只能出现在 `xlindisk-source-fs` 或 App crate。
* mutation 最终走平台机制（PhotoKit change request、MediaStore trash/delete），
  不由 Core 直接 unlink。
