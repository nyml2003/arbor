# Punctum / VSH 第一期架构计划

- 状态：已批准
- 批准日期：2026-07-11
- 实现状态：未开始
- 评审结果：Planner、Architect、Critic 共识通过，最终 Critic 结论为 `APPROVE`
- 产品事实来源：[项目群总控](./punctum-vsh-program.md)

## 文档职责

本文是第一期实现的正式架构与 Agent 编排依据。它定义共享边界、workspace 拓扑、依赖方向、验证门禁、写入所有权和执行 wave。

产品范围、玩家可见行为和权限政策仍以项目群总控为准。本文不能反向修改产品事实。实现中的技术决定一旦改变产品行为，必须返回用户确认。

本计划只覆盖 Punctum、VSH、游戏、游戏内控制台和 TUI AI Chater。仓库中其他已有项目不在范围内，也不构成兼容约束。

## 已批准结论

第一期采用 `grid/input only` 方案。Punctum 的强制共享基础只有：

- 二维离散空间、geometry、`Surface<T>`、diff 和 `Patch<T>`。
- 规范化键盘事件与文本输入事件。

第一期不创建共享 `interaction` crate，也不冻结以下上层概念：

- component tree。
- component lifecycle。
- focus runtime。
- widget system。
- layout tree。
- event routing。
- retained render tree。
- 共享 `NodeId` 或 `TargetId`。

游戏和 Chater 可以先分别实现 selection、focus、text editing 和 routing。只有出现真实同构重复时，才能通过新 ADR 抽取上层 interaction 能力。

触发新 ADR 必须同时满足：

1. 游戏和 Chater 具有相同 input alphabet、state shape、transition 和 output oracle。
2. 同一套 black-box suite 可以原样运行于两个实现。
3. 候选 public contract 不含 battle、chat 或 backend 类型。
4. 抽取会删除两套重复实现，而不是增加 wrapper。

## Workspace 拓扑

第一期使用四个独立 Cargo workspace。不在 Arbor 根创建 Cargo workspace，也不创建 umbrella workspace。

| Workspace | Root manifest | 计划成员 | Lockfile |
| --- | --- | --- | --- |
| Punctum | `apps/punctum/Cargo.toml` | `punctum-grid`、`punctum-input`、`punctum-terminal`、`punctum-gpu` | `apps/punctum/Cargo.lock` |
| VSH | `packages/vsh/Cargo.toml` | `vsh-core` | `packages/vsh/Cargo.lock` |
| Game | `apps/gen3-game/Cargo.toml` | battle、UI、VSH adapter、host、E2E crates | `apps/gen3-game/Cargo.lock` |
| Chater | `apps/tui-chater/Cargo.toml` | chat application、UI、model adapter、host、E2E crates | `apps/tui-chater/Cargo.lock` |

四 workspace 方案用于隔离范围外 Rust 项目。当前没有足够证据支持根 workspace 或专用 umbrella workspace。只有实际出现不可接受的 dependency、lockfile 或验证成本时，才能通过新 ADR 重新评估。

## 写入所有权

设置唯一的 Program Integration Agent。它是 leader 或一个现有 `executor` 单人承担的任务身份，不是新增 agent role。

Program Integration Agent 独占：

- 四个 workspace root manifest。
- member list 和 `[workspace.dependencies]`。
- 四个 `Cargo.lock`。
- canonical path dependency。
- Game 和 Chater composition root。
- Game 和 Chater 跨域 E2E 接线。

lane writer 只能修改自己 crate 的 `Cargo.toml`、`src`、`tests` 和 fixtures。dependency 或 path 变化必须提交 change request，由 Program Integration Agent 在 barrier 串行接受。

两个 writer 不得同时修改同一文件、root manifest、lockfile 或 composition root。

## Path dependency 与版本门禁

path dependency 只在消费方 workspace root 的 `[workspace.dependencies]` 中定义。member manifest 只使用 `{ workspace = true }`。

| Consumer | Dependency | Cargo path | Canonical repo-relative target |
| --- | --- | --- | --- |
| Game | `punctum-grid` | `../punctum/crates/punctum-grid` | `apps/punctum/crates/punctum-grid` |
| Game | `punctum-input` | `../punctum/crates/punctum-input` | `apps/punctum/crates/punctum-input` |
| Game | `punctum-gpu` | `../punctum/crates/punctum-gpu` | `apps/punctum/crates/punctum-gpu` |
| Game | `vsh-core` | `../../packages/vsh/crates/vsh-core` | `packages/vsh/crates/vsh-core` |
| Chater | `punctum-grid` | `../punctum/crates/punctum-grid` | `apps/punctum/crates/punctum-grid` |
| Chater | `punctum-input` | `../punctum/crates/punctum-input` | `apps/punctum/crates/punctum-input` |
| Chater | `punctum-terminal` | `../punctum/crates/punctum-terminal` | `apps/punctum/crates/punctum-terminal` |

verifier 必须 canonicalize path，并确认目标位于 repo 内且等于批准路径。symlink、absolute dependency 或解析到其他副本全部拒绝。

每个 wave 为四个 workspace 分别记录：

```text
root_manifest_sha256
sorted_member_list_sha256
member_manifest_sha256_by_path
lockfile_sha256
approved_upstream_export_sha256
```

`upstream_export_sha256` 覆盖批准 crate 的 manifest、`src`、public fixtures 和 contract tests。consumer task packet 固定所需 hash。任务开始、handoff 和 verifier 重跑前都要复核。hash 改变时，下游任务进入 `Blocked`。

## 合同边界

### `punctum-grid`

提供 `GridPos`、`GridSize`、`GridRect`、`Surface<T>`、clip、blit、diff 和 `Patch<T>`。

不包含 identity、component state、focus、input、backend cell 或产品类型。

核心不变量：

- 容量计算不溢出。
- patch 始终有界。
- span 排序且不重叠。
- `apply(previous, diff(previous, next)) == next`。

### `punctum-input`

```text
KeyEvent { physical, logical, modifiers, phase }
TextEvent { text }
```

adapter 只能表达 host 实际提供的 press、repeat 和 release，不能伪造缺失事件。`punctum-input` 不负责 focus、dispatch、command binding 或 application state。

### Terminal adapter

- raw Terminal event 转换为 `punctum-input`。
- `Surface<TerminalCell>` 和 `Patch` 转换为 ANSI 输出。
- Unicode width、continuation、cursor 和 terminal capability 留在 adapter。
- adapter 不持有 chat state。

### GPU adapter

- window keyboard event 转换为 `punctum-input`。
- `Surface<SpriteCell>` 和 `Patch` 转换为 resource lookup 与 GPU submission。
- atlas、texture、alpha、shader、viewport 和 GPU resource 留在 adapter。
- adapter 不持有 game state。

Terminal 与 GPU backend 共享 geometry、surface 和 diff，不共享万能 `Cell`。

### Battle

`battle-domain` 持有 deterministic state 和 rule。`battle-application` 只暴露 query、legal action 和 `submit(BattleCommand)`。

Human keyboard UI 直接映射到 `BattleCommand`。`battle-vsh-adapter` 只能调用同一 public application API，不能访问 domain 内部状态。

### VSH

```text
ShellText -> AST -> PlanDraft
Agent output -------> PlanDraft
PlanDraft -> resolve -> schema/type validation -> sealed TypedPlan -> execute
```

- `PlanDraft` 永远不可信。
- `TypedPlan` 不能公开反序列化，也不能绕过 validator 构造。
- capability 使用 default-deny。
- discover、complete、read、write 和 invoke 分别授权。
- `resolve`、schema lookup 和 diagnostic 必须使用 capability-filtered registry view。
- 未授权 command 对 principal 表现为不可发现，不能从错误、补全或 schema diagnostic 泄漏存在性。
- `TypedPlan` 记录 `PrincipalId`、provider/command identity、registry generation、schema version 和 effect requirement。
- sealing 不构成永久授权。

每个 read、write 或 invoke effect 执行前，authorization service 原子校验 principal、registry/schema version 和 capability generation，并签发不可序列化、不可复制、单次消费的 `EffectPermit`。provider 必须消费 permit 才能执行。

permit 签发是 authorization linearization point。撤权先发生则 effect 拒绝；permit 先签发则当前 effect 可以完成，后续 effect 仍需重新授权。多 effect plan 在首次拒绝处停止，已完成 effect 不自动回滚。需要原子性的 command 由 application 或 provider 提供 transaction。

### TUI AI Chater

`chat-application` 持有 conversation、model selection 和 model port。Chater UI 消费 grid/input，自行持有 editor、selection 和 routing。Terminal adapter 不持有 chat state。第一期 Chater 不依赖 VSH。

## 依赖方向

```text
# A -> B 表示 A depends on B

game-host -> game-ui -> battle-application -> battle-domain
game-host -> punctum-gpu -> punctum-grid + punctum-input
game-ui -> punctum-grid + punctum-input

battle-agent -> battle-vsh-adapter -> battle-application
game-console -> battle-vsh-adapter -> vsh-core
battle-vsh-adapter -> vsh-core
vsh-core -X-> battle-domain

tui-host -> chater-ui -> chat-application -> model-port
tui-host -> punctum-terminal -> punctum-grid + punctum-input
chater-ui -> punctum-grid + punctum-input

punctum-grid/input -X-> game / chat / VSH / Crossterm / wgpu
```

## Battle Rule Fixture Gate

- semantic owner：用户或 Product Owner。
- custodian：Program Integration Agent。
- identity：`BATTLE-RULES-v0.1`。
- tracked approval record 保存 canonical fixture bundle 的 SHA-256。

fixture 未批准、缺失或 hash 不符时，`battle-domain` 和所有 game downstream task 标记 `Blocked`。Agent 不得自行补规则，也不得修改 fixture 迁就实现。grid/input、VSH、Terminal 和 Chater lane 可以继续。

本门禁尚未通过。因此文档已经足够启动 `S0`、Punctum、VSH 和 Chater 工作，但不能宣称整个第一期已无阻塞。

## GPU Reference Gate

GPU release oracle 使用 tracked record `GPU-REF-v0.1`。主 adapter 固定为 pinned Linux CI image 中的 Mesa `llvmpipe` Vulkan software adapter。

record 必须记录并精确匹配：

```text
OCI image digest
Mesa package version
LLVM version
wgpu version
backend = Vulkan
AdapterInfo.name
AdapterInfo.vendor
AdapterInfo.device
AdapterInfo.device_type = Cpu
AdapterInfo.driver
AdapterInfo.driver_info
approved_fixture_sha256
```

任一字段为空、CI image 使用 mutable tag 或 runtime identity 不匹配时，GPU readback 和 release gate 标记 `Blocked`。

普通 hardware adapter 只运行 logical oracle 和 smoke test。fallback 必须有独立 pinned image、identity record、golden 和 Product Owner 批准。第一期不预先批准 fallback。

## 测试矩阵

| 范围 | Oracle |
| --- | --- |
| grid | scalar full-frame reference；property test 验证 diff/apply |
| input | Terminal/GPU raw fixture 与 canonical event fixture 精确相等 |
| Terminal | in-memory terminal golden，覆盖 resize、wide cell、cursor |
| GPU logical | CPU reference 的 coordinate、resource ID、clip、order 和 instance data 精确相等 |
| GPU readback | `GPU-REF-v0.1`；固定 `Rgba8Unorm`、viewport、scissor、MSAA 1、nearest sampling、atlas 和 clear color |
| Battle | 已批准 `BATTLE-RULES-v0.1` vector 与 replay hash |
| VSH | capability matrix、malformed draft、bypass 和 TOCTOU concurrency |
| Human/VSH | 相同 `BattleCommand` 产生相同 application event log |
| Chater | deterministic fake model 与 Terminal surface golden |

GPU readback 去除 row padding，归一为 top-left RGBA8。逐通道绝对误差不超过 1。普通 hardware adapter 结果不属于 release oracle。

VSH TOCTOU 必须覆盖：

- seal 后、首个 effect 前撤权，handler 调用 0 次。
- effect 1 后撤权，effect 2 不执行并返回 `AuthorizationRevoked`。
- registry 或 schema version 变化时拒绝旧 `TypedPlan`。
- revoke 与 permit issuance 并发时符合 linearization order。
- principal 和 context 没有伪造或反序列化路径。
- principal 与 operation authorization matrix 覆盖 100%。

核心不变量分支覆盖目标为 90%。Ollama 和 DeepSeek live smoke test 不作为 deterministic CI oracle。

## 执行 Wave

每个 task 使用 repo-absolute、task-unique target：

```text
<repo-absolute>/.target/tasks/<wave>/<task-id>/<workspace>
```

### `S0`：串行脚手架

Program Integration Agent 创建四个 workspace、四个 lockfile、canonical path 表、initial baseline、`BATTLE-RULES-v0.1` approval slot 和 `GPU-REF-v0.1` record。

`S0` 不并行。它完成并通过独立验证前，不启动 `F1`。

### `F1`：三个并行 lane

- Punctum lane：grid/input。
- VSH lane：`vsh-core`。
- Battle lane：`battle-domain` 和 `battle-application`，只在 Battle Rule Fixture Gate 通过后启动。

### `B1`：串行 barrier

接受 crate-local manifest delta，更新对应 lockfile，生成 Punctum、VSH 和 Battle export hash。

### `F2`：三个并行 lane

- Terminal adapter。
- GPU adapter。readback 需要 `GPU-REF-v0.1` 通过。
- `chat-application` 和 model port。

### `B2`：串行 barrier

更新 Punctum 和 Chater baseline，冻结 Terminal、GPU 和 Chat export hash。

### `F3`：三个并行 lane

- game UI library。
- battle-VSH、console 和 agent library。
- Chater UI 与 model adapter library。

### `B3`：串行 barrier

接受四个 workspace 的 crate-local manifest delta，更新对应 lockfile 和 baseline。

### `F4`：串行 integration

唯一 Program Integration Agent 先完成 Game composition 和 `game-e2e`，再完成 Chater composition 和 `chater-e2e`。

跨域 E2E 归消费方 workspace：

- `apps/gen3-game/crates/game-e2e/` 验证 Human/VSH 等价、GPU logical output 和 battle closure。
- `apps/tui-chater/crates/chater-e2e/` 验证 keyboard、chat application、fake model 和 Terminal surface。
- 不建立同时依赖 Game 与 Chater 的第五个 E2E workspace。

### `F5`：只读验证

逐一运行四个 workspace 完整模板，再运行 GPU reference、Game E2E、Chater E2E、path canonicalization 和 upstream hash 检查。

## Agent Staffing

| 阶段 | Writer | Reviewer | 并行规则 |
| --- | --- | --- | --- |
| `S0` | 一个 Program Integration Agent | 一个只读 `verifier` | 串行，不启动其他 writer |
| `F1` | 最多三个 `executor` | `verifier`；VSH 追加 `security-reviewer` | 三个 lane 可并行，Battle 受规则门禁限制 |
| `B1` | Program Integration Agent | `verifier` | 串行 barrier |
| `F2` | 最多三个 `executor` | `verifier` | 三个 lane 可并行，GPU readback 受 reference gate 限制 |
| `B2` | Program Integration Agent | `verifier` | 串行 barrier |
| `F3` | 最多三个 `executor` | `verifier`；VSH 追加 `security-reviewer` | 三个 lane 可并行，文件所有权不得重叠 |
| `B3` | Program Integration Agent | `verifier` | 串行 barrier |
| `F4` | Program Integration Agent | `verifier` | Game 与 Chater composition 串行 |
| `F5` | 无 writer | 独立 `verifier`；VSH 追加 `security-reviewer` | 只读验证 |

每个 writer handoff 必须报告：

- 修改路径和写入所有权。
- 使用的合同版本与 upstream export hash。
- absolute `CARGO_TARGET_DIR`。
- 实际执行的验证命令与 exit code。
- 未通过的 gate、残余风险和 change request。

leader 收到 handoff 后先检查 write scope、baseline 和 upstream hash，再让 verifier 使用新的 task-unique target 独立重跑。barrier 通过前不得启动下一 wave。

## Workspace 验证模板

每组命令使用该 task 独占的 absolute `CARGO_TARGET_DIR`。

```powershell
cargo metadata --locked --manifest-path <manifest> --format-version 1
cargo check --workspace --all-targets --locked --manifest-path <manifest>
cargo fmt --all --manifest-path <manifest> -- --check
cargo clippy --workspace --all-targets --locked --manifest-path <manifest> -- -D warnings
cargo test --workspace --all-targets --locked --manifest-path <manifest>
```

四个 `<manifest>`：

```text
<repo>/apps/punctum/Cargo.toml
<repo>/packages/vsh/Cargo.toml
<repo>/apps/gen3-game/Cargo.toml
<repo>/apps/tui-chater/Cargo.toml
```

lane 验证把最终 test 收窄为 `-p <owned-package>`。wave barrier 和 `F5` 必须执行完整 workspace 模板，并检查 baseline、write scope 和反向依赖。

## 下一次实现 Session 的启动方式

下一位 agent 必须按以下顺序取得上下文：

1. 遵守新 session 实际注入的 `AGENTS.md instructions`。仓库根当前没有持久化的 `AGENTS.md` 文件，不要把该路径当成启动依赖。
2. 读取[项目群总控](./punctum-vsh-program.md)。
3. 读取本架构计划。
4. 读取[Punctum PEP 0001](../../apps/punctum/peps/0001-punctum-technical-direction.md)，只作为次级来源；冲突时以前两份文档为准。

第一轮实现只执行 `S0`。不要同时派发 `F1`，也不要修改其他已有项目。

可在新的 Codex session 中直接发送：

```text
开始实现 Punctum / VSH 项目群的第一期。

遵守当前 session 注入的 AGENTS.md instructions。先读取 workspace/manage/punctum-vsh-program.md 和 workspace/manage/punctum-vsh-architecture-plan.md。产品事实以总控文档为准，架构、所有权、门禁和 wave 以架构计划为准。只关注 Punctum、VSH、gen3-game、游戏控制台和 tui-chater，忽略其他已有项目。

本次只执行 S0：由单一 Program Integration Agent 创建四个独立 Cargo workspace 的最小脚手架、独立 lockfile、canonical path 表、初始 SHA-256 baseline、BATTLE-RULES-v0.1 approval slot 和 GPU-REF-v0.1 record。不要进入 F1，不要实现业务逻辑，不要自行补充未批准的对战规则，不要建立 Arbor 根 Cargo workspace。

完成后独立验证 write scope、四个 cargo metadata --locked、path canonicalization 和 baseline，并报告修改文件、验证证据、尚未通过的门禁和下一 wave 的可并行 lane。
```

`S0` 验收后，下一轮才启动 `F1`。OMX runtime 可用时按一个 wave 启动 team；普通 Codex App 使用 bounded native subagents。不得一次跨 wave 派发。

## 当前门禁状态

| Gate | 状态 | 影响 |
| --- | --- | --- |
| `P0 Product Clarified` | 已通过 | 产品范围可作为实现依据 |
| `A0 Architecture Approved` | 已通过 | 可以进入 `S0` |
| `S0 Workspace Ready` | 未开始 | `F1` 尚不可启动 |
| `BATTLE-RULES-v0.1` | 待用户批准 | Battle lane 和 game downstream 被阻塞 |
| `GPU-REF-v0.1` | 未建立 | GPU readback 和 release 被阻塞 |

## 风险与约束

| 风险 | 约束 |
| --- | --- |
| grid/input 过小，产品重复 interaction | 允许短期重复，达到 extraction gate 后再立 ADR |
| path dependency 漂移 | canonical path、export hash、consumer pin 和 barrier 复核 |
| VSH seal 后撤权失效 | per-effect `EffectPermit`、linearization test、100% capability matrix |
| Battle rule 不明确 | 未批准 `BATTLE-RULES-v0.1` 时阻塞 Battle lane |
| GPU golden 受硬件影响 | pinned llvmpipe identity，hardware 只做 smoke test |
| Agent 争抢 manifest 或 lockfile | Program Integration Agent 单 owner，barrier 串行接受 |
| 四 workspace 依赖版本漂移 | 各自 lockfile 和 export hash；有真实成本后再评估 umbrella workspace |

## 计划变更规则

- 产品事实变化时，先更新项目群总控，再评估本文。
- shared core 扩张必须经过 extraction gate 和新 ADR。
- root manifest、lockfile、path dependency 或 composition ownership 变化必须由 Program Integration Agent 接受。
- 任何放宽 authorization、Battle fixture 或 GPU release oracle 的变化都需要独立评审。
- 每个 wave 完成后更新总控状态，不在临时聊天中维护唯一事实。
