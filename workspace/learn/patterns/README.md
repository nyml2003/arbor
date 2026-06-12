# 设计模式

## 按来源项目

| 文档 | 来源项目 | 语言 | 核心主题 |
|------|---------|------|---------|
| `vfs-pure-logic.md` | ObolosFS | TS | 纯逻辑 VFS 核心、Driver 接口、Result-based error、Brand types、ofsh 编译器架构 |
| `task-domain-model.md` | workshop/workc | Rust | DDD Aggregate Root、状态机、Newtype ID、Repository 模式、Presenter 输出分离 |
| `projection-and-knowledge-pipeline.md` | work-context-2 | Python | 主数据投影模式、知识管道、状态机设计方法论、Schema versioning |
| `skill-validation-pipeline.md` | work-context | Python | 校验管线、Python Result/Option 类型、frozen dataclass、懒加载 DI 容器 |
| `ts-runtime-performance-rules.md` | jue + ObolosFS | TS | 非 UI 热路径规则：显式依赖、slot/TypedArray、bitset、Result、single pass |
| `ts-ui-performance-rules.md` | Arbor + visual-frame + jue | TS | UI 层性能规则：按需挂载、局部加载、窗口化渲染、少做无意义布局 |
| `minimal-reactivity-vdom.md` | jue | TS | 响应式引擎（WeakMap + Proxy）、VDOM diff/patch 6 种操作 |
| `lightweight-ssr-inline-data.md` | Aura | TS | 内联 JSON + 前端 hydrate 的轻量 SSR 模式 |
| `monorepo-dependency-graph.md` | Ethereal | Rust | WorkspaceGraph 依赖图、拓扑排序、import 扫描架构校验 |
| `astro-file-tree-portfolio.md` | Untitled | Astro+TS | SSG 文件树作品集、统一 ContentNode、递归 TreeItem |
| `cpp-c-abi-dll.md` | dll_csv_transformer | C++ | C ABI DLL + CMake 跨平台导出宏 |
| `pokemon-battle-engine-rust.md` | tdd-demo | Rust | 回合制对战引擎：Creature/Move/BattleState 领域模型 + 类型克制 |
| `cross-language-url-parser-bench.md` | url-parser-bench | Rust+Haskell | 跨语言自研解析器 + 共享 fixtures benchmark |
| `virtual-scroll-padding.md` | visual-frame | Vue3/TS | 前后占位空白实现的水平虚拟滚动 |
| `cpp-compiler-pipeline.md` | changfen/Sed | C++ | Flex/Bison → IR → 多 pass 后端优化编译器管线 |
| `fluxbin-transport-abstraction.md` | FluxBin | TS | 多传输后端抽象（ws/tcp） + env 隔离 |
| `clover-js-toolchain.md` | Clover | TS | 自建 JS 工具链框架（eslint+protocol+std+tsconfig） |
| `shamrock-rust-workspace.md` | shamrock | Rust | 8-crate Rust workspace 游戏引擎 |
| `rust-cli-workspace.md` | forgbench | Rust | clap + domain 分离的标准 CLI 结构 |
| `vite-tsx-portfolio.md` | sysfolio | TSX | Vite+TSX 作品集：entities/features/shared/app 四层 + 双测试 |
| `kaubo-language-engine.md` | kaubo | C++ | 自研语言引擎：手写 Lexer/Parser/ByteCode VM/Object 模型/GC |
| `ipc-layer-pattern.md` | Arbor (Phase 1) | TS | Electron IPC 四层管线：zod schema → handler → bridge → API |
| `solidjs-file-tree.md` | Arbor (Phase 1) | TS | createResource + Signal 递归文件树组件 |

## 按主题

| 主题 | 文档 |
|------|------|
| 错误处理 | `vfs-pure-logic.md`、`task-domain-model.md`、`skill-validation-pipeline.md` |
| 接口抽象 | `vfs-pure-logic.md`、`task-domain-model.md` |
| 状态机 | `task-domain-model.md`、`projection-and-knowledge-pipeline.md` |
| 性能/热路径 | `ts-runtime-performance-rules.md`、`ts-ui-performance-rules.md`、`cross-language-url-parser-bench.md`、`virtual-scroll-padding.md` |
| 文件树 | `solidjs-file-tree.md`、`astro-file-tree-portfolio.md` |
| 静态站点/SSR | `lightweight-ssr-inline-data.md`、`astro-file-tree-portfolio.md` |
| 构建/依赖管理 | `monorepo-dependency-graph.md` |
| 响应式/VDOM | `minimal-reactivity-vdom.md` |
| IPC 通信 | `ipc-layer-pattern.md` |
| 语言引擎 | `kaubo-language-engine.md` |
| 校验/质量 | `skill-validation-pipeline.md` |

## 每篇文档的约定

- 独立描述一个项目或一个模式——不引用 Arbor 或其他项目
- 来源标注在文末
- "反模式警示"节标注常见错误
