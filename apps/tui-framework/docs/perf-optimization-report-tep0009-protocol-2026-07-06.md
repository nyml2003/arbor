# TEP-0009 协议基础改造性能报告 2026-07-06

## 结论

本轮只落地 TEP-0009 的协议基础，不启用 layout cache、render cache 或 dirty rect 默认路径。

标准 aster bench 通过。相对 TEP-0008 最终标准日志，P95 下降，P99 轻微上升但未超过 5% 门槛，C frames 仍为 0。

| 指标 | TEP-0008 最终 | 本轮标准 | 变化 |
| --- | ---: | ---: | ---: |
| Avg frame | 0.167ms | 0.168ms | +0.6% |
| P95 frame | 0.327ms | 0.317ms | -3.1% |
| P99 frame | 0.382ms | 0.390ms | +2.1% |
| C frames | 0 | 0 | 持平 |

标准日志：

```text
apps/aster-rs/target/aster_perf_tep0009_protocol.jsonl
```

cache shadow 采样日志：

```text
apps/aster-rs/target/aster_perf_tep0009_protocol_shadow.jsonl
```

## 本轮改造

1. 新增 `SignalId`、`SignalDep` 和 `SignalSource`。
2. `Signal` 订阅记录 `DirtyKind`，不再只能把订阅者标成 Render dirty。
3. 新增 `PropsRevision`、`PropsRevisionBuilder` 和 domain 层 `ComponentProps` 协议。
4. 旧 `Widget` trait 暴露 `props_revision()` 和 `signal_deps()` 桥接方法。
5. Text、Button、Input、RichText、ScrollView 声明低风险 props revision / signal deps。
6. `FrameStats` 和 aster JSONL 增加 dirty 等级分布字段：
   - `dirty_render`
   - `dirty_layout`
   - `dirty_structure`
   - `dirty_theme`
   - `dirty_full`

## 标准路径结果

命令：

```bash
cargo run -p aster-tui --features bench-log -- --bench --bench-out target\aster_perf_tep0009_protocol.jsonl
```

结果：

```text
Profile: headless_strict
Total frames: 119
Avg frame ms: 0.168
P95 frame ms: 0.317
P99 frame ms: 0.390
C frames: 0
Budget: PASS
```

分 scene 均值：

| Scene | Avg |
| --- | ---: |
| idle | 0.143ms |
| streaming | 0.295ms |
| scrolling | 0.000ms |
| palette_open | 0.065ms |
| model_switch | 0.309ms |
| exit | 0.001ms |

dirty 分级汇总：

```text
dirty render/layout/structure = 7/0/0
```

## Cache Shadow 采样

命令：

```bash
cargo run -p aster-tui --features bench-log -- --bench --bench-cache-shadow --bench-no-fail --bench-out target\aster_perf_tep0009_protocol_shadow.jsonl
```

结果：

```text
Avg frame ms: 0.213
P95 frame ms: 0.480
P99 frame ms: 0.576
C frames: 0
Budget: PASS
```

cache 采样：

```text
layout hits/misses/mismatch = 129586/4186/15624
render hits/misses/mismatch = 0/308/4181
```

判断：

1. 新协议元数据没有让标准路径超预算。
2. shadow mismatch 仍非 0，cache 默认路径不能启用。
3. 后续必须先补 retained update、FrameSnapshot 和更精确的 props/signal deps，再重新评估 cache。

## 验证

已通过：

```bash
cargo test --workspace
cargo test -p aster-tui --features bench-log
cargo run -p aster-tui --features bench-log -- --bench --bench-out target\aster_perf_tep0009_protocol.jsonl
cargo run -p aster-tui --features bench-log -- --bench --bench-cache-shadow --bench-no-fail --bench-out target\aster_perf_tep0009_protocol_shadow.jsonl
```

## 剩余风险

1. `ComponentProps` 已要求自定义 props 显式 opt-in。现有 aster 自定义 props 暂时使用默认 revision 0，后续需要来源 revision 才能安全进入 cache key。
2. Input 仍是旧协议字段状态，还没有迁移到 component-owned Signal。
3. 当前 `signal_deps()` 返回 `Vec`，适合作为桥接接口；最终 retained protocol 应改成 props 持有稳定 deps slice。
4. cache shadow mismatch 仍然存在，不能把本轮协议元数据误认为 cache 可启用证据。
