# Locator & Source contract

Core 不持有 `PathBuf` 是对的，但 CLI 最终要显示路径、未来安全删除要能**重新定位并
重新验证**对象。因此「显示」「排序」「重开」必须是 Source 的能力，不是 Core 的模型。

## Locator 是什么

> Locator 是 SourceSession 范围内的 **opaque, immutable handle**。

```text
LocatorId = 一个小整数（Copy + Send + Sync）
真实 path / URI / PHAsset identifier = 由 Source 自己管理
```

| 问题 | 冻结结论 |
| --- | --- |
| 表示 | `LocatorId(u64)`，opaque，由 Source 发放 |
| 归属 | 属于**发放它的 Source + 那次 session** |
| 生命周期 | **仅在 session 内有效**。`scan session A: LocatorId(35)` 与 `scan session B: LocatorId(35)` 完全无关 |
| 跨线程 | `Copy + Send + Sync`，可在 worker 之间传递 |
| 跨阶段 | 允许：scan 之后 fingerprint 阶段可重开 |
| 用错 Source / session | `LocatorInvalid` (205) |
| session 结束后重开 | `LocatorExpired` (206) |
| 持久化给 MCP / 数据库 | **不属于 v0.0.1**；将来另定义 `ExternalLocator` |

不承诺「今天 scan 得到的 Locator / ObjectId，下个月还能拿来恢复同一个对象」。
即使 Unix inode 很诱人也不形成这个 API contract。

## display_locator 是懒执行的

不要在 scanner 发现每个文件时就构造完整 absolute path `String`——否则前面避免
`PathBuf` allocation 的努力又被抵消。

```text
LocatorId 一直走 hot path
        ↓
只有最终要展示的结果（100 个重复、100 个最大文件）
        ↓
display_locator(locator) 才 materialize
```

这条对 CLI、MCP、GUI 都成立。

**display locator 绝不能反过来作为对象 identity。** CLI 可以显示
`/home/a.txt`，但 Core 不能因为这个字符串相同就认为对象相同。

## Source trait

```rust
fn id(&self) -> SourceId;
fn capabilities(&self) -> SourceCapabilities;
fn scan(&self, request: &ScanRequest, sink: &mut dyn EntrySink, cancel: &CancellationToken) -> Result<ScanOutcome, Error>;
fn stat(&self, locator: LocatorId) -> Result<Observation, Error>;
fn validate_observation(&self, locator: LocatorId, before: &Observation, after: &Observation) -> Result<ObservationValidation, Error>;
fn open_content(&self, locator: LocatorId, request: ContentRequest) -> Result<Box<dyn ContentReader + Send + '_>, Error>;
fn display_locator(&self, locator: LocatorId) -> Result<String, Error>;
fn sort_key(&self, locator: LocatorId) -> Result<Vec<u8>, Error>;
```

## 并发是 Capability，不是假设

Runtime 不得假定「所有 Source 都线程安全」：

```text
CONCURRENT_STAT
CONCURRENT_READ
```

Filesystem 两个都声明；未来某些移动 provider 可能只能串行。Planner / Scheduler 据此
决定并行度，而不是写死。

## EntryBatch

* 扫描输出 **EntryBatch**，不是一个文件一次跨线程事件。
* `EntrySink::push` 必须容忍任意顺序、任意 worker 到达：确定性在**输出时**施加，
  不在扫描时施加。

## ContentReader

fingerprint 消费 Source 提供的 `ContentReader`，不接受 `std::fs::File`。
否则同一条 pipeline 以后无法读 PhotoKit resource 或 SAF document。

## 请求里的策略（不再是文档里的名词）

`ScanRequest` 携带 `link_policy` / `reparse_policy` / `mount_policy`，因此不同 policy
是**可见的不同运行**，结果可比。具体语义见
[spec/filesystem-semantics.md](../spec/filesystem-semantics.md)。
