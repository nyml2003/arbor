# Kaubo 展品 — UI 设计

## 设计目标

不是"一个编译器可视化工具"。是**一个IDE里的编译器在做它的工作，而你可以在旁边看着**。

三个关键词：
- **灵动**：界面不是死的。光标移动→所有视图同步响应。面板展开/收起有呼吸感。
- **多视图**：同一段代码的四张脸（tokens/AST/bytecode/result）同时可见。不是 tab 切换，是**并列投影**。
- **IDE体验**：语法高亮、行号、错误波浪线、光标驱动的联动。这不是 textarea + pre。

---

## 参考项目

| 项目 | 学什么 |
|------|--------|
| **AST Explorer** (astexplorer.net) | 代码↔AST双向联动、光标驱动、树节点展开/折叠状态机 |
| **Python Tutor** (pythontutor.com) | 逐步执行可视化、栈/堆/箭头、执行状态的空间化表达 |
| **Compiler Explorer** (godbolt.org) | 源码↔输出逐行对应、颜色映射、面板可折叠 |
| **Gibber / Estuary** | 即时代码反馈、演出式界面、空白即界面 |
| **项目ional Editing** 理念 | 同一结构的多视图投影、焦点跟随 |

---

## 总体布局

```
┌──────────────────────────────────────────────────────────────┐
│  Pipeline Bar                                                │
│  ●────●────●────●────●    Lexer → Parser → CodeGen → VM     │
│  12   1    24   > 5                                           │
│  tokens mod  inst output                                      │
├──────────────────────────────────────────────────────────────┤
│                         │                                     │
│   Code Editor            │   Projection Panel                 │
│   ┌───────────────────┐  │   ┌───────────────────────────┐   │
│   │ 1 │ var x = 42;   │  │   │                           │   │
│   │ 2 │ var y = x+1;  │  │   │   Selected stage detail   │   │
│   │ 3 │ print(y);     │  │   │   (expanded view)         │   │
│   │   │               │  │   │                           │   │
│   │   │               │  │   │                           │   │
│   │   │               │  │   │                           │   │
│   │   │               │  │   │                           │   │
│   └───────────────────┘  │   └───────────────────────────┘   │
│                         │                                     │
│                         │   Mini Projections (collapsed)      │
│                         │   ┌──────┬──────┬──────┬──────┐    │
│                         │   │Tokens│ AST  │ByteCd│Result│    │
│                         │   │12    │1 mod │24 ins│> 43  │    │
│                         │   └──────┴──────┴──────┴──────┘    │
│                         │                                     │
├──────────────────────────────────────────────────────────────┤
│  Status Bar                                                   │
│  kaubo-engine v0.1.0  │  total: 1.2ms  │  486 tests passing  │
└──────────────────────────────────────────────────────────────┘
```

**核心交互**：
- 顶部 Pipeline Bar：展示四个阶段的实时状态（已跑完的亮色，未跑完的灰色，正在跑的脉冲动画）。点击某个阶段→右侧 Projection Panel 切到该阶段的详细视图。
- 左侧 Code Editor：CodeMirror 6，语法高亮、行号、光标跟踪。
- 右侧 Projection Panel：当前选中阶段的详细可视化。Tokens 就是表格，AST 就是缩进树，Bytecode 就是反汇编列表，Result 就是输出文本。
- 下方 Mini Projections：四个阶段各一张"缩略卡片"——显示关键数字（12 tokens / 1 module / 24 instructions / output: 43）。点击卡片→右侧 Projection Panel 切到该阶段。当前激活的卡片有高亮边框。
- 底部 Status Bar：引擎版本、总耗时、测试通过数。

---

## 多视图联动

这是"灵动"的核心机制。所有视图共享同一个焦点状态。

### 光标驱动

```
Code Editor 光标在 L2:9 (字符 'x')
        │
        ├── Token 面板：L2:9 的 token (ID "x") 高亮
        ├── AST 面板：包含该 token 的 AST 节点 (Identifier "x") 高亮，父节点链展开
        ├── Bytecode 面板：该 AST 节点对应的指令 (LOAD_NAME 0) 高亮
        └── Result 面板：如果 VM 在执行中，当前执行到的指令高亮
```

方向反过来也行：点击 AST 节点→编辑器滚动到对应行并高亮。点击 Token→编辑器光标跳到对应位置。点击 Bytecode 指令→对应的 AST 节点和源码行同时高亮。

### 颜色编码

每一行源码有一个微妙的背景色标记，四种颜色对应四个阶段的产物：
- Token 的颜色（淡蓝）标记该行产生了哪些 token
- AST 的颜色（淡绿）标记该行属于哪个语法节点
- Bytecode 的颜色（淡紫）标记该行对应哪些指令
- Result 的颜色（淡黄）标记该行被执行了几次

鼠标悬停在某一行→该行的四个颜色标记同时变亮，Mini Projections 里的对应卡片脉冲一次。

---

## 动画原则

### 面板展开/收起

点击 Mini Projection 卡片→它不是"tab 切换"。当前展开的面板**向上收缩**，新选中的面板**向下展开**。总共约 200ms，缓出曲线。

### Pipeline Bar 的状态转换

- 阶段未运行：灰色圆点
- 阶段运行中：脉冲动画（缩放 1 → 1.3 → 1，循环）
- 阶段已完成：亮色实心圆点，带一个微小的"弹入"动画
- 阶段出错：红色圆点，带抖动

### 代码变更→流水线重跑

用户在编辑器里打字→防抖 150ms→流水线重新执行。四个阶段依次完成（Lexer 0.2ms, Parser 0.3ms, CodeGen 0.2ms, VM 0.5ms），Pipeline Bar 上的圆点依次亮起。**这不是进度条，是编译器的心跳。**

---

## IDE 体验

### 代码编辑器

- **CodeMirror 6**：模块化、轻量、支持自定义 language mode
- kaubo 语法高亮：关键字蓝色、字符串绿色、数字橙色、注释灰色斜体
- 行号、缩进辅助线
- 错误波浪线：Parser 错误在对应行下方显示红色波浪线，hover 显示错误信息
- 自动补全：MVP 不做，但预留 extension 接口

### 状态栏

左边：`kaubo-engine v0.1.0`
中间：`lex: 0.2ms | parse: 0.3ms | codegen: 0.2ms | exec: 0.5ms | total: 1.2ms`
右边：`486 tests`

### 快捷键

| 键 | 动作 |
|----|------|
| `Ctrl+Enter` | 强制重新执行流水线 |
| `Ctrl+1/2/3/4` | 切换到 Tokens/AST/Bytecode/Result 面板 |
| `Ctrl+Shift+F` | 重置到默认示例 |

---

## 示例系统

打开页面时不是空白编辑器。预加载一个默认示例，流水线已经跑完，四个面板都有内容。

示例列表（下拉菜单在 Pipeline Bar 右侧）：
- Hello World — `print("Hello, Kaubo!");`
- 变量与运算 — `var x = 42; print(x + 1);`
- Lambda — `var add = \|a, b\| { return a + b; }; print(add(2, 3));`
- 条件分支 — `var x = 10; if (x > 5) { print("big"); } else { print("small"); }`
- JSON 对象 — `var p = json { name: "Alice", age: 30 }; print(p.name);`

切换示例→编辑器内容更新→流水线自动重跑→所有面板更新。

---

## 暗色主题

默认深色主题（和 Arbor container 的 tokens.css 保持一致）。每个面板有微妙的背景色区分——不是纯黑，而是不同层次的深灰。Pipeline Bar 的四个圆点使用四种主色调：蓝、绿、紫、琥珀。

---

## 不出现在 MVP 中

- Monaco Editor（CodeMirror 6 够用）
- 可拖拽面板
- 多文件/标签页
- 移动端布局
- 自动补全
- 面板大小拖拽调整（先用固定比例）
