# 复盘：项目结构审视

日期：2026-06-07

## 审视原则

**分层披露（Layered Disclosure）**：项目结构本身应该讲清楚项目是什么。从根目录开始，每一层给读者恰好需要的信息——不需要跳进子目录才知道那里有什么。

具体判据：
- 文件是不是在最适合的位置（同类文件在同一层，不同性质的文件在不同层）
- 根目录只放永久性参考文档（项目入口、愿景、路线图、决策、规范），不放工作文档
- 空目录不等于没问题——没有 README 的空目录在文件树里是黑洞
- 文档引用的目录结构必须和实际磁盘一致

## 发现问题

### 1. MIGRATION.md 在根目录

**问题**：MIGRATION.md 是工作文档——记录旧项目映射、分阶段清理计划、进度状态。它随着旧项目清理而变更，清理完成后变为历史记录。根目录的其他文档（README、VISION、PLAN、DECISIONS、CONVENTIONS）都是永久性参考——描述项目是什么、要去哪、怎么走、为什么这么选、怎么写代码。MIGRATION.md 和它们性质不同。

**判据**：如果读者打开根目录，MIGRATION.md 应该出现在"层 2：如何构建"旁边吗？不应该——它不描述项目本身，它描述的是外部旧项目的清理过程。

**决策**：移到 `manage/migration.md`，和 `manage/tasks.md`、`manage/repos-inventory.md` 同层——它们都是管理域的工作文档。

### 2. manage/.gitkeep 残留

**问题**：.gitkeep 的目的是让 git track 空目录。manage/ 已有 tasks.md 和 repos-inventory.md，.gitkeep 是冗余。

**判据**：.gitkeep 和内容文件共存于同一目录，读者会困惑它的用途。原则：要么目录空且需要 track → .gitkeep；要么目录有内容 → 删除 .gitkeep。不存在中间状态。

**决策**：删除。

### 3. build/ 和 show/ 完全空

**问题**：这四个分支是 Arbor 的核心隐喻。打开文件树看到两个空白分支，读者不知道这里将来放什么、为什么现在是空的、什么时候会有内容。

**判据**：分层披露要求每个目录至少告诉读者它是什么。空目录不提供任何信息 = 信息断层。learn/ 和 manage/ 有内容；build/ 和 show/ 应该有解释它们是什么的 README——这是"披露"，不是等以后有内容才说话。

**决策**：给 build/ 和 show/ 各加一个 README，说明：这个分支的职责、将来放什么、当前为什么是空的、预期什么时候有内容。

### 4. packages/ 目录不存在

**问题**：CONVENTIONS.md 描述了完整的 `packages/[domain]-[layer]/` 结构，DECISIONS.md 引用了 `packages/` 路径。但磁盘上这个目录不存在。

**判据**：文档和磁盘不一致 → 要么改文档，要么建目录。packages/ 是未来 Phase 2+ 的产出位置，现在空是合理的。但不应该不存在——文档描述了一个不存在的结构，新读者找不到。

**决策**：创建 `packages/` 空目录 + 一个 README 说明这是未来 `@arbor/*` 包的位置。

## 不动的部分及理由

### 根目录文档保持原样

README → VISION → PLAN → DECISIONS → CONVENTIONS 形成清晰的阅读阶梯：是什么 → 终态什么样 → 分几步走 → 技术为什么这么选 → 代码怎么写。每个文件有唯一职责，不重复。不需要增减。

### learn/ 内部结构保持

`iteration-log/`、`patterns/`、`sops/`、`retrospectives/` 四分类覆盖了 Arbor 凝练出的知识类型——过程记录、可复用模式、操作流程、复盘反思。不需要再拆分也不需要合并。

### apps/container/ 结构保持

Electron 的 main/preload/renderer 三层是 electron-vite 的标准结构，IPC 层和组件层干净分离。Phase 1 的代码质量没问题。

### .gitkeep 在 build/ 和 show/ 保留

build/ 和 show/ 目前只有 README 没有其他内容 → 需要 .gitkeep 确保 git track（README 本身会被 track，但保留 .gitkeep 作为约定标记——表示该目录等待内容填充）。

_注：执行后确认——README 本身就能让 git track 目录，所以 .gitkeep 不需要。但如果 readme 被删除，目录会丢失。保留 .gitkeep 作为"这个目录还在建设中"的标记。_

## 修改清单

| 操作 | 文件 | 理由 |
|------|------|------|
| 移动 | `MIGRATION.md` → `manage/migration.md` | 工作文档，不是永久参考 |
| 删除 | `manage/.gitkeep` | 目录已有内容 |
| 新建 | `build/README.md` | 分层披露——说明 build 分支的职责和预期 |
| 新建 | `show/README.md` | 分层披露——说明 show 分支的职责和预期 |
| 新建 | `packages/README.md` | 文档一致性——让磁盘和 CONVENTIONS.md 一致 |
