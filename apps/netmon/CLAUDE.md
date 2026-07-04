# netmon 开发指引

## 概述

纯 Python 脚本，Windows 网络连通性监控工具。单文件 ~250 行，轮询式 TCP 探测，零外部依赖。

## 构建 / 运行

```bash
# 直接运行（无需构建）
python netmon.py

# 语法检查
python -c "import py_compile; py_compile.compile('netmon.py', doraise=True)"
```

## 架构

```
Monitor.run()
  └─ while running:
       time.sleep(interval)
       _do_probe()
         └─ Prober.probe()          # TCP connect → ICMP ping 备选
              └─ OutageTracker      # record_down / record_up → summary()
                   └─ Logger        # 控制台 + 文件双写
```

## 关键约束

- **纯 stdlib**：不加任何 pip 依赖
- **轮询探测**：`socket.create_connection()` 测试可达性，简单可靠
- **双重确认**：TCP 全部失败后用 ping 再确认，避免单点误判
- **只记状态变化**：DOWN → UP、UP → DOWN 才输出，重复探测不输出
- **Windows 控制台安全**：只输出 ASCII 前缀（`!` / `+`），避免 GBK 编码错误

## 修改指引

- 改检测逻辑 → `Prober` 类
- 改日志格式 → `Logger` 类
- 改统计摘要 → `OutageTracker.summary()` / `Summary` dataclass
- 加命令行参数 → `parse_args()` + `Config` dataclass
- 改主循环行为 → `Monitor._main_loop()` / `Monitor._do_probe()`
