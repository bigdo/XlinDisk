# Cross-platform correctness fixtures

「symlink policy」「same-filesystem policy」在 PR0 之前只是名字。从 PR0 起它们进
`ScanRequest`，并对应下面的 fixture 清单。**PR3 逐项落地，PR8 进 release gate。**

## Fixture 清单

| # | 场景 | 期望 |
| --- | --- | --- |
| 1 | regular file / directory / special entry | 正确分类 |
| 2 | symlink（Unix） | `DoNotFollow`：标记 `LINK`，不穿越 |
| 3 | junction / reparse point（Windows） | 同 #2 |
| 4 | broken link | `BROKEN_LINK`，不 crash |
| 5 | 循环链接 | 只报告，不重复遍历、不死循环 |
| 6 | mount boundary | `StayWithinRoots`：停止并标记 `MOUNT_BOUNDARY` |
| 7 | Windows 大小写 | 按平台语义处理，结果可复现 |
| 8 | Unicode 规范化（NFC / NFD） | 排序字节级，不依赖 collation |
| 9 | Unix special file（device / socket / fifo） | `EntryKind::Special`，`ObjectIdUnavailable::SpecialFile` |
| 10 | long path | 不截断、不 panic，报告错误码而非崩溃 |
| 11 | sparse file | 标记 `SPARSE`；不按 `logical_size` 计算占用 |
| 12 | hardlink | 同 `ObjectId`，重复组内只去重一次 |
| 13 | FAT / 无 inode 文件系统 | `FileSystemWithoutIdentity`，不输出可释放空间 |
| 14 | 网络挂载 | `NetworkFileSystem` |
| 15 | permission denied | entry-level，继续扫描 |
| 16 | 扫描期间 delete / rename | `GoneDuringScan`，不 crash |
| 17 | 扫描期间 modify | `ChangedDuringScan`，不进 confirmed duplicate |
| 18 | hash 期间 modify | 同上（re-stat 捕获） |
| 19 | 取消（1 / 2 / 8 worker） | 终止事件必达，返回 partial 或 cancelled |
| 20 | cancellation 中途的 progress 流 | `sequence` 允许空洞，`terminal` 必达 |

## 每条 fixture 的通过标准

1. ReferenceWalker 与 ParallelWalker 的**规范化结果一致**（#1–#20）；
2. 输出在 1 / 2 / 8 worker 下逐字节一致（#19、#20 除外，它们验证终止语义）；
3. 平台特有 fixture（#3、#7、#10）在对应 CI 目标上跑，其余全平台跑。

## 平台矩阵

* Desktop：Linux x86_64 / aarch64、macOS x86_64 / arm64、Windows x86_64。
* Core compile smoke：aarch64-apple-ios、aarch64-linux-android
  ——目的不是手机上有功能，而是防止 desktop-only 依赖渗进 core。
