# Punctum / VSH S0 验证记录

- 状态：`S0 Workspace Ready` 已通过
- 独立复核：`APPROVE`
- 当前阶段：`S0` 已完成，`F1` 未开始
- 验证日期：2026-07-11
- 唯一 writer：Program Integration Agent

## 完成范围

- 创建 Punctum、VSH、Game 和 Chater 四个独立 Cargo workspace。
- 创建 16 个架构计划内的最小 library crate 空壳。源码只有边界说明和 `#![forbid(unsafe_code)]`，没有业务 symbol。
- 为四个 workspace 生成各自根目录下的 `Cargo.lock`。
- Game 和 Chater 的跨 workspace path dependency 只定义在消费方 root `[workspace.dependencies]`；成员只使用 `{ workspace = true }`。
- 在 [`records.json`](./records.json) 中记录 canonical path 表、四份初始 baseline、`BATTLE-RULES-v0.1` approval slot 和 `GPU-REF-v0.1`。
- 提供默认只读的 [`Test-S0.ps1`](./Test-S0.ps1)，复算 metadata、path、lockfile、baseline 和 git scope。

未创建 Arbor 根 `Cargo.toml`、umbrella workspace 或第五个集成 workspace。未实现 grid、input、VSH、Battle、GPU、Terminal 或 Chater 业务逻辑。

## 实际命令与 exit code

四个 lockfile 分别通过以下命令生成，exit code 均为 0：

```text
cargo generate-lockfile --manifest-path apps/punctum/Cargo.toml
cargo generate-lockfile --manifest-path packages/vsh/Cargo.toml
cargo generate-lockfile --manifest-path apps/gen3-game/Cargo.toml
cargo generate-lockfile --manifest-path apps/tui-chater/Cargo.toml
```

Program Integration Agent 和独立只读 verifier 都对四个 manifest 分别执行了以下命令，8 次调用的 exit code 均为 0：

```text
cargo metadata --locked --format-version 1 --manifest-path apps/punctum/Cargo.toml
cargo metadata --locked --format-version 1 --manifest-path packages/vsh/Cargo.toml
cargo metadata --locked --format-version 1 --manifest-path apps/gen3-game/Cargo.toml
cargo metadata --locked --format-version 1 --manifest-path apps/tui-chater/Cargo.toml
```

四个 workspace 分别执行完整静态空壳验证，以下每种命令各 4 次，最终 16 次调用的 exit code 均为 0：

```text
cargo check --workspace --all-targets --locked --manifest-path <manifest>
cargo fmt --all --manifest-path <manifest> -- --check
cargo clippy --workspace --all-targets --locked --manifest-path <manifest> -- -D warnings
cargo test --workspace --all-targets --locked --manifest-path <manifest>
```

需要 target 的命令使用以下四个 task-unique absolute `CARGO_TARGET_DIR`；验证后已清理临时产物，不进入 git diff：

```text
C:\Users\nyml\code\arbor\.target\tasks\s0\program-integration\punctum
C:\Users\nyml\code\arbor\.target\tasks\s0\program-integration\vsh
C:\Users\nyml\code\arbor\.target\tasks\s0\program-integration\gen3-game
C:\Users\nyml\code\arbor\.target\tasks\s0\program-integration\tui-chater
```

只读聚合验证命令由 writer 和独立 verifier 分别执行，exit code 均为 0。脚本同时使用 Windows PowerShell 5.1 运行时提供的 API，以下精确命令也通过：

```text
powershell -NoProfile -File workspace/manage/punctum-vsh-s0/Test-S0.ps1
```

中间验证曾发现 Punctum 空壳源码的末尾空行不符合 rustfmt：首次 `cargo fmt --check` exit code 为 1。随后对四个 workspace 执行 `cargo fmt --all`，exit code 均为 0；刷新受 `src/**` 影响的 upstream export hash 后，完整模板和 baseline 复核全部通过。

## 验证结果

| 检查 | 结果 |
| --- | --- |
| 四个 `cargo metadata --locked` | 通过，四个 workspace root 与计划一致 |
| canonical path | 通过，7 条目标均位于仓库内且等于批准路径 |
| dependency 声明位置 | 通过，member manifest 无直接 `path =` |
| lockfile 独立性 | 通过，四个不同根目录中的普通文件，hash 分别匹配 baseline |
| SHA-256 baseline | 通过，root、排序成员列表、成员 manifest、lockfile、approved upstream export 全部可复算 |
| workspace 拓扑 | 通过，根 `Cargo.toml` 不存在，无第五或 umbrella workspace |
| git scope | 通过，最终变更都位于批准的 S0 写入范围 |
| 业务逻辑边界 | 通过，16 个 crate 都是空壳，没有进入 `F1` |
| 独立只读 verifier | `APPROVE` |

Cargo 和 Git 在当前 sandbox 中输出了无法 canonicalize 用户主目录、无法读取用户级 git ignore 的 warning。所有相关命令仍为 exit code 0；项目 path 由 verifier 使用 repo root 的真实 canonical path 单独校验，不依赖这些 warning 涉及的用户目录。

## 未通过的 gate

- `BATTLE-RULES-v0.1`：`Blocked`。canonical fixture path 和 SHA-256 都为 `null`，等待用户或 Product Owner 批准。Battle lane 和 game downstream 不能启动。
- `GPU-REF-v0.1`：`Blocked`。只有架构已固定的 `backend = Vulkan` 与 `AdapterInfo.device_type = Cpu` 有值；OCI image digest、Mesa、LLVM、wgpu、adapter identity 和 fixture hash 均逐项标记 `Blocked`。GPU readback 和 release 不能启动。

## 下一阶段

`F1` 尚未开始。下一轮可以并行启动：

- Punctum lane：grid/input。
- VSH lane：`vsh-core`。

Battle lane 仍受 `BATTLE-RULES-v0.1` 阻塞，不能与前两条 lane 一起启动。
