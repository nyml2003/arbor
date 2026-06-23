# Plan — 迭代路线图

## 总览

Arbor 现在按两条线推进：

- **主线**：把个人产出引擎跑起来。容器、经验沉淀、管理和展示都服务这条线。
- **孵化线**：在 `apps/` 和 `packages/` 下孵化独立工具。成熟后再评估是否拆独立 git 仓库。

| Phase | 目标 | 当前状态 |
|-------|------|----------|
| 1 | 容器应用 | 已有 Electron + SolidJS 容器和文件树体验，继续补 Markdown 预览和展示能力 |
| 2 | 管理工具 | 仍处于文档和任务清单阶段 |
| 3 | 沉淀工具 | 已有 `workspace/learn` 知识库和 pattern 索引，继续依赖 agent 维护 |
| 4 | 引擎闭环 | 已有简历展示数据和 web 构建入口，静态站点导出未完成 |

---

## 仓库治理策略

当前默认策略是孵化器 monorepo：

- `apps/container`、`workspace/*` 是 Arbor 本体，长期留在本仓库。
- `apps/capture`、`apps/keydock`、`apps/clipdock`、`apps/memvfs`、`apps/aster`、`apps/shamrock` 是孵化项目，先留在本仓库。
- `packages/arbor-ui-*`、`packages/skill-manager-core` 是可复用基础设施，先跟随使用方一起演化。
- 只有当一个项目有独立用户、独立发布节奏、独立构建测试，并且不再依赖 Arbor 私有数据时，才拆独立 git 仓库。

具体拆仓规则和项目状态记录在 `workspace/manage/repo-strategy.md`。

---

## Phase 1：容器应用

**目标**：一棵能看的文件树，四个空分支，在 Electron 桌面窗口里。

**输入**：
- 本目录下的所有文档（vision、plan、decisions、conventions）
- WatchDesk 项目的架构模式作为参考
- 用户偏好：Electron + SolidJS + pnpm workspace

**产出**：
- Electron 桌面应用，左侧文件树，右侧内容区
- 文件树显示四个分支：`build/`、`learn/`、`manage/`、`show/`
- 点击分支节点切换右侧内容
- 文件树读取本地文件系统的一个工作区目录

**不做**：
- 复杂的插件系统、主题、设置面板
- 高级动画、拖拽、右键菜单
- 网络相关功能

**验收标准**：
- 打开应用，能看到文件树
- 四个分支存在（哪怕是空文件夹）
- 点击树节点，右侧内容区有反应
- 可以 `pnpm install && pnpm dev` 跑起来

**技术路线**：
- 直接迁移 WatchDesk 的 Electron + SolidJS 架构（复制代码，不做重搭）
- pnpm workspace（monorepo）
- 分层：shell（Electron 主进程）、renderer（SolidJS 渲染）、contract（共享类型）
- 把 WatchDesk 容器壳改造为 Arbor 的文件树形态（四个分支）

**完成后清理**：
- 归档 Aura（博客展示逻辑被 Arbor 容器取代）
- 归档 ventus（文档/规划阶段的产物，已融入 Arbor 文档）

---

## Phase 2：管理工具

**目标**：任务和项目管理，CLI 可用，容器内 GUI 可用。

**输入**：
- workshop (workc) 的领域模型和命令体系
- work-context-2 的 domain model 设计
- 用户的日常任务管理需求

**产出**：
- `@arbor/manage-core`（TS 包）：Task 实体、状态机、存储接口、业务逻辑
- `@arbor/manage-cli`（TS 包，npm bin 入口）：CLI 命令（create/list/update/complete/delete）
- 容器内「管理」分支的 GUI 面板：任务列表、创建/编辑、状态切换

**架构约束**：
- core 包零 UI 依赖，只做数据和逻辑
- CLI 壳调用 core，不做业务逻辑
- GUI 面板同样是 core 的壳
- 存储层用文件系统（JSON/YAML），不引入数据库

**验收标准**：
- CLI 可以 `arbor task create "做一件事"` 
- CLI 可以 `arbor task list` 看到任务列表
- 容器内管理面板能看到同样的任务
- 任务数据在文件系统的 `manage/` 目录下持久化

**完成后清理**：
- 删除 workshop（Rust workc，管理逻辑被接管）
- 删除 work-context（Python 技能工作台，被 Arbor 沉淀系统取代）
- 删除 work-context-2（Python 工作台，领域模型已迁移到 Arbor）
- 删除 tasks/、my-task-1/、skills/（workc 任务空间，不再需要）

---

## Phase 3：沉淀工具（agent-first，轻代码）

**目标**：捕获经验、写笔记、形成可检索的知识库。**不写大量代码，依赖 agent 能力。**

**核心思路**：
- 笔记就是 `learn/` 目录下的纯 Markdown 文件，文件系统即数据结构
- 组织、搜索、标签建议、模式提取 → 交给 Claude/agent
- 容器内的沉淀面板 = 文件树 + Markdown 预览，不需要复杂的编辑/标签系统

**输入**：
- Phase 2 过程中自然产出的 TS 架构经验
- rn084 的深度技术文档模式（FABRIC_DEBUG_LEARNING.md 等）

**产出**：
- `learn/` 目录下的 Markdown 笔记模板和初始内容
- 容器内「沉淀」分支的轻量面板：文件树浏览 + Markdown 渲染预览
- agent 可用的沉淀操作指引（如何组织笔记、如何检索、如何提取模式）

**不做**：
- `@arbor/knowl-core` / `@arbor/knowl-cli` 等独立包
- 标签系统、全文搜索引擎
- 复杂的笔记编辑功能

**验收标准**：
- 容器内能浏览 `learn/` 目录下的 Markdown 文件并预览
- agent 能够对沉淀目录进行搜索、总结、归类
- 笔记数据在 `learn/` 目录下以纯 Markdown 组织

**做完后引擎状态**：
- 管理工具能管任务 → 沉淀工具 agent 能记经验 → 迭代工具能做东西 → 透出能展示

---

## Phase 4：引擎闭环

**目标**：四个分支都有实质内容，引擎自转，静态站点可导出。

**产出**：
- 容器导出为纯静态站点（文件树 → 可交互网页）——这就是你说的「文件系统风格的多媒体作品集」
- CLI 有统一的入口命令 `arbor`
- 四个分支都有实际内容落地
- 透出分支挂上导出后的站点链接

**验收标准**：
- 运行一个命令可以生成静态站点
- 站点上有文件树，可浏览，可点击
- 别人打开链接就能看到你的工具和作品
- 引擎能跑通完整的四环节循环

**完成后清理**：
- 删除 WatchDesk（被 Arbor 容器完全取代）
- 删除 ventus-（博客替代前端，被 Arbor 透出取代）

---

## Build 域并行孵化：截图工具

**目标**：做一个独立的桌面截图工具。默认截图后复制到剪贴板，弹系统通知，点击通知时用系统默认图片查看器打开缓存图。

**定位**：
- 它是 build 域的独立工具，不是 Arbor 容器里的一个页面
- 代码放在 `apps/capture/`
- 技术路线：`Tauri 2 + Rust-first + 静态 overlay + 延迟加载的 SolidJS settings`

**当前阶段**：
- 已完成文档、目录骨架和空壳初始化
- 还没有进入真实的截图、剪贴板、通知和缓存实现
- 短期继续留在 Arbor；v1 主链路跑通后优先评估拆独立仓库

**v1 不做**：
- CLI
- 标注
- OCR
- 录屏
- 历史资料库

---

## Build 域技术储备：Rust Native GUI

**目标**：把 KeyDock 中验证出的 `Rust DSL + 平台适配层` 沉淀为 Arbor 的第二套 GUI 储备线。

**定位**：
- 它是 GUI 技术储备，不是 Phase 1 容器路线的替代
- 它适合系统工具、小窗口、常驻工具、原生输入和高性能交互
- `apps/container/` 仍然使用 Electron + SolidJS

**当前阶段**：
- KeyDock 是第一份原型资产
- KeyDock 和 ClipDock 已经共用 `packages/arbor-ui-core`、`packages/arbor-ui-windows`
- 已验证安全 Rust app 层、组件 DSL、primitive tree、Win32/Direct2D 渲染、剪贴板和 `SendInput` 边界
- 继续在本仓库沉淀模式，暂不把 `arbor-ui-*` 拆独立库

**下一步触发条件**：
- 两个以上 native GUI 工具持续依赖同一套稳定 API
- KeyDock/ClipDock 的组件模型继续稳定
- 平台适配层边界扫描持续通过

---

## Build 域技术实验：memvfs

**目标**：验证 Rust in-memory VFS、daemon 和 CLI 的分层。

**定位**：
- 它是系统工具实验，不是 Arbor 主容器能力。
- `memvfs-core` 负责纯内存模型。
- `memvfsd` 负责保持一个 daemon 进程。
- `memvfs-cli` 通过 localhost TCP JSON 协议访问 daemon。

**当前阶段**：
- 已有 core/daemon/cli workspace。
- 已有基础 POSIX-like 语义：目录、文件、inode、block、读写、rename、unlink、stat。
- 短期继续留在 Arbor；只有当它变成日常工具或可复用库时再拆仓。

---

## Build 域实用工具：Aster

**目标**：提供一个本地 agent CLI，先只调用 DeepSeek API 完成日常问答。

**定位**：
- 它是本地 AI CLI 孵化工具，不是 Arbor 主容器能力。
- 当前版本负责把命令行 prompt 转成 DeepSeek chat completion 请求，并支持进程内连续对话。
- 不读取本地文件，不执行命令，不保存会话。

**当前阶段**：
- 已有 TypeScript CLI、参数解析、DeepSeek HTTP 调用、流式输出、终端 Markdown 渲染、本地 skill 注入和包级测试。
- 默认模型使用 `deepseek-v4-flash`。
- 后续再评估是否增加持久化会话、多模型配置或本地工具能力。

---

## Build 域游戏引擎实验：Shamrock

**目标**：孵化一个 Rust 宝可梦对战模拟引擎，先保留可测试、可回放、可扩展的 1v1 单打核心。

**定位**：
- 它是独立 Rust workspace 孵化项目，不是 Arbor 主容器内置页面。
- `battle-core`、`battle-data`、`battle-format`、`battle-mechanics`、`battle-view`、`battle-replay`、`battle-cli` 保持单向依赖边界。
- 后续如果接入 `apps/container`，优先通过 `battle-view` 的 view-model 和 replay JSON 边界接入，不让 UI 直接依赖 core 内部状态。

**当前阶段**：
- 已从 `work-context/repos/shamrock` 迁入 `apps/shamrock`。
- 已保留 Gen1 demo 数据包、golden replay 和 CLI。
- 本次迁移不修改 container，不新增 IPC。

---

## Build 域基础设施：Skill 管理器

**目标**：实现 agent skill 的安装、校验和 lockfile 规范。

**定位**：
- 它管理的是 agent skill 工作流包，不是 npm、uv 或 Maven 语言包。
- v1 模型是 `SourceSkill -> SkillPackage -> InstalledSkill`。
- `packages/skill-manager-core` 承载领域逻辑，`packages/skill-manager-cli` 承载 CLI 壳。

**当前阶段**：
- 已写清 `arbor.skills.json`、`arbor.skills.lock.json`、`skill.package.json` 的职责。
- 已实现 TypeScript core/cli v1：path source、严格版本校验、非受管 Skill 元数据生成、copy 安装、content hash、lockfile、prune。
- 已新增 `packages/arbor-skills` 作为 Arbor 自维护 Skill 集合，用它试运行本地 path source 安装。
- Git、tarball、npm source 仍停留在规范和校验层，后续按同一 source port 接入。
- 不拆仓。等真实安装场景稳定后再评估。

---

## 保留项目（后续看情况纳入 Arbor）

| 项目 | 原因 | 可能的 Arbor 归属 |
|------|------|------------------|
| openclaw | AI 网关，体量太大，不动先 | 迭代域：自动化工具 |
| edict | 依赖 openclaw 的智能体系统 | 迭代域：智能体编排 |
| kaubo / kaubo-features | 自研语言引擎，活跃开发中 | 迭代域：语言工具 |
| GearBox | Rust 跨平台框架 | 迭代域：框架基础设施 |
| ObolosFS | TS 文件系统抽象库 | 迭代域 / 框架 |
| zipfiles | C++ 文件备份，归档参考 | 不纳入，参考用 |
| common-event-bus | C++ 事件总线，归档参考 | 不纳入，参考用 |
| SillyTavern-Launcher | AI 前端启动器，工具性质 | 迭代域：实用工具 |
| my-expo-app | Expo 移动端入门 | 迭代域：移动宿主 |
| rn-demo / rn084 | RN 学习项目 | 沉淀域：学习笔记 |
