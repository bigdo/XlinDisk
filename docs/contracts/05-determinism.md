# Determinism

扫描顺序可以不确定，输出必须确定。否则测试、CLI diff、MCP 响应和 regression
全部失去意义。

## Test invariant

> 同一 Source snapshot、同一 Plan、同一版本的 Core，在不同 worker count 下应产生相同的
> **规范化结果**。

CI 至少比较 `workers = 1 / 2 / 8` 的 normalized result schema，
而不是比较扫描事件顺序。

## canonical sort key

排序由 Source 提供的 `sort_key(locator)` 决定，规则是**字节级**且**不修改语义**的：

| 平台 | key |
| --- | --- |
| Unix | raw filename / path bytes |
| Windows | lossless UTF-16 code units |

两者都按 lexicographic 比较。

**不做**：

```text
lowercase / case folding
NFC / NFD normalization
filesystem-specific comparison
```

原因：排序是为了 reproducibility，不是在判断路径等价。任何 case fold 或 normalization
都会悄悄改变 identity 语义——那是 Source 的事，不是排序的事。

## 冻结规则

1. `canonical_cmp` 是字节级比较，不使用 locale / Unicode collation。
2. group 排序：size 降序 → fingerprint 升序 → canonical locator key 升序。
3. member 顺序：按 canonical locator key，不依赖 worker 数量。
4. 确定性在**输出阶段**施加，不在扫描阶段强求顺序——否则等于放弃并行。

## 代码

```rust
pub fn canonical_cmp(a: &[u8], b: &[u8]) -> Ordering;
pub struct GroupOrderKey { size, fingerprint, locator_key }  // Ord 已按上述规则实现
pub fn sort_canonical(keys: &mut [GroupOrderKey]);
```

`determinism::tests::order_is_independent_of_arrival_order` 用全排列验证任意到达顺序
都归一到同一序列，这是 1/2/8 worker 一致性的最小可执行版本。
