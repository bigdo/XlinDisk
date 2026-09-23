# Cross-platform correctness fixtures

「symlink policy」「same-filesystem policy」在 r0 之前只是名字。从 r1 起它们是
`ScanRequest` 的字段，并对应下面的 fixture 清单。
**PR3 逐项落地，PR8 进 release gate。**

具体语义见 [spec/filesystem-semantics.md](../spec/filesystem-semantics.md)。

## Fixture 清单

| # | 场景 | 期望 |
| --- | --- | --- |
| 1 | regular file / directory | 正确分类 |
| 2 | symlink（Unix） | `DoNotFollow`：标记 `LINK`，不穿越 |
| 3 | junction / reparse point（Windows） | `DoNotTraverseReparsePoint`：标记 `REPARSE_POINT` |
| 4 | broken symlink | `EntryKind::Symlink` + `TARGET_UNAVAILABLE`，**不是** scan fatal |
| 5 | 循环链接（显式 follow 时） | 基于目录 identity 检测 ancestor cycle，只报告不重复遍历 |
| 6 | mount boundary | `StayOnInitialFilesystem`：停止并标记 `MOUNT_BOUNDARY` |
| 7 | Windows 大小写 | 按平台语义枚举，排序仍为 UTF-16 码元，不做 case fold |
| 8 | Unicode 规范化（NFC / NFD） | 排序字节级，**不做** normalization |
| 9 | Unix special file | `EntryKind::Special` + `SOCKET` / `FIFO` / `BLOCK_DEVICE` / `CHAR_DEVICE`；枚举但不 `open_content` |
| 10 | long path | 不截断、不 panic，报告错误码而非崩溃 |
| 11 | sparse file | 标记 `SPARSE`；只定义 `logical_size`，不推导 physical |
| 12 | hardlink | 同 `ObjectId`，`object_groups` 折叠为一个存储对象 |
| 13 | FAT / 无 inode 文件系统 | `FileSystemWithoutIdentity`，不输出可释放空间 |
| 14 | 网络挂载 | `NetworkFileSystem` |
| 15 | permission denied | entry-level，继续扫描 |
| 16 | 扫描期间 delete / rename | `NotFound` (201)，不 crash |
| 17 | 扫描期间 modify | `ChangedDuringScan`，进 `ExcludedCandidate` |
| 18 | hash 期间权限变化 | `PermissionChanged` (202) |
| 19 | hash 期间 modify | `validate_observation` 判 `ChangedDuringScan` |
| 20 | 取消（1 / 2 / 8 worker） | 终止事件必达，返回 `Partial` 或 `Cancelled` |
| 21 | cancellation 中途的 progress 流 | `sequence` 允许空洞，`terminal` 必达 |
| 22 | 超大 size group（500 万同尺寸） | `ResourceLimitExceeded` 或 `GroupTooLarge`，不 OOM |
| 23 | 1 / 2 / 8 worker determinism | normalized result 逐字节一致 |

## 每条 fixture 的通过标准

1. ReferenceWalker 与 ParallelWalker 的**规范化结果一致**（#1–#22）；
2. 输出在 1 / 2 / 8 worker 下逐字节一致（#20、#21 除外，它们验证终止语义）；
3. 平台特有 fixture（#3、#7、#10）在对应 CI 目标上跑，其余全平台跑。

## 平台矩阵

* Desktop：Linux x86_64 / aarch64、macOS x86_64 / arm64、Windows x86_64。
* Core compile smoke：aarch64-apple-ios、aarch64-linux-android
  ——目的不是手机上有功能，而是防止 desktop-only 依赖渗进 core。
