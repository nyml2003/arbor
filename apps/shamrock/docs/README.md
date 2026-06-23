# Shamrock 文档

## 阅读入口

- [当前状态](./current/README.md)
- [架构设计](./architecture/README.md)
- [子系统](./systems/README.md)
- [缺陷清单](./defects/README.md)
- [参考与协作](./reference/README.md)

这组文档服务一个目标：先做 `Gen1 内容包`，但把对战引擎设计成可扩展内核。  
内容可以少，架构不能短视。

当前基线：

- 当前真实 workspace 是 7 个 crate
- replay 已支持导入导出、重放、恢复和回归校验
- `docs/current/roadmap.md` 只保留当前未完成主线
- 工程规则、注释规则和第一轮拆分计划已落地到 `docs/reference/`
