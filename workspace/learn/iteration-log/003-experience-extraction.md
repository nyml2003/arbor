# 003 - 经验沉淀

日期：2026-06-07

## 做了什么

深度阅读了 work-context/repos/ 中的全部 31 个项目，从中提取了 8 个项目的设计模式，写入 `learn/patterns/`：

| 项目 | 语言 | 模式文档 | 核心主题 |
|------|------|---------|---------|
| ObolosFS | TS | `vfs-pure-logic.md` | 纯逻辑 VFS + Driver 接口 + Result-based error + Brand types |
| workshop | Rust | `task-domain-model.md` | Aggregate Root + 状态机 + Newtype ID + Repository |
| work-context-2 | Python | `projection-and-knowledge-pipeline.md` | 投影模式 + 知识管道 + 状态机设计 |
| work-context | Python | `skill-validation-pipeline.md` | 校验管线 + Python Result/Option + DI 容器 |
| jue | TS | `minimal-reactivity-vdom.md` | 响应式引擎 + VDOM diff/patch |
| Aura | TS | `lightweight-ssr-inline-data.md` | 内联 JSON SSR |
| Ethereal | Rust | `monorepo-dependency-graph.md` | 依赖图 + 拓扑排序 + import 扫描校验 |
| Untitled | Astro+TS | `astro-file-tree-portfolio.md` | SSG + content collections + 文件树作品集 |

## 学到了什么

### 跨语言模式共振

Result-based error、纯逻辑 core、Driver/Repository 接口、不可变领域类型——这些模式在 Rust/TS/Python 三种语言中独立出现，不是语言特化的。

### 按功能维度的项目分类

31 个项目中，8 个是可复用的库（FluxBin、Clover、Ethereal、shamrock、forgebench、sysfolio、Untitled、dealpytool），6 个是最佳实践参考（CultivationBattle、2048-h5 等），其余是纯 demo 或空壳。详细清单在 `manage/repos-inventory.md`。

### 模式文档应该独立

初版 pattern 用"对 Arbor 有什么价值"做框架，污染了项目本身的设计描述。已全部修正——每篇 pattern 独立描述一个项目，Arbor 不出现（来源引用除外）。

## 决策

1. 经验抽取的粒度：一个核心项目一篇 pattern，不合并
2. Pattern 文档独立——不对 Arbor 做引用
3. 37 个旧项目 + 31 个 repos 项目 → 只有 8 个有独特架构值得抽

## 下一步

已完成。进入 learn/ 结构整理。
