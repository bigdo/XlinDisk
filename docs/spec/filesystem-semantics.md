# Filesystem semantics specification

PR0 r1 冻结。这份文档把「symlink policy」这类**名词**变成可执行契约；
每条都在 `ScanRequest` 或 `Source` 上有对应字段 / 方法，也都有 fixture（见
[contracts/08](../contracts/08-cross-platform-fixtures.md)）。

## 1. Symlink

* 默认 `LinkPolicy::DoNotFollow`：symlink 本身作为 `Entry` 返回，标记 `LINK`。
* broken symlink：`EntryKind::Symlink` + `EntryFlags::TARGET_UNAVAILABLE`，
  **不是** scan fatal。
* 显式 `FollowWithinRoots` 时才跟随，且目标必须仍在 roots 内。
* 循环检测基于**目录 identity**（ObjectId / directory identity），不是字符串路径。

## 2. Windows junction / reparse point

* 默认 `ReparsePolicy::DoNotTraverseReparsePoint`。
* 只有 `ScanSpec` 显式要求 `TraverseWithinRoots` 才穿越。
* 标记 `EntryFlags::REPARSE_POINT`。

这是最安全的选择：Windows 上不先做 identity 工作就穿越 reparse point，是目录循环的
主要来源。

## 3. Loop

如果显式 follow：

```text
ancestor cycle detection by ObjectId / directory identity
```

不靠路径字符串比较（那在 hardlink、mount、reparse 组合下必然漏判）。

## 4. Mount boundary

不存在「神秘默认」，`ScanSpec` 必须明确：

```rust
enum MountPolicy {
    StayOnInitialFilesystem,   // 默认；遇挂载点停止并标记 MOUNT_BOUNDARY
    CrossFilesystems,          // 显式 opt-in
}
```

各个官方 preset 决定自己的默认值；Core 本身不替调用方猜。

## 5. Special files

Unix：socket、fifo、block device、char device。

v0.0.1：

```text
枚举，但不 open_content
```

* `EntryKind::Special` + subtype flags（`SOCKET` / `FIFO` / `BLOCK_DEVICE` /
  `CHAR_DEVICE`，未知则 `SpecialSubtype::Unknown`）；
* fingerprint plan 自动过滤，不会尝试 hash 一个 socket；
* `open_content` 对 special file 返回 `UnsupportedOperation`。

## 6. Sparse / compressed / CoW

v0.0.1：

```text
只定义 logical_size
```

不推导：

```text
physical bytes
reclaimable bytes
```

sparse 文件标记 `EntryFlags::SPARSE`。这一条同时解决了前面 duplicate 空间估算的问题：
既然不推导，就不会给出错误的「可释放空间」。

## 7. Windows case / Unicode / determinism

排序**不**用 lowercase、Unicode normalization 或 filesystem-specific comparison。
排序只是为了 reproducibility，不是在判断路径等价。

`Source::sort_key(locator)`：

| 平台 | key |
| --- | --- |
| Unix | raw filename / path bytes，lexicographic |
| Windows | lossless UTF-16 code units，lexicographic |

不做 case fold，不做 NFC/NFD normalization——任何归一化都会悄悄修改 identity 语义。

## 8. Race：rename / delete / modify

| 情况 | 结果 |
| --- | --- |
| 扫描期间消失 | `NotFound` (201)，从结果移除，继续扫描 |
| 扫描期间被改名 | 同上（旧 locator 不再可解析） |
| 扫描期间被修改 | `ChangedDuringScan`，进 `ExcludedCandidate` |
| hash 期间权限变化 | `PermissionChanged` (202) |
| hash 期间被修改 | `validate_observation` 判 `ChangedDuringScan`，不入 confirmed group |

判定由 `Source::validate_observation` 完成，filesystem 的默认实现是
`Observation::strict_compare`（object id + size + mtime + revision）。

## 9. 一致性：谁负责什么

| 责任 | 归属 |
| --- | --- |
| 提供 observation 字段 | Source |
| 决定「两次观察是否一致」 | Source（`validate_observation`） |
| 定义状态枚举与结果语义 | Core |
| 保证不稳定对象不进 confirmed group | Runtime / pipeline |
| 决定并行度 | Runtime（依据 `CONCURRENT_STAT` / `CONCURRENT_READ`） |
