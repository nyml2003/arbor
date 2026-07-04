# netmon — 网络连通性监控

定时轮询式网络连通性监控工具。ICMP ping 优先（Windows 断网瞬间返回失败）+ TCP 备选，断网/恢复时记录时间点，Ctrl+C 出统计摘要。

## 特点

- **Ping 优先**：ICMP ping 探测，Windows 断网时瞬间返回 "General failure"，不等待超时
- **TCP 备选**：ping 全部失败后用 TCP 连接 53 端口再确认（穿透封锁 ICMP 的网络）
- **零依赖**：纯 Python stdlib，无需 pip install
- **日志双写**：控制台实时输出 + UTF-8 日志文件
- **统计摘要**：Ctrl+C 退出时打印总断网次数、累计断网时长、可用率

## 用法

```bash
# 默认运行（2s 间隔，ping 优先）
python netmon.py

# 查看每次探测详情
python netmon.py -v

# 自定义间隔和目标
python netmon.py -i 5 -t 114.114.114.114 -t 223.5.5.5

# 安静模式（仅输出状态变化）
python netmon.py -q

# 纯 ICMP 模式（禁用 TCP 备选）
python netmon.py --no-tcp-fallback
```

## 命令行参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `-i, --interval SEC` | `2` | 探测间隔 |
| `-t, --targets HOST` | `8.8.8.8 1.1.1.1 baidu.com` | 探测目标，可多次指定 |
| `--log PATH` | `./netmon.log` | 日志文件路径 |
| `--timeout SEC` | `2` | 单次探测超时 |
| `--no-tcp-fallback` | 关闭 | 禁用 TCP 备选 |
| `-v, --verbose` | 关闭 | 显示每次探测详情 |
| `-q, --quiet` | 关闭 | 仅输出状态变化 |

## 输出示例

```
[14:32:15] + Monitor started (targets=8.8.8.8, 1.1.1.1, baidu.com, interval=2.0s, mode=+tcp)
[14:32:15] + Network UP (initial check)
[14:35:22] ! Network DOWN
[14:38:07] + Network UP (was down for 2m 45s)
^C
=== Summary ===
  Total runtime:     1h 28m 0s
  Total outages:     2
  Total downtime:    3m 18s
  Longest outage:    2m 45s
  Availability:      96.27%
===============
```

verbose 模式（`-v`）额外输出：
```
[14:32:17] ~ [UP] ping 8.8.8.8: ok
[14:32:19] ~ [UP] ping 8.8.8.8: ok
[14:32:21] ~ [DOWN] all targets unreachable
```

- `+` = INFO，`!` = WARN，`~` = DEBUG
