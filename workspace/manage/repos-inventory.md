# work-context repos/ 全量项目清单

最后更新：2026-06-07

---

## 三维度分类

每个项目按三个维度评估：

| 维度 | 含义 | 
|------|------|
| **📖 经验** | 有没有可抽取的架构/设计模式，写入 learn/patterns/ |
| **🔧 功能** | 代码本身能不能作为库/工具复用 |
| **📐 参考** | demo 是否展示了某种技术的最佳实践，值得保留作为参考实现 |

---

## work-context/repos/（26 个）

### 1. 2048-h5
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TSX + Vite | 65 | 小游戏组件拆分（app/canvas/game） | 无 | TSX 游戏架构 |

### 2. Aurora
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TS + Rust + TSX | 653 | monorepo 聚合模式 | kaubo-features + sysfolio 子模块 | — |

→ Arbor 保留列表中，不动

### 3. Clover
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TS (pnpm ws) | 299 | pnpm monorepo 工具链拆分 | `@clover.js/*` 6 个包：cli + eslint-config + eslint-plugin + protocol + std + tsconfig | JS 工具链框架的标准结构 |

→ 如果 Arbor 以后需要自建 eslint 配置体系，Clover 是直接可用的基础

### 4. CultivationBattle
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TSX + Vite + Playwright | 48 | 游戏 + E2E 测试的完整 CI 配置 | 无 | Vite + Playwright + ESLint flat config 的标准工程配置 |

### 5. dealpytool
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Python | 71 | ✅ domain/application 分层 + workflow orchestration | Python CLI 工具 | Python 项目分层架构模板 |

### 6. Ethereal
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Rust | 102 | ✅ monorepo 依赖图 + import 扫描 + adapter trait → `monorepo-dependency-graph.md` | 可直接作为 Arbor 的构建编排层 | Rust 工程：SWC、SQLite、capability 校验 |

### 7. FlowX
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| — | 12 | 无 | 无 | 无（仅 style.md 设计文档） |

### 8. FluxBin
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TS (pnpm ws) | 326 | 多传输后端抽象（websocket/tcp）+ env 隔离 | `@fluxbin/*` 6 个包：core + client + transport-websocket + transport-tcp + devtools + env-* | 传输层抽象 + 多环境构建 |

→ 与 ObolosFS 的 Driver 抽象同一种模式，具体应用在数据传输领域

### 9. forgbench
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Rust | 17 | CLI 工具结构 | Codex skill CLI workbench | Rust CLI 工具最小模板 |

### 10. hello-rust
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Rust | 5 | 无 | 无 | 无（hello world） |

### 11. jue
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TS | 422 | ✅ 响应式引擎 + VDOM → `minimal-reactivity-vdom.md` | 无（Vue/SolidJS 已有更好实现） | 理解响应式框架原理的最小代码量参考 |

### 12. multimedia-platform-planning
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| 文档 | 7 | 无 | 无 | 无（纯规划文档） |

### 13. my-android-demo
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Kotlin/Java | 9224 | 无（脚手架生成） | 无 | Android 项目结构 |

### 14. my-miniprogram-demo
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TS | 40 | 无 | 无 | 微信小程序 TS 工程结构 |

### 15. nexus
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| 文档 | 54 | 无 | 无 | 无（文档项目） |

### 16. node_modules
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| — | 0 | — | — | 空目录，直接删除 |

### 17. Pulse
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| 文档 + 少量 TS | 61 | 无 | 无 | 无（文档/规划项目） |

### 18. react_ventus
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| — | 0 | — | — | 空目录，直接删除 |

### 19. rn083
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TS | 1662 | React Native Fabric 深度使用 | 无 | React Native 项目结构 |

→ Arbor 保留列表中，不动

### 20. shamrock
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Rust (8 crates) | 78 | workspace 拆分（data/core/mechanics/format/view/replay/cli） | 宝可梦对战引擎 | Rust workspace 多 crate 结构 |

→ 自研游戏引擎的基础代码

### 21. ShapeBin
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Java | 456 | 无（含 .class 编译产物） | Android 应用 | 无 |

### 22. sysfolio
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TSX + Vite | 441 | 作品集前端 + design 文档 | Portfolio 前端代码 | Vite + TSX + Vitest + Playwright 完整前端工程 |

→ 透出分支的前端基础代码

### 23. tdd-demo
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Rust | 15 | 无 | 无 | Rust 项目的 TDD 测试结构 |

### 24. tetris-h5
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| HTML/JS | 80 | 无 | 俄罗斯方块游戏 | Canvas 游戏架构 |

### 25. Untitled
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Astro + TS | 88 | ✅ SSG + content collections + ContentNode → `astro-file-tree-portfolio.md` | 文件树多媒体作品集 | Astro SSG + 多媒体嵌入 |

→ Arbor Phase 4 静态站点导出的直接参考

### 26. url-parser-bench
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Rust + Haskell | 18 | 无 | URL 解析性能对比 | 跨语言 benchmark 结构（fixtures/ + scripts/ + 各语言独立目录） |

### 27. vscode-theme
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| JSON | 14 | 无 | VS Code 主题 | VS Code 主题定义结构 |

---

## work-context-2/repos/（4 个）

### 28. jue
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TS | 9 | 同 #11，更小的副本 | 无 | 无 |

### 29. Lumina
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| 文档 | 300 | 无（全为 .omx agent 日志） | 无 | agent 协作的项目规划文档 |

### 30. rust-web-demo
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| Rust + TS | 69 | Rust + npm（rslib）混合构建 | 无 | Rust + TypeScript 混合项目的工程配置（rslib + benchmarks） |

### 31. vscode-git-commit-plugin
| 语言 | 文件 | 📖 经验 | 🔧 功能 | 📐 参考 |
|------|------|---------|---------|---------|
| TS | 7 | 无 | VS Code 插件（空壳） | 无 |

---

## 汇总

### 📖 已抽经验（5 个）
Ethereal、Untitled、dealpytool、jue、FluxBin（FluxBin 的传输层抽象与 ObolosFS Driver 同模式，记录在案）

### 🔧 可复用库/工具（7 个）
FluxBin、Clover、Ethereal、shamrock、forgebench、sysfolio、Untitled

### 📐 最佳实践参考（6 个）
CultivationBattle（Vite+Playwright 工程配置）、2048-h5（TSX 游戏结构）、tdd-demo（Rust TDD）、rust-web-demo（Rust+npm 混合构建）、url-parser-bench（跨语言 benchmark）、tetris-h5（Canvas 游戏）

### 🗑 纯 demo/模板/空壳/文档（13 个）
FlowX、hello-rust、multimedia-platform-planning、my-android-demo、my-miniprogram-demo、nexus、node_modules、Pulse、react_ventus、ShapeBin、vscode-theme、vscode-git-commit-plugin、Lumina

### 🔒 保留不动（2 个）
Aurora、rn083
