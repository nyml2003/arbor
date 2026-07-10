# Punctum

Punctum 是一个实验性的离散网格 UI 与渲染项目。

项目先验证一种共享模型：应用把状态绘制到二维网格表面，终端和图形窗口分别负责提交该表面。Punctum 不试图让终端模拟完整 GPU，也不把游戏引擎塞进 UI 核心。

当前阶段只维护提案和原型。技术决策记录在 [`peps/`](peps/README.md)。

## 当前方向

- 实现语言：Rust stable
- 核心模型：泛型离散表面 `Surface<T>` 与增量提交 `Patch<T>`
- 终端原型：Crossterm
- 图形原型：winit + wgpu
- UI 范式：先采用 immediate paint，组件和布局后置
- 非目标：完整游戏引擎、像素级 GUI、统一所有输入和渲染能力

下一步按 [PEP 0001](peps/0001-punctum-technical-direction.md) 中的三个实验推进。
