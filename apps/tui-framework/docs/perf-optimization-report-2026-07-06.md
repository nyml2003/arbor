# arbor-tui 性能优化报告 2026-07-06

## 结论

本轮以 aster 的 `headless_strict` 标准基准路径作为试点，保留了所有正向或中性偏正的优化。最终结果：

| 指标       |  优化前 |  优化后 |    变化 |
| ---------- | ------: | ------: | ------: |
| Avg frame  | 0.291ms | 0.166ms |  -43.0% |
| P95 frame  | 0.581ms | 0.350ms |  -39.8% |
| P99 frame  | 0.639ms | 0.529ms |  -17.2% |
| S 水位帧   |   73.9% |   98.3% | +24.4pp |
| A 水位帧   |   26.1% |    1.7% | -24.4pp |
| B/C 水位帧 |    0.0% |    0.0% |    持平 |

最终日志：`apps/aster-rs/target/aster_perf_after.jsonl`。
优化前日志：`apps/aster-rs/target/aster_perf_full_before.jsonl`。

## 阶段基准

| 阶段                     |     Avg |     P95 |     P99 | 结果                         |
| ------------------------ | ------: | ------: | ------: | ---------------------------- |
| before                   | 0.291ms | 0.581ms | 0.639ms | 原始基线                     |
| focus + layout prealloc  | 0.262ms | 0.503ms | 0.566ms | 保留                         |
| transcript parse-once    | 0.261ms | 0.510ms | 0.564ms | 保留，P95 轻微抖动但整体正向 |
| scroll viewport render   | 0.197ms | 0.386ms | 0.421ms | 保留，主要收益来源           |
| diff row-slice fast path | 0.162ms | 0.346ms | 0.381ms | 保留                         |
| final correctness fix 后 | 0.166ms | 0.350ms | 0.529ms | 保留，修复动态 focus 回归    |

## 已落地优化

1. 按需重建 focus map

   - 文件：`arbor-tui-application/src/app.rs`、`terminal_app.rs`、`arbor-tui/src/app.rs`、`arbor-tui/src/testing.rs`
   - 改动：新增 `focus_dirty` 和 `request_focus_rebuild()`。只有初次渲染、resize、root rebuild、widget action handled 后才重建 focus map。
   - 回归处理：完整测试发现 Tabs 切换 active child 后 focus map 失效，已修成 handled action 后标记 focus dirty。
2. Layout HashMap 预分配

   - 文件：`arbor-tui-domain/src/layout_engine.rs`
   - 改动：measure/layout 前先统计节点数，用 `HashMap::with_capacity()` 降低每帧 rehash 和扩容。
3. Transcript build 阶段只解析一次 Markdown

   - 文件：`arbor-tui-composites/src/transcript/builder.rs`
   - 改动：`build()` 里每条消息只 `parse_blocks()` 一次，同一份 block 同时用于 `content_h` 计算和 widget 构建。
4. ScrollView 只渲染可视窗口

   - 文件：`arbor-tui-widgets/src/scroll/widget.rs`、`arbor-tui-domain/src/render.rs`、`screen.rs`
   - 改动：新增 `render_tree_viewport()` 和 `VirtualScreen::blit_region()`。ScrollView 仍按完整内容高度 layout，但只 render 与 viewport 相交的节点。
   - 收益：长 transcript 场景不再为每帧创建完整 `content_h` 高度的 child screen。
5. Diff row-slice fast path

   - 文件：`arbor-tui-domain/src/diff.rs`、`screen.rs`
   - 改动：先用整行 slice 比较跳过未变行，再扫描变化行。减少 `cell_at_ref()` 逐格 bounds check。
6. RichText 微优化

   - 文件：`arbor-tui-widgets/src/rich_text/widget.rs`
   - 改动：去掉 `flat_map(Some(...))`，改为直接 `map()`。

## 未强行落地的候选

1. DirtyTracker 真正限制渲染范围

   - 未落地。
   - 原因：正确实现需要缓存旧 layout/render 结果，并处理 dirty widget、祖先、子树、旧 rect、新 rect、文本高度变化后的兄弟节点位移。当前只用 dirty widget id 做局部 diff/render 会漏画。
   - 建议：下一轮先设计 layout/render cache invalidation，再做局部 render。
2. 完整 `render_into()` / buffer pool

   - 部分落地。
   - 已通过 `render_tree_viewport()` 和 `blit_region()` 降低 ScrollView 分配和拷贝。
   - 未改全局 Widget API，避免一次性触碰所有 widget。
3. RichText wrapped lines 持久缓存

   - 未落地。
   - 原因：当前 facade 每帧 rebuild widget tree，widget 内部缓存无法稳定复用。需要 keyed/reconciliation 后再做。
4. Layout arena/Vec 索引

   - 部分落地。
   - 已做 HashMap 预分配。
   - 未替换为 arena/Vec，因为这会改变 layout result 查询接口，影响面中高。
5. Widget keyed component / reconciliation

   - 未落地。
   - 原因：架构级改动，需要单独 PR/TEP 和较完整回归测试。

## 验证

已通过：

```bash
cargo test --workspace
cargo test -p aster-tui --features bench-log
cargo run -p aster-tui --features bench-log -- --bench --bench-out .\target\aster_perf_after.jsonl
```

最终基准：

```text
Profile: headless_strict
Total frames: 119
Avg frame ms: 0.166
P95 frame ms: 0.350
P99 frame ms: 0.529
S frames: 117 (98.3%)
A frames: 2 (1.7%)
B frames: 0 (0.0%)
C frames: 0 (0.0%)
Budget: PASS
```

## 剩余风险

1. `focus_dirty` 现在对 widget action 采用保守失效策略。它保证 Tabs 这类动态 child 场景正确，但会让输入类 handled action 后重建 focus map。最终基准仍然正向。
2. ScrollView viewport render 依赖 layout 坐标裁剪。现有滚动、透明容器、边框、焦点输入测试均通过，但嵌套自渲染子树的复杂 widget 仍建议增加专项用例。
3. DirtyTracker 局部 render 仍是下一阶段最大收益点，但不能跳过缓存和 invalidation 设计。
