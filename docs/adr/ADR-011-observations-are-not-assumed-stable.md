# ADR-011: Observations are not assumed stable across I/O

- **Status:** Accepted (2026-09-23, r1)

## Context

任何跨越 I/O 的操作都面对 TOCTOU：扫描时看到的对象，在 hash 时可能已经不同。
把它当作某条 pipeline 的实现细节，等于让每个后续能力（cache、MCP、delete、
PhotoKit）各自重新发明一遍。

## Decision

Core 引入一等模型：

```text
Entry → Observation → Operation → Re-observation → Validation → Result
```

* `Observation`：source、locator、object、logical_size、modified、revision；
* 操作前后各取一次观察；
* `Source::validate_observation` 判定二者是否一致；
* 只有 `ObservationStable` 能进入 confirmed result；
* 其余进 `ExcludedCandidate`，不混入 `DuplicateGroup`。

## Consequences

* 状态名是 `ObservationStable`，不是 `ContentUnchanged`：这是工程判断，不是内容证明。
* mtime 不是内容版本号；`revision` 是留给平台更强信号的扩展位。
* 稳定性强度由 Source 决定，Core 不硬编码比较逻辑。
* 未来安全删除直接复用这套模型做 TOCTOU 防护。
