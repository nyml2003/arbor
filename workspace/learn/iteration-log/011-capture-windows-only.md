# 011 - 收缩为 Windows-only

日期：2026-06-07

## 做了什么

1. 移除 `xcap` 依赖
2. 删除截图服务里的非 Windows fallback 路径
3. 把当前 MVP 明确改成 Windows-only
4. 文档里去掉“暂时为 macOS/Linux 预留”的表述

## 学到了什么

### 跨端承诺会稀释当前最重要的问题

现在真正难的是：

- Windows HDR 截图正确性
- 剪贴板稳定性
- toast 交互可靠性

如果这个时候还继续保留跨端 fallback，代码里就会同时存在两套甚至三套心智模型。对 MVP 没帮助，只会增加回退路径和误判。

### 平台专属能力就该平台专属维护

截图不是普通业务逻辑。它依赖：

- 操作系统的颜色管线
- 桌面合成器
- 显示器 HDR / SDR 模式
- 剪贴板格式

这类东西本来就不适合被一个“通用抽象”轻易抹平。当前阶段最合理的做法是先把 Windows 路径做对。

## 决策

1. 当前 MVP 只支持 Windows
2. Windows 截图后端单独维护
3. 非 Windows 平台现在明确返回 unsupported，而不是保留弱 fallback

## 下一步

- 继续修 Windows HDR
- 修 toast 收口
- 做完整人工验证
