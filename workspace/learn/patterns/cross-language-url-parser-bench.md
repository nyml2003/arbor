# 模式：跨语言自研解析器 + Benchmark（url-parser-bench）

## 一句话

用 Rust 和 Haskell 各自手写 URL 解析器，共享同一套 fixtures，用脚本统一跑 bench 对比。两套实现都支持 URL 的 6 个组成部分（scheme/authority/path/query/fragment），并各自对照生态标准库。

## 核心架构

```
fixtures/
  ├── urls.txt              ← 共享测试数据
  └── query_strings.txt
    │
    ├──────────────┬──────────────┐
    ▼              ▼              ▼
rust/           haskell/       scripts/
  src/lib.rs      src/UrlParser.hs  compare.ps1
  (String)        (ByteString)
```

两套解析器同构但不共享代码——数据结构一致（Url/Authority/QueryPair），实现各自用各自语言的惯用方式。

## 关键设计

### 1. 共享 fixtures，独立实现

```
fixtures/urls.txt → Rust parse_url() → bench
                  → Haskell parseUrlBytes() → bench
```

测试数据和 benchmark 脚本是唯一的共享组件。两边独立编译、独立运行。好处：任何一边的实现更改不影响另一边，bench 结果有参照。

### 2. 双路径设计：正确 vs 快速

Haskell 侧：

```
parseUrl (String) → parseUrlBytes (ByteString) → parseUrlBytesFast (FastUrl)
                    ↑ 校验 UTF-8                      ↑ 保留原始 ByteString
```

- `parseUrlBytesFast` 返回 `FastUrl`——字段是原始 `ByteString`，不做 UTF-8 校验
- `parseUrlBytes` 调 `thawFastUrl`——把 `ByteString` 解码为 `Text`，性能差但正确
- `parseUrl` 是最外层，String → ByteString → FastUrl → Url

Rust 侧直接解析为 `String`——没有 ByteString 层。但同样分了解析器（`parse_url`）和对照实现（`url` crate）。

### 3. 百分号解码：零分配优化

Rust 侧：
```rust
fn collect_percent_chunk(input, start) -> (String, usize) {
    let mut decoded = Vec::new();
    while bytes[index] == b'%' {
        decoded.push((hex_value(hi)? << 4) | hex_value(lo)?);
        index += 3;
    }
    String::from_utf8(decoded)
}
```

Haskell 侧（快速路径）：
```haskell
percentDecodeBytes plusAsSpace input
  | not (needsDecoding plusAsSpace input) = Right input  -- ← 没有 % 就直接返回原串
  | otherwise = unsafeCreate outputLength (fillDecodedBytes ...)
```

如果字符串没有百分号编码，**直接返回原始 ByteString，零拷贝**。这是手写解析器相比通用库最大的性能优势——你可以在数据特征上做针对性优化。

### 4. Byte 级别的词法常量

```haskell
ampersandByte, colonByte, percentByte :: Word8
percentByte = 37
colonByte = 58
-- ...
isSchemeByte value = isAsciiAlphaNum value || value == plusByte || value == hyphenByte || value == dotByte
```

不用正则，不用 `char` 比较——所有字符判断都落到 `Word8` 级别。Haskell 的 `ByteString` 是 `[Word8]`，`breakWhere` 比 `break` 快。

### 5. Benchmark：自研 vs 生态对照

```powershell
pwsh .\scripts\compare.ps1 -Iterations 50000
# Rust: custom-parse-* vs url-crate-* vs url-ecosystem-*
# Haskell: custom-* vs stdlib-readp-*
```

每个解析器有明确的 benchmark 标签：`custom-*` 是手写的，`stdlib-readp-*` 是标准库的，`url-crate-*` 是生态 crate 的。结果可以直接对照——手写 vs 标准库 vs 生态，在同一个数据集、同一台机器上。

## 反模式警示

### ❌ 用正则做 URL 解析

RFC 3986 的 ABNF 语法用正则翻译会引入大量回溯，且难以正确解析 IPv6、百分号编码等边界情况。手写字符级解析器反而更清晰。

### ❌ 不共享 fixtures 做跨语言比较

如果两边各用各的测试数据，bench 结果没有对照意义。共享 fixtures 是跨语言 benchmark 的前提。

## 来源

- url-parser-bench 源码（`rust/src/lib.rs`、`haskell/src/UrlParser.hs`、`scripts/compare.ps1`、`README.md`）
- 2026-06-07 agent 阅读后提炼
