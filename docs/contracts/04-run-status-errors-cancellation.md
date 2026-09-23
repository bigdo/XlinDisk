# RunStatus / error / cancellation / progress

否则 CLI、MCP 和 GUI 会各自解释「算完了没」。

## RunStatus

| 状态 | 含义 | `is_usable()` | `is_final()` |
| --- | --- | --- | --- |
| `Completed` | Plan 要求的全部范围完成 | 是 | 是 |
| `Partial { errors }` | 有有效结果，但部分范围未完成 | 是 | 是 |
| `Cancelled` | 调用方主动取消 | 否 | **否** |
| `Failed` | 无法形成有意义结果 | 否 | 是 |

`Partial` 的典型成因：内存预算耗尽、过多不可读条目、部分 source 不可达。

`Cancelled` **可以携带部分结果**，但 `is_final()` 为 false：任何人都不得据此得出结论。
取消后的降级路径是 `Partial`（有可用结果）或 `Cancelled`。

## 稳定错误码

判别值就是 wire format。新增可以，重排属于破坏。`message` 只是人类可读信息，
consumer 不得解析它。

```
100 RootNotFound          101 SourceInitFailed     102 ExecutorUnavailable
103 PlanInvalid           104 SourceUnavailable

200 PermissionDenied      201 NotFound             202 PermissionChanged
203 ChangedDuringScan     204 IoError              205 LocatorInvalid
206 LocatorExpired        207 ObjectIdUnavailable   208 UnsupportedOperation
209 ResourceLimitExceeded

300 Cancelled

400 Internal
```

* `1xx` fatal：中止整个 run。
* `2xx` entry-level：记录并继续，绝不因为百万文件中的一个异常终止任务。
* `3xx` 控制流；`4xx` 内部 bug。

## Entry error 的 retention budget

100 万个 permission error 不可能在内存里存成 100 万个 error object：

```text
first N detailed errors  +  aggregated counters
max_detailed_entry_errors = 1024
```

之后只累加计数：

```text
PermissionDenied: 1,923,339
NotFound: 412
```

`ErrorBudget::count_of(code)` / `counts()` 给出全部计数，`retained()` 给出明细。
这也正是未来 MCP 需要的形式。

## Cancellation

`CancellationToken`（`Arc<AtomicBool>`，可 clone 到 worker）：

* 协作式取消，Source 与 Runtime 在合理粒度检查；
* 不用于中断 syscall 中的线程；
* 取消后仍要发出**终止事件**，让 consumer 知道流结束了而不是静默。

## Progress

```rust
ProgressEvent { run_id, sequence, stage, counters, elapsed, terminal }
ProgressCounters { entries_seen, bytes_read, candidates }
```

* channel 是 bounded / lossy：`sequence` 有空洞是设计使然，consumer 由此发现中间事件
  被丢弃；慢 UI 或停止读取的 MCP client 不允许阻塞 scanner；
* **final event 不可丢**：`terminal: Option<TerminalEvent>` 只在最后一个事件出现，
  是判断结束的唯一依据；
* Core 只给结构化数据，不产出面向用户的字符串。
