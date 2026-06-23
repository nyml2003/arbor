# KeyDock 原子组件说明

## 设计原则

KeyDock 的 UI 用原子组件建模，但不引入前端框架。通用组件 DSL、数据模型和渲染描述在 `packages/arbor-ui-core`，Windows Direct2D/DirectWrite renderer 在 `packages/arbor-ui-windows`，KeyDock app 层只组合这些原语表达键盘业务。

`arbor-ui-core` 不得依赖 Win32 类型，不出现 `HWND`、`WPARAM`、`LPARAM`、`SendInput`、`unsafe`、Direct2D/DirectWrite COM 类型或 `windows::Win32::*`。

## 组件分层

```text
KeyboardSurface
  -> DockHandle
  -> StatusIndicator
  -> KeyRow
      -> Key / ModifierKey / ActionKey / SpaceKey

arbor-ui-core
  -> Rust DSL
  -> Primitive tree
      -> Surface / Row / Button / Text / Image
```

`Key`、`ModifierKey`、`ActionKey`、`SpaceKey` 是业务组件，负责输入语义。`Button`、`Text`、`Image` 是渲染原语，负责描述屏幕上应该画什么。平台渲染层只接收原语树，不理解键盘业务。

## Component DSL

Component DSL 是 `arbor-ui-core` 暴露的安全 Rust builder 层，不是前端框架，也不是平台控件库。

示例：

```rust
button("key-close-title", rect)
    .state(ButtonState::Normal)
    .intent(ButtonIntent::Action)
    .child(image("close-icon", icon_rect).build())
    .build()
```

DSL 最终生成 `Primitive` tree。平台层只消费 `Primitive` tree。

## Primitive tree

Primitive tree 是 `arbor-ui-core` 的安全 Rust 渲染模型。

### Surface

面板或区域背景。

字段：

- `id`
- `rect`
- `background`
- `border`
- `radius`
- `children`

规则：

- 只表达视觉容器
- 不处理点击行为
- 不持有窗口句柄或渲染资源

### Row

水平排列容器。

字段：

- `id`
- `rect`
- `gap`
- `align`
- `children`

规则：

- 用于按键行、标题栏和状态区
- 不参与输入注入语义
- 不根据文字长度反向改变布局高度

### Button

可点击矩形原语。

字段：

- `id`
- `rect`
- `state`
- `intent`
- `content`

状态：

- `normal`
- `hovered`
- `pressed`
- `active`
- `disabled`

规则：

- 可以承载 `Text` 或 `Image`
- 点击语义由业务组件提供
- 不直接产生 `InputCommand`
- 不知道 `SendInput` 或 Win32 虚拟键码

### Text

文本绘制原语。

字段：

- `id`
- `rect`
- `content`
- `style`
- `align`

规则：

- 只描述文本内容和样式 token
- 不持有 DirectWrite 对象
- 文本不能撑开固定格式布局
- 超出时按组件规则截断或缩小，不覆盖相邻元素

### Image

图片绘制原语。

v1 只允许引用内置静态资源。

字段：

- `id`
- `rect`
- `tint`
- `opacity`

规则：

- 不允许外部文件路径
- 不允许网络图片
- 不在组件层做解码
- 不持有 Direct2D bitmap
- 当前资源 ID 使用组件 `id`
- v1 核心键盘不依赖图片；它只为关闭图标、状态图标等未来能力预留

## KeyboardSurface

键盘最外层面板。

职责：

- 保存面板尺寸
- 保存内边距、行间距、键间距
- 管理所有 `KeyRow`
- 生成可渲染的 primitive tree

不负责：

- 创建窗口
- 处理 Win32 消息
- 注入输入
- 管理 DPI API

## DockHandle

停靠控制区。

v1 职责：

- 显示标题 `KeyDock`
- 提供关闭按钮区域
- 为后续拖动和停靠预留语义

v1 不做：

- 真正拖动窗口
- 多显示器吸附
- 位置记忆

## StatusIndicator

状态指示区域。

显示：

- Shift 是否激活
- Ctrl 是否按下
- Alt 是否按下

原则：

- 只显示和输入语义相关的状态
- 不显示调试信息
- 不显示技术说明

## KeyRow

一行按键。

职责：

- 按权重分配水平空间
- 处理行内 key gap
- 生成每个键的矩形区域

规则：

- 行高由 `KeyboardSurface` 统一控制
- 单个键不能反向影响整行高度
- 文本不能撑开布局

## Key

普通字符键。

字段：

- `id`
- `label`
- `output`
- `width_units`
- `state`

状态：

- `normal`
- `hovered`
- `pressed`

行为：

- 点击后产生一个字符输入命令
- 如果 Shift 激活，输出 Shift 后的字符
- 点击后一次性 Shift 自动释放
- 渲染时生成一个 `Button(Text)`

## ModifierKey

修饰键。

类型：

- Shift
- Ctrl
- Alt

状态：

- `inactive`
- `active`
- `pressed`

行为：

- Shift v1 采用一次性锁存：点一次后影响下一个普通键
- Ctrl/Alt v1 采用开关式按下：再次点击释放
- 组合键发送后 Ctrl/Alt 默认释放，避免用户困在修饰状态里
- 渲染时生成一个 active-aware `Button(Text)`

## ActionKey

动作键。

类型：

- Backspace
- Enter
- Esc
- Close

行为：

- Backspace、Enter、Esc 生成虚拟键输入
- Close 关闭 KeyDock，不向目标窗口发送输入
- 渲染时生成一个 `Button(Text)`；未来可替换为 `Button(Image)`

## SpaceKey

空格键。

职责：

- 占据更宽区域
- 点击后发送 Space
- 文本显示为 `Space`
- 渲染时生成一个宽 `Button(Text)`

## 状态模型

```text
KeyboardState
  shift_latched: bool
  ctrl_active: bool
  alt_active: bool
  hovered_key: Option<KeyId>
  pressed_key: Option<KeyId>
```

状态转换原则：

- pointer move 只改变 hover
- pointer down 设置 pressed
- pointer up 如果仍命中同一 key，触发 action
- pointer cancel 清理 pressed
- 输入完成后按规则释放修饰键

## 输入命令

组件层只产生平台无关命令：

```text
InputCommand
  -> KeyTap(KeyCode)
  -> ModifiedKeyTap { modifiers, key }
  -> Text(char)
  -> CloseApp
```

`KeyCode` 是 KeyDock 自己定义的键码枚举，不直接使用 Win32 `VIRTUAL_KEY`。

## 命中测试

输入：

- 指针位置
- 当前布局快照

输出：

- `Option<KeyId>`

规则：

- 命中测试只看安全 Rust 的矩形
- 不读取窗口句柄
- 不调用 Win32 API
- 坐标单位使用逻辑像素，由平台层负责 DPI 换算

## 渲染快照

渲染层接收不可变快照：

```text
ViewSnapshot
  surface_rect
  primitive_tree
```

这样渲染层只负责画，不参与业务状态判断。

## 组件到原语映射

```text
KeyboardSurface -> Surface
DockHandle -> Row(Button(Text), Button(Text/Image))
StatusIndicator -> Row(Text, Text, Text)
KeyRow -> Row
Key -> Button(Text)
ModifierKey -> Button(Text)
ActionKey -> Button(Text/Image)
SpaceKey -> Button(Text)
```

映射规则：

- 业务组件决定状态和行为
- primitive 决定绘制结构
- `arbor-ui-windows` 决定如何把 primitive 画到 Windows 屏幕
- 新视觉能力必须先进入 `arbor-ui-core` primitive 模型，不能直接写进 Win32 renderer

## 后续扩展边界

后续可以增加主题、位置记忆、多布局，但必须先扩展组件模型，再让平台层消费。不能把新行为直接写进 Win32 消息处理里。
