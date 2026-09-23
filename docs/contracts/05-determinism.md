# Determinism

扫描顺序可以不确定，输出必须确定。否则测试、CLI diff、MCP 响应和 regression
全部失去意义。

## 冻结规则

1. **排序是字节级**（`canonical_cmp`），不使用 locale / Unicode collation。
   同一 fixture 在 macOS、Windows、Linux 上必须给出同一顺序。
2. **group 排序**：size 降序 → fingerprint 升序 → canonical locator key 升序。
3. **member 顺序**：按 canonical locator key，不依赖 worker 数量。
4. **验收标准**：同一 fixture 在 **1 / 2 / 8 worker** 下输出逐字节一致。

## 实现约束

* 确定性在**输出阶段**施加，不在扫描阶段强求顺序——否则等于放弃并行。
* `Fingerprint` 的 `Ord` 是字节级比较，不是 hash 的语义大小。
* 排序使用稳定排序，方便对已排好的流重复排序。

## 代码

```rust
pub fn canonical_cmp(a: &[u8], b: &[u8]) -> Ordering;
pub struct GroupOrderKey { size, fingerprint, locator_key }  // Ord 已按上述规则实现
pub fn sort_canonical(keys: &mut [GroupOrderKey]);
```

`determinism::tests::order_is_independent_of_arrival_order` 用全排列验证：
任意到达顺序都归一到同一序列。这是 1/2/8 worker 一致性的最小可执行版本。
