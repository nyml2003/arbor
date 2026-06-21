# Decisions — 技术选型记录

## 1. 容器应用选 Electron + SolidJS，而非 Tauri

**决定**：容器应用使用 Electron + SolidJS。

**理由**：
- 用户对 Electron + SolidJS 有成熟经验（WatchDesk 项目）
- Tauri 需要 Rust 编译链路，用户反馈 Rust 迭代偏慢，不适合快速验证阶段
- Electron 生态成熟，文件系统、窗口管理、自动更新等有成熟方案
- SolidJS 的响应式模型比 React/Vue 更轻，适合桌面端性能要求

**代价**：
- Electron 包体大（~150MB+）
- 内存占用高于 Tauri

**未来可能重新评估**：如果后续性能/包体成为瓶颈，可以考虑 Tauri 做宿主壳重写。

---

## 2. 管理和沉淀工具用 TypeScript，而非 Rust

**决定**：`@arbor/manage-*` 和 `@arbor/knowl-*` 包用 TypeScript 实现。

**理由**：
- workshop (Rust) 的实践证明：在需求快速变化的早期，Rust 的编译时间和严格的类型系统拖慢迭代
- 管理和沉淀工具的复杂度在领域建模和交互设计，不在底层性能
- TypeScript 的类型系统足够表达复杂的领域模型
- 用户已有大量 TS/Node 经验，只是需要系统化沉淀——做这件事本身就是沉淀的过程
- 用户已有 Node.js 环境，不需要独立二进制

**代价**：
- 二进制体积比 Rust 大
- 性能不如 Rust（但对于管理/沉淀类工具，瓶颈在 IO 不在计算）

**适用范围**：
- 管理工具（任务 CRUD）
- 沉淀工具（笔记读写）
- 容器 UI（SolidJS）
- 不适用于高性能计算类工具（如语言解析器——保留 Rust）

---

## 3. 选 SolidJS 而非 React

**决定**：UI 框架使用 SolidJS。

**理由**：
- 用户在 WatchDesk 中已深度使用 SolidJS
- SolidJS 编译后无虚拟 DOM 运行时，性能接近原生
- 响应式原语（Signal、Effect）适合文件树这种数据驱动 UI
- 比 React 轻量，打包体积更小

**代价**：
- 生态比 React 小（但桌面应用需要的组件不多，影响有限）

---

## 4. 文件存储而非数据库

**决定**：管理/沉淀工具使用文件系统存储（JSON/YAML/Markdown），不引入 SQLite 或其他数据库。

**理由**：
- 文件即数据：可以直接用文件管理器浏览、编辑、备份、git 版本控制
- 零依赖：不需要引入数据库驱动
- 与文件树隐喻一致：树上看到的节点就是文件系统中的节点
- 用户体量的数据（个人使用，不是 SaaS）文件系统完全够用

**代价**：
- 查询/搜索能力弱于数据库（后续如果需要可以做索引层）
- 并发一致性需要自己处理

**未来可以加**：如果搜索/查询成为瓶颈，在文件层之上加一层 SQLite 索引，文件仍是 source of truth。

---

## 5. pnpm workspace 做 monorepo 管理

**决定**：使用 pnpm workspace 管理 monorepo。

**理由**：
- 用户在 WatchDesk、Aura、ObolosFS 中都有 pnpm workspace 经验
- pnpm 的严格依赖管理避免幽灵依赖
- workspace protocol (`workspace:*`) 让内部包引用简单
- 支持过滤和并行构建

**结构**：
```
arbor/
├── packages/
│   ├── manage-core/
│   ├── manage-cli/
│   ├── knowl-core/
│   ├── knowl-cli/
│   └── ...
├── apps/
│   └── container/       # Electron + SolidJS
└── pnpm-workspace.yaml
```

---

## 6. CLI 以 npm bin 脚本分发，不做独立二进制

**决定**：CLI 工具以 npm package 的 `bin` 入口形式分发（`node dist/cli.js`），不做独立二进制编译。参考 ObolosFS ofsh 的 CLI 方案。

**理由**：
- bun 近期不稳定，不引入 bun 工具链依赖
- ObolosFS 的 `@obolosfs/ofsh` 已验证了 SWC + tsc → `bin` entry 的 CLI 分发模式
- 目标用户（自己）有 Node.js 环境，不需要独立二进制
- 保持工具链纯 Node.js + pnpm，减少复杂度

**代价**：
- 需要 Node.js 运行时（用户已有）
- 没有独立二进制那么「干净」

**参考**：ObolosFS `packages/ofsh/` —— SWC 转译 JS、tsc 生成类型、package.json `"bin"` 字段指向 `dist/cli.js`

---

## 7. 核心逻辑抽离为独立包（core/cli/ui 三层）

**决定**：每个工具域拆为三个包：

```
@arbor/[domain]-core     # 纯逻辑，零 UI/CLI 依赖
@arbor/[domain]-cli      # CLI 壳，调 core
容器内 UI                 # SolidJS 组件，调 core
```

**理由**：
- core 可以独立测试、独立发布
- CLI 和 GUI 是 core 的两个壳，不互相依赖
- 如果以后换宿主（比如从 Electron 换 Tauri），core 不受影响

---

## 8. 沉淀工具依赖 agent 能力，不做重代码

**决定**：沉淀域不构建 `@arbor/knowl-core` + `@arbor/knowl-cli` 等独立包。笔记以纯 Markdown 文件形式存放在 `learn/` 目录下，由 Claude/agent 提供组织、搜索、标签建议、模式提取等能力。

**理由**：
- 沉淀的核心行为（总结、归类、检索、提取模式）恰好是 LLM agent 擅长的事
- 写代码做 Markdown 解析、标签系统、搜索索引，就是重新发明一个更差的 agent
- 保持沉淀域轻量：文件系统即数据结构，agent 即引擎
- 随着 agent 能力提升，沉淀域自动增强——写死的代码做不到这一点

**代价**：
- 需要 agent 会话才能做沉淀操作（离线不可用）
- 搜索结果质量依赖 agent 的上下文窗口和推理能力

**未来可以加**：如果沉淀量大到 agent 上下文装不下，可以在文件层之上加嵌入索引（RAG），文件仍是 source of truth。

---

## 9. 暂不做 Rust 部分，但不排除

**决定**：Phase 1-3 全用 TypeScript。Rust 在 Arbor 中的角色留到以后。

**理由**：
- 当前优先级最高的容器、管理、沉淀三个域，TypeScript 最合适
- 等引擎跑通以后，迭代域的工具（如文件处理、语言解析）自然可以用 Rust
- 避免过早引入多语言复杂度

**Rust 的未来角色**：
- 迭代域：性能敏感的工具（解析器、文件操作、编译器）
- 可能的 Tauri 重写（如果需要更轻的桌面宿主）
- GearBox 框架概念可以融入

---

## 10. 截图工具单独走 Tauri 2 + Rust-first，不复用 Arbor 的 Electron 壳

**决定**：`apps/capture/` 作为独立工具孵化，技术路线使用 `Tauri 2 + Rust-first + 静态 overlay + 延迟加载的 SolidJS settings`。

**理由**：
- 截图工具是常驻小应用，不适合继续背 Electron 的完整运行时
- 这个工具的热路径是抓屏、裁剪、编码、剪贴板和通知，主收益在 Rust，不在前端
- overlay 是截图首屏，应该尽量接近静态页面，不值得为它启动整套框架运行时
- settings 页不是高频路径，可以单独按需加载框架
- Tauri 足够承载这两类页面，不需要把 Arbor 容器壳复用进去
- 这样可以复用 Arbor 的分层经验，但不把 Arbor 的产品边界和运行时边界带过来

**代价**：
- 仓库里会出现 TS 和 Rust 的混合栈
- 后续实现需要 Tauri 和 Rust 工具链
- 文档、目录和构建约定需要单独维护一套

**边界**：
- 这条决策只适用于 `apps/capture/`
- `apps/container/` 仍然保持 Electron + SolidJS 路线
- 管理和沉淀工具仍然按原计划留在 TypeScript

**当前约束**：
- 截图主链路全部放在 Rust
- overlay 不上框架，不做 hydration
- settings 才使用 SolidJS，而且按需加载
- 前端不承载图像处理和缓存管理
- 先写文档和骨架，再进入实现

---

## 11. 保留 Rust Native GUI 作为第二套 GUI 储备线

**决定**：Arbor 保留 `Rust DSL + 平台适配层` 作为第二套 GUI 技术路线。它和 `apps/container/` 的 Electron + SolidJS 主容器并列，但现阶段不替代主容器。

**定位**：
- Electron + SolidJS：承载 Arbor 主容器、复杂内容界面、快速迭代和 Web 生态能力
- Rust Native GUI：承载轻量系统工具、常驻小窗口、高性能交互、强平台能力

**理由**：
- KeyDock 已经验证了一个可行形态：安全 Rust app 层生成组件树，平台层负责窗口、渲染、输入和 DPI
- 这条路线可以把 Win32/unsafe 限制在 platform adapter，不让系统 API 泄漏到业务状态机
- 对虚拟键盘、截图 overlay、悬浮工具、托盘小面板这类系统工具，原生窗口比完整 Web runtime 更合适
- 这不是 Tauri 的替代品。Tauri 适合 Web UI + Rust 能力，Native GUI 适合完全不需要 WebView 的工具

**当前约束**：
- 不立即抽公共框架 crate
- 不把 Arbor 主容器迁到 Native GUI
- 不承诺跨平台一次到位
- 先把 KeyDock 作为样本，继续沉淀 DSL、primitive tree、platform adapter 和 unsafe boundary

**后续触发条件**：
- 出现第二个需要原生小窗口的 Arbor 工具
- KeyDock 的组件模型稳定到可以复用
- Windows 适配层边界通过测试和静态扫描持续保持干净

---

## 12. Arbor 采用孵化器 monorepo，不为目录整洁提前拆仓

**决定**：Arbor 继续作为个人工具孵化 monorepo。主容器、知识库、治理文档和未成熟工具留在本仓库。独立工具或库只有满足拆仓条件后，才迁出为独立 git 仓库。

**理由**：
- 当前仓库同时承担产品孵化、经验沉淀和项目治理职责，过早拆仓会切断经验提取和共用基础设施的演化路径。
- `capture`、`keydock`、`clipdock`、`memvfs` 等项目还在验证边界，放在同一仓库里更容易共享文档、模式和测试约定。
- `arbor-ui-core`、`arbor-ui-windows` 仍随 KeyDock/ClipDock 共同变化，单独拆库会让 API 过早固化。
- `skill-manager-core` 目前主要是规范，不是可发布实现，拆仓没有收益。

**拆仓条件**：
- 项目有独立用户或独立发布目标。
- 项目可以独立构建、测试和阅读 README。
- 项目不依赖 Arbor 私有 `workspace/` 数据。
- 未来数次迭代大概率不需要和 Arbor 主线一起修改。
- 拆仓能带来发布、复用、权限隔离或历史清晰度收益。

**当前分类**：
- 长期留在 Arbor：`apps/container`、`workspace/learn`、`workspace/manage`、`workspace/show`。
- 短期留在 Arbor，成熟后评估：`apps/capture`、`apps/keydock`、`apps/clipdock`、`apps/memvfs`。
- 跟随使用方演化：`packages/arbor-ui-core`、`packages/arbor-ui-windows`。
- 先作为规范沉淀：`packages/skill-manager-core`。

**代价**：
- 仓库会同时包含 TypeScript、Tauri、Rust native GUI 和 Rust daemon 项目。
- 根脚本和文档必须持续维护，否则入口文档容易落后于真实结构。

**执行规则**：
- 不为“目录干净”拆仓。
- 新增 app/package 时，必须说明它属于 Arbor 本体、孵化产品、技术样本、可复用库或纯经验沉淀。
- 拆仓前先补齐 README、构建命令、测试命令和最小验收记录。
- 具体状态表维护在 `workspace/manage/repo-strategy.md`。
