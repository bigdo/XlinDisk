# ADR-002: Storage objects are not paths

- **Status:** Accepted (2026-09-22)

## Decision

Core 的公共对象模型**不含** `PathBuf` 或任何 filesystem 语义。
位置由 Source 发放的 opaque `LocatorId` 表示；人类可读路径只能由
`Source::display_locator` 产出，用于展示，不作为身份或输入。

## Consequences

* PhotoKit / MediaStore / SAF adapter 可以在不改 Core 的前提下接入。
* 任何「临时把路径塞进某个 enum 变体」的写法都是违约，包括 spill 目录
  （见 `duplicate::SpillTarget`，由 host 解析位置）。
* Spec、类型定义里的注释必须持续说明这条边界，否则后来者会自然地把路径加回来。
