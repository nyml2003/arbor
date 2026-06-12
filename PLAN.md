# Plan — 迭代路线图

## 总览

| Phase | 目标 | 估算 |
|-------|------|------|
| 1 | 容器应用 | 文件树桌面壳 |
| 2 | 管理工具 | 任务/项目管理 |
| 3 | 沉淀工具 | 笔记/经验捕获 |
| 4 | 引擎闭环 | 导出 + 四分支实内容 |

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

**v1 不做**：
- CLI
- 标注
- OCR
- 录屏
- 历史资料库

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
