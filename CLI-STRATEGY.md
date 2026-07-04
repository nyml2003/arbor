# CLI Strategy — 跨平台 CLI 工具选型框架

## 为什么需要这份文档

Arbor 是一个个人工具孵化 monorepo，已经有 TypeScript、Rust、Python 三种语言在不同工具中共存。每个新 CLI 工具都会面临同样的选型问题：用什么语言、怎么分发、跨平台怎么做。

**没有银弹。** 不同的工具面向不同用户、有不同的性能要求和运行时约束。这份文档不是给出一刀切的答案，而是提供一个决策框架，让每个新项目能有意识地做取舍。

---

## 决策维度

做 CLI 工具选型时，需要依次回答以下问题。每个问题的答案都会缩小选择范围。

### 维度 1：目标用户有没有运行时？

```
用户是谁？
├── 只有我自己 → 已安装什么就用什么，分发不是问题
├── 开发者同事 → Node.js / Python / Homebrew 都可能已经装了
└── 非开发者 / 服务器 → 必须给单二进制，Go 或 Rust
```

**Arbor 的默认答案：大部分工具只有我自己用，运行时不是瓶颈。** 这就是为什么 Decision 6 选了 npm bin 脚本而非独立二进制。

但当工具要给别人用时（比如静态站点导出给面试官看），就需要重新评估。

### 维度 2：启动速度和内存重要吗？

| 场景 | 启动要求 | 举例 |
|------|---------|------|
| 常驻/daemon | 不重要 | memvfsd、netmon |
| 偶尔运行 | 不太重要（<500ms 即可） | task create、battle-cli |
| 高频调用（shell 别名/管道） | **重要**（<50ms） | 暂无，但未来可能有 |
| 交互式 REPL | 不重要（只启动一次） | aster chat |

Python 启动 ~50-200ms，Node.js ~30-100ms，Go/Rust <5ms。只有高频调用场景才需要编译语言。

### 维度 3：需要什么生态能力？

| 需求 | 推荐栈 |
|------|--------|
| HTTP 客户端 / JSON 处理 | Go（标准库）、Node.js（fetch）、Python（requests） |
| 文件系统操作 | 三者都可以，Go 的 `filepath` 跨平台最好 |
| 子进程管理 | Node.js（`child_process`）、Python（`subprocess`） |
| 终端 UI（颜色/进度条/交互） | Python（Rich/Typer）体验最好，Rust（ratatui）性能最好 |
| AI/LLM 调用 | Node.js（Anthropic SDK）、Python（openai） |
| 系统级操作（剪贴板/输入/窗口） | Rust 独占 |
| 跨平台 GUI | Electron（TS）、Tauri（Rust） |

### 维度 4：分发方式是什么？

| 分发方式 | 适用场景 | Arbor 已有案例 |
|---------|---------|--------------|
| npm bin (`package.json` `"bin"`) | 自己和 JS 开发者 | `manage-cli`、`skill-manager-cli`、`aster` |
| `python script.py` / `python kaubo-ops` | 自己用，Python 项目 | `netmon`、`kaubo-features` |
| Rust `cargo build --release` → 单二进制 | 性能敏感、要分发 | `shamrock`、`memvfs` |
| `pipx install` / PyPI | 给 Python 开发者 | 暂无 |
| Homebrew / Scoop / 直接下载二进制 | 给任何人 | 暂无 |

---

## 三种范式对比（Arbor 专有视角）

### TypeScript + npm bin（当前默认）

**代表**：`manage-cli`、`skill-manager-cli`、`aster`

| 优点 | 缺点 |
|------|------|
| 和 monorepo 工具链统一（pnpm、SWC、tsc、Vitest） | 必须装 Node.js 24+ |
| 零额外分发成本——`pnpm install` 就有 CLI | 启动 ~30-100ms |
| 可以直接 import workspace 内部包 | `process.platform` 判断是手动挡 |
| ESM + strict TypeScript 类型安全 | 不适合给非 JS 开发者用 |

**什么时候选这个**：
- 工具是 Arbor monorepo 的一部分
- 需要引用 `@arbor/*` workspace 包
- 目标用户（自己）有 Node.js 环境
- 不需要系统级能力（剪贴板、输入钩子、原生窗口）

**跨平台注意事项**：
```
路径：path.join() / path.relative()，持久化统一 POSIX
编码：始终显式 utf-8
信号：避免 SIGALRM，用 AbortController/timeout
换行：.gitattributes 强制 LF
颜色：检测 process.stdout.isTTY 和 FORCE_COLOR
```

### Rust + 单二进制

**代表**：`shamrock`、`memvfs`、`keydock`、`clipdock`

| 优点 | 缺点 |
|------|------|
| 单文件分发，零运行时依赖 | 编译慢，迭代不快 |
| 启动 <5ms，内存最小 | 学习曲线陡 |
| 系统级能力（Win32 API、Direct2D、剪贴板） | 和 monorepo TS 工具链不互通 |
| `clap` 的 derive API 编译时验证参数 | 跨平台构建需要 CI matrix |

**什么时候选这个**：
- 需要系统级能力（窗口、剪贴板、输入、GPU）
- 性能敏感（游戏引擎、文件系统 daemon）
- 要分发给没有运行时的用户
- 项目本身用 Rust（kaubo、shamrock）

**跨平台注意事项**：
```
路径：std::path::Path / PathBuf，持久化用 POSIX
编码：String 默认 UTF-8，OsString 用于 OS 路径
信号：用 tokio::signal 或 ctrlc crate
临时文件：tempfile crate 处理平台差异
构建：CI 跑 windows/macos/linux 三个 target
分发：goreleaser（Go）或 cargo-dist（Rust）自动多平台发布
```

### Python 脚本

**代表**：`netmon`、`wifi-finder`、`kaubo-ops`

| 优点 | 缺点 |
|------|------|
| 开发最快——改完即跑，无编译 | 必须有 Python 3.x |
| 生态最全（网络、AI、数据分析） | 版本管理地狱（pyenv/venv/pip） |
| 纯 stdlib 可以零依赖 | 分发最麻烦（PyInstaller 打包 30-80MB） |
| `pathlib` 跨平台路径处理内置 | GIL 限制真并行 |
| `subprocess` 跨平台进程管理 | 启动比 Node/Go/Rust 都慢 |

**什么时候选这个**：
- 快速原型、一次性脚本
- Python 生态独占能力（AI/ML、网络分析）
- 纯 stdlib、零外部依赖的小工具
- Kaubo 项目内部工具链

**跨平台注意事项**：
```
路径：只用 pathlib.Path，永远不要字符串拼接
编码：open() 显式传 encoding="utf-8"
       PYTHONUTF8=1 环境变量
       subprocess.run(..., encoding="utf-8", errors="replace")
信号：Windows 上无 fork()，用 subprocess 替代
      SIGALRM 不可用，用 threading.Timer
换行：open(newline="\n") 统一 LF
颜色：检测 sys.stdout.isTTY，Windows 旧终端不支持 ANSI
```

---

## 新增 CLI 工具 Checklist

开新 CLI 工具时，过一遍这个清单：

```
□ 目标用户是谁？他们有运行时吗？
□ 性能要求：高频调用（<50ms）还是偶尔跑一次？
□ 需要什么生态能力？（HTTP/文件/终端/系统/LLM）
□ 分发方式：npm bin / 脚本 / 单二进制 / 其他？
□ 和 Arbor 现有工具链的关系？（需要引用 workspace 包吗？）
□ 偏好的语言？（团队/个人熟悉度）
```

然后对照三种范式的优缺点，做有意识的取舍。接受选型的代价——不是每个工具都需要完美。

最终在 `DECISIONS.md` 加一条决策记录即可，格式参考现有条目：

```markdown
## N. [工具名] 选 [语言/范式]，因为 [核心原因]

**决定**：...

**理由**：
- ...

**代价**：
- ...

**不适用的替代方案**：
- [方案 A]：[为什么不用]
- [方案 B]：[为什么不用]
```

---

## 业界参考

### 大厂 CLI 选型规律

| 工具 | 语言 | 范式 | 为什么 |
|------|------|------|--------|
| kubectl | Go | Cobra + 单二进制 | 要跑在服务器上，零依赖 |
| Docker CLI | Go | Cobra + 单二进制 | 同上 |
| GitHub CLI (`gh`) | Go | Cobra + 单二进制 | 跨平台一键安装 |
| Terraform | Go | 单二进制 | DevOps 工具标配 |
| AWS CLI v2 | Python | PyInstaller 打包 | 历史包袱，v1 就是 Python |
| Vercel CLI | Node.js | 单二进制（pkg 打包） | 目标用户有 Node |
| Claude Code | Node.js | npm 分发 | 目标用户有 Node |
| uv | Rust | 单二进制 | Python 包管理器，必须零依赖快速启动 |
| bun | Zig | 单二进制 | JS 运行时，必须零依赖 |
| ripgrep | Rust | 单二进制 | 性能敏感（全文搜索） |

### CLI 设计原则十二条（CLIG 精简版）

1. **stdout 给数据，stderr 给诊断**——这是"能被脚本用"的分界线
2. **`--help` 要有用法示例**——不只是 flag 列表
3. **`--version` + 语义化版本号**
4. **`--json` / `--quiet` / `--no-color`**——给脚本消费的标准 flag
5. **TTY 检测**——管道时自动关颜色、关进度条
6. **配置优先级**：CLI args > 环境变量 > 配置文件 > 默认值
7. **退出码可依赖**：0 = 成功，1 = 运行时错误，2 = 参数错误
8. **错误信息告诉用户正确答案**——不只说"错了"，说"应该是什么"
9. **子命令 > 互斥 flag**——`mycli backup create` 比 `mycli --backup-create` 好
10. **不强制交互**——管道/CI/cron 里也能用
11. **Shell 补全生成**——bash、zsh、fish、PowerShell
12. **XDG 规范配置路径**：Linux `~/.config/app/`、macOS `~/Library/Application Support/app/`、Windows `%APPDATA%\app\`

### 交叉编译速查

| 语言 | 交叉编译命令 |
|------|------------|
| Go | `GOOS=linux GOARCH=amd64 go build` |
| Go（全静态） | `CGO_ENABLED=0 GOOS=linux go build` |
| Rust | `cargo build --target x86_64-unknown-linux-musl`（需要先 `rustup target add`） |
| Rust（用 cross） | `cross build --target x86_64-unknown-linux-musl` |
| Deno | `deno compile --target x86_64-unknown-linux-gnu main.ts` |
| Zig | `zig build -Dtarget=x86_64-linux-musl`（自带交叉编译器，不需要额外安装） |

---

## 当前 Arbor CLI 分布

| 工具 | 语言 | 范式 | 入口 |
|------|------|------|------|
| `arbor-manage` | TypeScript | npm bin | `packages/manage-cli/dist/cli.js` |
| `skill-manager` | TypeScript | npm bin | `packages/skill-manager-cli/` |
| `aster` | TypeScript | npm bin | `apps/aster/dist/cli.js` |
| `netmon` | Python | 脚本 | `apps/netmon/netmon.py` |
| `wifi-logger` | Python | 脚本 | `apps/wifi-finder/wifi_logger.py` |
| `kaubo-ops` | Python | 脚本 | `packages/kaubo-features/kaubo-ops/` |
| `shamrock` battle-cli | Rust | cargo | `apps/shamrock/` |
| `memvfs-cli` | Rust | cargo | `apps/memvfs/` |
| keydock / clipdock | Rust | cargo（GUI） | `apps/keydock/`、`apps/clipdock/` |

**规律**：当前选择基本落在三句话里：
- TypeScript → 管理类工具，要引用 workspace 包，npm bin 分发
- Python → 网络/系统脚本，纯 stdlib 零依赖，快速原型
- Rust → 系统工具、游戏引擎、GUI，需要零依赖或系统级能力

**规律没有坏掉**。所有选择都是按"目标用户 × 性能要求 × 生态能力 × 分发方式"四个维度做的。

---

## 维护规则

- 新增 CLI 工具时，在 `DECISIONS.md` 加一条决策，引用本文档的维度框架
- 发现新的跨平台坑时，更新本文档对应范式的注意事项
- 如果某个范式的使用场景变了，更新"什么时候选这个"
- 每半年重新审视一次现有 CLI 分布表，看是否有需要迁移/合并的
