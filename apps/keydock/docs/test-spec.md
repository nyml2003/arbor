# KeyDock 测试规格

## 目标

这份规格用于后续实现阶段验收 KeyDock。测试重点不是覆盖率数字，而是确认产品行为、Win32 边界和 unsafe 范围符合方案。

## 单元测试

### 布局

- QWERTY 布局包含 v1 必做键
- 每行按键顺序稳定
- `width_units` 能计算出非重叠矩形
- Space 宽度大于普通字符键
- 文本标签不会影响布局尺寸

### 命中测试

- 点在 key rect 内返回对应 `KeyId`
- 点在 key gap 内返回 `None`
- 点在面板外返回 `None`
- 边界点行为稳定

### 状态机

- hover 只由 pointer move 改变
- pointer down 设置 pressed
- pointer cancel 清理 pressed
- pointer up 命中同一 key 才触发 action
- Shift 点击后进入一次性锁存
- 普通字符输入后 Shift 自动释放
- Ctrl/Alt 组合输入后默认释放

### 输入命令

- 普通字符生成 `Text` 或 `KeyTap`
- Backspace/Enter/Esc 生成虚拟键命令
- Ctrl+C 这类组合按 modifier down -> key tap -> modifier up 顺序表达
- Close 生成关闭命令，不生成输入命令

### primitive tree

- `KeyboardSurface` 生成根 `Surface`
- 每个 `KeyRow` 生成一个 `Row`
- 普通 `Key` 生成 `Button(Text)`
- `ModifierKey active` 生成 `Button` 的 active 状态
- `ActionKey` v1 生成 `Button(Text)`，未来允许 `Button(Image)`
- `Text` 的内容和样式不改变布局尺寸
- `Image` 只能通过组件 `id` 引用内置资源
- `Image` 不能持有文件路径、网络地址或平台 bitmap

## 静态边界检查

实现后必须通过：

```powershell
rg "unsafe|windows::Win32|HWND|WPARAM|LPARAM|SendInput|ID2D|IDWrite" apps/keydock/src/app
```

期望：无结果。

实现后必须检查：

```powershell
rg "unsafe" apps/keydock/src/platform/windows
```

期望：结果只出现在小型 wrapper 或 window proc 附近，每处有不变量注释。

## 构建检查

后续 Rust 项目创建后运行：

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo check --target x86_64-pc-windows-msvc
```

如果普通 PowerShell 找不到 MSVC 工具链，使用 VS 2022 Developer PowerShell，或先加载：

```powershell
& "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
```

## 手动验收

### 基础输入

1. 打开 Notepad
2. 让文本区域获得焦点
3. 打开 KeyDock
4. 点击 `A`、`B`、`C`
5. Notepad 中出现输入内容

### 不抢焦点

1. 打开 Notepad 并让文本区域获得焦点
2. 点击 KeyDock 的普通键
3. 输入进入 Notepad
4. Notepad 仍是输入目标

### 修饰键

- 点击 Shift 后再点 A，输出大写 A
- Shift 使用一次后释放
- Ctrl + A 能选中目标文本
- Alt 组合后不会永久停留在 active 状态

### DPI

- 在 100%、150%、200% 缩放下打开 KeyDock
- 键位不重叠
- 文本不截断
- 边框清晰

### 多应用

至少验证：

- Notepad
- Windows Terminal
- 浏览器地址栏或输入框

## 已知系统限制

- 普通权限 KeyDock 不承诺向管理员权限窗口注入输入
- 被 Windows UIPI 拒绝时必须报告失败，不 panic
- v1 不支持 IME 候选词和组合文本

## 完成标准

- 产品文档、组件文档、架构文档和测试规格保持一致
- app 层没有 Win32 和 unsafe 泄漏
- 基础输入、不抢焦点、修饰键和 DPI 手动验收通过
- 构建、测试、clippy 通过
