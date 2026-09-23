# ADR-014: Locators are opaque, source-owned, session-scoped handles

- **Status:** Accepted (2026-09-23, r1)

## Decision

`LocatorId` 是一个 opaque、immutable、`Copy + Send + Sync` 的小整数：

* 由 Source 发放，归属该 Source 与该 session；
* 仅在 session 生命周期内有效；
* Core 从不解释其含义；
* 人类可读路径只能经 `display_locator` 获得，且**懒执行**（只对最终要展示的结果调用）；
* 排序用 `sort_key`，与显示、身份三者互相独立。

## Consequences

* scanner 不为每个文件构造 absolute path，避免把省下的 allocation 又还回去。
* display locator 绝不能反向作为 identity。
* 持久化给 MCP / 数据库不属于 v0.0.1；将来另定义 `ExternalLocator`，
  而不是现在就把 path / URI 字符串塞进 Core。
* 重新定位与重新验证（`stat`）成为未来安全删除的基础能力。
