# RunStatus / error / cancellation / progress

否则 CLI、MCP 和 GUI 会各自解释「算完了没」。

## RunStatus

| 状态 | 含义 | 有可用结果 |
| --- | --- | --- |
| `Completed` | 完整跑完 | 是 |
| `Partial { errors }` | 提前结束但结果可用（取消、预算耗尽） | 是 |
| `Cancelled` | 取消且未产出可用结果 | 否 |
| `Failed` | 致命错误 | 否 |

**取消后返回部分结果**：明确规定。取消 → `Partial`（若有结果）或 `Cancelled`。

## 错误码

稳定判别值就是 wire format：新增可以，重排属于破坏。

```
100 RootNotFound          101 SourceInitFailed     102 ExecutorUnavailable  103 PlanInvalid
200 PermissionDenied      201 GoneDuringScan       202 ReadFailed
203 LocatorInvalid        204 LocatorExpired       205 ObjectIdUnavailable
206 UnsupportedEntry      207 BudgetExceeded
300 Cancelled
400 Internal
```

* `1xx` fatal：中止整个 run。
* `2xx` entry-level：记录并继续，绝不因为百万文件中的一个异常终止任务。
* `3xx` 控制流；`4xx` 内部 bug。

## Entry-level error 上限

`ErrorBudget { max_retained = 1000 }`：

* `total()` 记录全部计数；
* 超过上限只保留前 N 条明细，其余以计数汇总（`dropped()`）。
  一个无权限的网络目录能产生百万条错误，全留会直接吃掉内存预算。

## Cancellation

`CancellationToken`（`Arc<AtomicBool>`，可 clone 到 worker）：

* 协作式取消，Source 与 Runtime 在合理粒度检查；
* 不用于中断 syscall 中的线程；
* 取消后仍要发出**终止事件**，让 consumer 知道流结束了而不是静默。

## Progress

```rust
ProgressEvent { run_id, sequence, stage, entries_seen, bytes_read, candidates, elapsed, terminal }
```

* channel 是 bounded / lossy：`sequence` 有空洞是设计使然，
  慢 UI 或停止读取的 MCP client 不允许阻塞 scanner；
* `terminal: Option<TerminalEvent>` 只在最后一个事件出现，是判断结束的唯一依据；
* Core 只给结构化数据，不产出面向用户的字符串。
