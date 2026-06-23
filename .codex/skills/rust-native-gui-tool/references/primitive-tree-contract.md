# Primitive Tree 合同

## 意图

让组件 DSL 产出平台无关绘制描述，而不是平台控件或系统资源。

## 适用场景

- 修改 `arbor-ui-core` 组件。
- 增加 `Surface`、`Row`、`Button`、`Text`、`Image` 等节点能力。
- 修改 `arbor-ui-windows` 渲染映射。

## 必须遵守的规则

- primitive 节点只描述“画什么”和“基本交互状态”。
- 节点不能持有 HWND、COM、Direct2D brush、DirectWrite format 或 bitmap。
- app 层负责布局，renderer 不重新解释业务布局。
- `Button` 描述可点击视觉状态，不直接产生业务命令。
- `Image` 引用资源 ID，不直接持有文件路径或解码结果。

## 推荐模式

- DSL builder 输出不可变 tree 或 view snapshot。
- 资源缓存放在 renderer adapter。
- 组件共享真实共性，例如 `id`、`rect`、`children`。
- 新节点先确认至少两个使用场景，再抽到共享 crate。

## 反模式

- 组件节点持有平台资源。
- 把业务命令塞进组件节点。
- 平台层根据节点 id 推断产品语义。
- 为了单个 app 的需求污染共享 primitive。

## 证据

- `workspace/learn/patterns/rust-native-gui-dsl.md` 记录 primitive tree 是 app 和平台之间的合同。
- `apps/keydock/docs/architecture.md` 规定 `arbor-ui-core` components 不知道窗口句柄、DPI、COM 或 Win32 错误。
