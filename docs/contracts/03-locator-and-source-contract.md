# Locator & Source contract

Core 不持有 `PathBuf` 是对的，但 CLI 最终要显示路径、未来安全删除要能**重新定位并
重新验证**对象。因此「显示」和「重开」必须是 Source 的能力，不是 Core 的模型。

## Locator

| 问题 | 冻结结论 |
| --- | --- |
| 表示 | `LocatorId(u64)`，opaque，由 Source 发放 |
| 归属 | 属于**发放它的 Source + 那次 session** |
| 跨线程 | `Copy`，可在 worker 之间传递 |
| 跨阶段 | 允许：scan 之后 fingerprint 阶段可重开 |
| 用错 Source / session | `LocatorInvalid` (203) |
| session 结束后重开 | `LocatorExpired` (204) |
| 人类可读路径 | 只能经 `display_locator` 获得 |

`display_locator` 的结果**只读**：用于 CLI / UI / 日志，不许回头当作身份或输入。

## Source trait

```rust
fn id(&self) -> SourceId;
fn capabilities(&self) -> SourceCapabilities;
fn scan(&self, request: &ScanRequest, sink: &mut dyn EntrySink, cancel: &CancellationToken) -> Result<ScanOutcome, Error>;
fn open_content(&self, locator: LocatorId, request: ContentRequest) -> Result<Box<dyn ContentReader + Send + '_>, Error>;
fn stat(&self, locator: LocatorId) -> Result<Snapshot, Error>;          // revalidate
fn display_locator(&self, locator: LocatorId) -> Result<String, Error>;
```

`Source: Send + Sync`。并发读取默认允许；Source 若不保证并发安全，必须声明
`SourceCapabilities::SERIALIZE_READS`，由 Runtime 串行化（PR0 新增的能力位）。

## ContentReader

fingerprint 消费 Source 提供的 `ContentReader`，不接受 `std::fs::File`。
否则同一条 pipeline 以后无法读 PhotoKit resource 或 SAF document。

## EntryBatch

* 扫描输出 **EntryBatch**，不是一个文件一次跨线程事件。
* `EntrySink::push` 必须容忍任意顺序、任意 worker 到达：确定性在**输出时**施加，
  不在扫描时施加。

## 请求里的策略（不再是文档里的名词）

* `LinkPolicy::DoNotFollow`（默认）— 不穿越链接，标记 `LINK`；
  `FollowWithinRoots` — 只跟随目标仍在 roots 内的链接，环只报告不重复遍历。
* `BoundaryPolicy::StayWithinRoots`（默认）— 遇挂载点停止并标记 `MOUNT_BOUNDARY`；
  `Cross` 需显式 opt-in。

两个 policy 都进 `ScanRequest`，因此不同 policy 是**可见的不同运行**，结果可比。
