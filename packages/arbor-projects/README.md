# Arbor Projects

arbor-projects 是 Arbor 仓库内的项目注册表和本地验证入口。它只使用 Python 3 标准库，不需要安装 Python 依赖。

## 边界

- arbor_projects.domain：项目、验证命令、覆盖率目标和验证报告。
- arbor_projects.application：按顺序执行项目门禁，并在首次命令失败时停止。
- arbor_projects.adapters：JSON 注册表、子进程和 LLVM coverage export。
- arbor_projects.cli：参数解析、文本输出和退出码。

领域模型使用不可变 @dataclass(frozen=True, slots=True)。应用端口使用 typing.Protocol。文件系统和进程调用不进入 domain/application。

projects.json 注册本期五个项目：

- punctum
- tetris
- ramus
- gen3-game
- tui-chater

## 使用

从仓库根目录运行：

~~~text
python packages/arbor-projects/arbor_projects list
python packages/arbor-projects/arbor_projects verify tetris
~~~

也可以进入工程目录运行：

~~~text
cd packages/arbor-projects
python -m arbor_projects list
python -m arbor_projects verify tetris
~~~

验证器直接执行注册表中的 argv，不使用 shell。verify 返回非零退出码时表示项目不存在、命令失败、覆盖率不完整或基础设施读取失败。

## 测试

~~~text
cd packages/arbor-projects
python -m unittest discover -s tests -v
python scripts/check_pure_coverage.py
~~~

纯逻辑覆盖率脚本使用标准库 trace。它要求 domain、application、JSON 格式解析和 LLVM export 解析的可执行语句行全部覆盖。
