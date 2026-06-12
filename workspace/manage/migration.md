# Migration — 旧项目 → Arbor 迁移表

## 映射总表

旧项目（C:\Users\nyml\code 和 D:\code）按其逻辑基因映射到 Arbor 的四个环节。

### C:\Users\nyml\code

| 旧项目 | 主语言 | 逻辑基因 | Arbor 归属 | 处理 |
|--------|--------|---------|-----------|------|
| WatchDesk | TS | Electron+SolidJS 容器架构 | 透出/迭代 | Phase 4 后删除 |
| Aura | TS/Node | SSR 模板、monorepo 模式 | 透出 | Phase 1 后归档 |
| ventus | Go(规划) | BFF 设计、静态生成链 | 透出/沉淀 | Phase 1 后归档 |
| workshop | Rust | 任务模型、CLI 命令体系 | 管理 | Phase 2 后删除 |
| work-context | Python | 技能管理、校验、打包 | 沉淀 | Phase 2 后删除 |
| work-context-2 | Python | 六边形架构、领域模型 | 管理/沉淀 | Phase 2 后删除 |
| tasks | workc | 多任务 workspace | 管理 | Phase 2 后删除 |
| my-task-1 | workc | 任务 workspace 示例 | 管理 | Phase 2 后删除 |
| skills | workc | 技能挂载配置 | 沉淀 | Phase 2 后删除 |
| knowledge-candidates | — | 空占位 | 沉淀 | Phase 3 后删除 |
| materials | — | 空占位 | — | 删除 |
| repos | — | 空占位 | — | 删除 |
| edict | Python/TS | 智能体编排、状态流转 | 迭代（AI 域） | 保留，后入 |
| openclaw | TS | AI 网关、路由、插件系统 | 迭代（AI 域） | 保留，后入 |
| kaubo | C++ | 编译器/运行时引擎 | 迭代（语言域） | 保留，后入 |
| kaubo-features | C++/Rust | GC、Lexer、Rust 重写 | 迭代（语言域） | 保留，后入 |
| GearBox | Rust | 跨平台框架抽象 | 迭代/框架 | 保留，后入 |
| ObolosFS | TS | 文件系统抽象 | 迭代/框架 | 保留，后入 |
| egui | Rust | GUI 模板 | — | 归档 |
| jue | TS | 自研 VDOM 实验 | 沉淀（模式参考） | 归档 |
| SillyTavern-Launcher | Shell | AI 前端启动器 | 迭代 | 保留 |
| my-expo-app | TS | Expo/RN 移动模板 | 迭代（移动域） | 保留 |
| rn-demo | TS | RN 版本对比 | 沉淀 | 保留 |
| rn084 | TS | Fabric 深度学习 | 沉淀 | 保留 |
| dll_csv_transformer | C++ | DLL 演示 | — | 归档 |
| my-mh | C# | 进程查找 | — | 归档 |
| toml_resume | Rust | 简历生成 | — | 归档 |
| hub | Java | Spring Boot 后端 | — | 归档 |
| untitled ~ untitled3 | Kotlin | 实验 | — | 归档 |

### D:\code

| 旧项目 | 主语言 | 逻辑基因 | Arbor 归属 | 处理 |
|--------|--------|---------|-----------|------|
| ventus | Go/React | 博客系统 | 透出 | 归档 |
| ventus- | TS/SolidJS | 博客替代前端 | 透出 | Phase 4 后删除 |
| WatchDesk | TS | (同上) | — | — |
| project/blog_back | Python/JS | Django+Vue 博客 | 透出 | 归档 |
| project/next-ventus-container | TS | pnpm 博客 monorepo | 透出 | 归档 |
| project/resume | TS/React | 简历/展示站 | 透出 | 归档 |
| project/deneb | TS/Vue | 微前端壳 | — | 归档 |
| project/visual-kaubo | TS/Vue | 代码可视化 | 迭代 | 保留参考 |
| project/vue-tsx-tailwind | TS/Vue | Vue3 模板 | — | 归档 |
| project/changfen (Sed) | C++ | SysY 编译器 | 沉淀（编译技术参考） | 归档 |
| project/tensorslow_release | C++ | Python 解释器 | 沉淀 | 归档 |
| project/others/Porkchop | C++ | 自研语言 | 沉淀 | 归档 |
| project/others/my_toy_compiler | C++ | LLVM 编译器 | 沉淀 | 归档 |
| project/others/pythonvm | C++ | Python VM | 沉淀 | 归档 |
| project/others/MatrixSlow | Python | ML 框架 | — | 归档 |
| project/demo-nest | TS | NestJS 演示 | — | 归档 |
| project/koa-backend | TS | Koa 后端 | — | 归档 |
| project/game | Python | Django 游戏 | — | 归档 |
| project/java | Java | Java 实验 | — | 归档 |
| common-event-bus | C++ | 事件总线 | 沉淀（模式参考） | 归档 |
| zipfiles | C++ | 文件备份 | 沉淀（模式参考） | 归档 |
| uemerald-memhack | C++/Qt | 内存修改 | — | 归档 |
| my-mh (D:) | C++ | 内存修改 | — | 归档 |
| porygon | Rust | 早期 Rust 项目 | — | 归档 |
| rust-script | Rust | 脚本工具 | — | 归档 |
| rustify_web | Rust | TCP 应用 | — | 归档 |
| quickjs | C | JS 引擎 | — | 归档 |
| visual-frame | TS/Vue | 虚拟滚动 | — | 归档 |
| ai | Python | ML 实验 | — | 归档 |
| js | TS | JS 工具 | — | 归档 |
| leetcode | TS | 算法题 | — | 归档 |

## 按 Phase 的删除/归档计划

| Phase | 删除 | 归档（移走/标记） |
|-------|------|------------------|
| 1 | — | Aura、ventus(C:)、project/blog_back、project/next-ventus-container、project/resume、project/deneb、project/vue-tsx-tailwind |
| 2 | workshop、work-context、work-context-2、tasks、my-task-1、skills | materials、repos |
| 3 | knowledge-candidates | egui、jue、dll_csv_transformer、my-mh(C:)、toml_resume、hub、untitled~3、各种 D:\code 的非核心项目 |
| 4 | WatchDesk、ventus- | — |

## 保留项目（后续决定是否纳入 Arbor）

这些项目暂不处理，等 Arbor 引擎跑通后再评估：

- openclaw + edict（AI 域）
- kaubo + kaubo-features（语言域）
- GearBox（框架域）
- ObolosFS（框架域）
- SillyTavern-Launcher（工具域）
- my-expo-app（移动域）
- rn-demo + rn084（学习域）
- project/visual-kaubo（可视化域）
