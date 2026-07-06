# TEP-0008 性能优化报告 2026-07-06

## 结论

本轮保留了安全且正向的改动。

标准基准没有开启 cache shadow。最终结果：

| 指标 | Stage1 记录 | 本轮最终 | 变化 |
| --- | ---: | ---: | ---: |
| Avg frame | 0.166ms | 0.167ms | +0.6% |
| P95 frame | 0.338ms | 0.327ms | -3.3% |
| P99 frame | 0.384ms | 0.382ms | -0.5% |
| C 水位帧 | 0 | 0 | 持平 |

最终标准日志：

```text
apps/aster-rs/target/aster_perf_tep0008_final.jsonl
```

cache shadow 采样日志：

```text
apps/aster-rs/target/aster_perf_tep0008_cache_shadow.jsonl
```

## 本轮启用的优化

### Render action 不再重建 focus map

之前 `dispatch_action()` 对所有 handled widget action 都调用 `request_focus_rebuild()`。

这会让输入字符、列表移动、按钮触发等普通 render 更新也重建 focus map。

本轮增加：

1. `Widget::dirty_on_action()`。
2. 默认返回 `DirtyKind::Render`。
3. Tabs 的左右切换返回 `DirtyKind::Structure`。
4. `App::dispatch_action()` 只有在 `Structure` 或 `Full` 时重建 focus map。

结果：

1. 输入类 action 只触发 render dirty。
2. Tabs active child 切换仍触发 focus rebuild。
3. 标准基准 P95 从 0.338ms 降到 0.327ms。

## 本轮完成的框架层基础

1. `WidgetKey`、`NodeIdentity` 和 `.key(...)` API。
2. 同级 duplicate key 检查。
3. 无 key path identity。
4. `DirtyKind` 分级 dirty tracker。
5. `arbor_tui_domain::reconcile` 计划器。
6. `LayoutCacheShadow` 和 `RenderCacheShadow`。
7. `render_tree_with_fragments()`。
8. `diff_regions(old, new, regions)`。
9. `--bench-cache-shadow` 采样参数。
10. JSONL meta 中输出 cache hit/miss/mismatch。

## 负向路径处理

最初实现 render cache shadow 时，标准 render 会走 no-op observer。

连续基准显示退化：

```text
P95 0.387ms -> 0.402ms
P99 0.430ms -> 0.496ms
```

处理：

1. 标准 `render_tree()` 恢复为直接递归。
2. `render_tree_with_fragments()` 只在 cache shadow 开启时使用。
3. `App::enable_cache_shadow(false)` 为默认值。

回退后标准基准恢复为：

```text
Avg 0.167ms
P95 0.327ms
P99 0.382ms
C frames 0
```

## Cache Shadow 采样

命令：

```bash
cargo run -p aster-tui --features bench-log -- --bench --bench-cache-shadow --bench-no-fail --bench-out .\target\aster_perf_tep0008_cache_shadow.jsonl
```

结果：

```text
frames: 119
layout hits: 2171
layout misses: 40
layout mismatches: 235
render hits: 0
render misses: 3
render mismatches: 65
```

判断：

1. layout cache 有命中空间。
2. render cache 不能启用。
3. render cache 需要稳定 `render_rev` 和 props update 协议。
4. 当前 facade 每帧 rebuild widget tree，直接复用旧 widget 实例会丢新 props。

## 未启用项

以下能力已经有协议或 API，但没有默认启用：

1. retained widget instance。
2. layout cache 复用。
3. render cache 复用。
4. dirty rect 局部 render/emit。

原因：

1. `WidgetNode` 还没有安全的 props update 接口。
2. widget render/layout revision 还没有覆盖所有内建组件。
3. dirty rect 需要 old rect、new rect、ancestor clip 和 layout dirty sibling 扩展。
4. 缺少这些条件时启用局部 render 会漏画或保留旧内容。

## 验证

已通过：

```bash
cargo test --workspace
cargo test -p aster-tui --features bench-log
cargo run -p aster-tui --features bench-log -- --bench --bench-out .\target\aster_perf_tep0008_final.jsonl
cargo run -p aster-tui --features bench-log -- --bench --bench-cache-shadow --bench-no-fail --bench-out .\target\aster_perf_tep0008_cache_shadow.jsonl
```

## 下一步

下一轮不要先启用 cache。

先补三件事：

1. `WidgetRevision`。
2. `Widget::update_from_spec()` 或等价 props update 协议。
3. retained root owner。

完成后再启用 Stage 5 和 Stage 7。
