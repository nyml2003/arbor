# Punctum / Ramus B1 验收记录

- 结果：`Passed`
- 日期：2026-07-11
- 范围：Punctum、Ramus、Game、TUI Chater 四个独立 Cargo workspace
- 自动校验：[`Test-B1.ps1`](./Test-B1.ps1)
- 冻结记录：[`records.json`](./records.json)

## 结论

`B1` 已通过。三个 `F1` public contract 已冻结。四个 workspace 的 manifest、lockfile、成员清单、canonical path dependency 和 upstream export hash 一致。

本阶段没有建立 Arbor 根 Cargo workspace。没有实现 Tetris 规则、Terminal/GPU adapter、产品 UI 或跨域 E2E。

## 物理改名

| 旧路径或名称 | 当前路径或名称 |
| --- | --- |
| `packages/vsh` | `packages/ramus` |
| `vsh-core` | `ramus-core` |
| Rust crate `vsh_core` | Rust crate `ramus_core` |
| `battle-vsh-adapter` | `battle-ramus-adapter` |
| `punctum-vsh-program.md` | `punctum-ramus-program.md` |
| `punctum-vsh-architecture-plan.md` | `punctum-ramus-architecture-plan.md` |

历史目录 `workspace/manage/punctum-vsh-s0` 保留。它是 `S0` 审计记录，不是活动命名。

## `B1` 接入

- Game 通过 `../../packages/ramus/crates/ramus-core` 依赖 `ramus-core`。
- `apps/punctum/examples/tetris` 已作为 `punctum-tetris-demo` 加入 Punctum workspace。
- Tetris package 只依赖 `punctum-grid` 和 `punctum-input`，目前没有业务实现。
- `apps/gen3-game/scripts/Test-Battle.ps1` 覆盖 `battle-application/src/*.rs`，不会漏掉侧别观察模块。

## 冻结 export

| Contract | SHA-256 |
| --- | --- |
| `punctum-f1` | `e7733a2534ece8ea605ec8da155a24a28abfdf7be3afa55b0dcefaff5ba0e4aa` |
| `ramus-f1` | `2eab79813eec897e7e2c7171af1db6d8ebf88676985fa6f1bcd058d5b917a109` |
| `battle-f1` | `f9fcd8f55214c964ebaed62327fe3849bf30009fbf625ac1d19400202bd62eb2` |

`records.json` 同时冻结四个 workspace 的 root manifest、lockfile、成员 manifest、成员列表和 approved upstream export。

## 本地验证

以下验证使用任务独占的 `.target/tasks/b1/*` 目录。

```powershell
workspace/manage/punctum-ramus-b1/Test-B1.ps1

cargo metadata --locked --format-version 1 --manifest-path <manifest>
cargo fmt --all --manifest-path <manifest> -- --check
cargo clippy --workspace --all-targets --locked --manifest-path <manifest> -- -D warnings
cargo test --workspace --all-targets --locked --manifest-path <manifest>
```

四个 `<manifest>` 均通过：

- `apps/punctum/Cargo.toml`
- `packages/ramus/Cargo.toml`
- `apps/gen3-game/Cargo.toml`
- `apps/tui-chater/Cargo.toml`

专项结果：

- Ramus：78 tests 通过；示例运行通过；10 个纯逻辑文件没有漏行或漏分支，functions `158/158`。
- Punctum：57 tests 通过；lines `339/339`、functions `43/43`、regions `469/469`。
- Battle：28 个 domain tests 与 15 个 application tests 通过；`Test-Battle.ps1` 的 line、function、region 和 branch 门禁通过。
- Game 与 TUI Chater 完整 workspace 的 fmt、clippy 和 tests 通过。
- `Test-B1.ps1` 检查退役路径、四份独立 lockfile、canonical dependency、baseline、export hash 和写入范围，结果为 `B1 verification passed`。

## 限制

- 本阶段只做本地验证，不建设 CI。
- `GPU-REF-v0.1` 仍为 `Blocked`。固定 llvmpipe 环境的 readback 与 release gate 尚未通过。
- 下一个阶段是 `PT1`。它只实现 headless Tetris core、surface paint 和 input mapping。
