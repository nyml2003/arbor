"""
WiFi 信号数据采集脚本
用法: python wifi_logger.py http://<手机IP>:8765/data

每 2 秒轮询手机 HTTP Server，实时打印信号数据，
同时保存到 wifi_log.jsonl 和 trajectory.geojson。
"""

import sys
import time
import json
import urllib.request
from datetime import datetime, timezone


def fetch(url: str) -> dict | None:
    """从手机 HTTP Server 拉取当前数据"""
    try:
        with urllib.request.urlopen(url, timeout=3) as resp:
            return json.loads(resp.read().decode())
    except Exception as e:
        print(f"\033[31m请求失败: {e}\033[0m")
        return None


def format_rssi_bar(rssi: int) -> str:
    """可视化信号强度条"""
    pct = max(0, min(100, (rssi + 100) * 100 // 70))
    filled = pct // 5
    bar = "█" * filled + "░" * (20 - filled)
    return f"[{bar}] {pct}%"


def rssi_color(rssi: int) -> str:
    if rssi >= -50:
        return "\033[32m"  # 绿色
    elif rssi >= -65:
        return "\033[36m"  # 青色
    elif rssi >= -75:
        return "\033[33m"  # 黄色
    elif rssi >= -85:
        return "\033[91m"  # 亮红
    else:
        return "\033[31m"  # 红色


def main():
    if len(sys.argv) < 2:
        url = "http://192.168.1.100:8765/data"
        print(f"用法: python {sys.argv[0]} <手机URL>")
        print(f"示例: python {sys.argv[0]} {url}")
        print("(URL 会在手机 App 界面上显示)")
        # 尝试默认 URL
        print(f"\n尝试默认地址…")
        # 也尝试从常见的网关段扫描
        import socket
        # 直接用默认的试试
    else:
        url = sys.argv[1]

    log_file = open("wifi_log.jsonl", "a", encoding="utf-8")
    geo_features: list[dict] = []

    print("=" * 55)
    print("  WiFi 信号采集器 — 实时记录中")
    print(f"  数据源: {url}")
    print("  Ctrl+C 停止")
    print("=" * 55)

    last_traj_len = 0
    start_time = time.time()
    sample_count = 0

    try:
        while True:
            data = fetch(url)
            if data is None:
                time.sleep(2)
                continue

            sample_count += 1
            rssi = data.get("rssi", -100)
            ssid = data.get("ssid", "?")
            bssid = data.get("bssid", "?")
            band = data.get("band", "?")
            distance = data.get("distance", "?")
            lat = data.get("lat", 0)
            lng = data.get("lng", 0)
            ts = data.get("timestamp", 0)
            t = datetime.fromtimestamp(ts / 1000).strftime("%H:%M:%S") if ts else "--:--:--"

            color = rssi_color(rssi)

            # 实时打印
            print(f"\r\033[K{t} │ {color}{rssi:4d} dBm\033[0m │ {format_rssi_bar(rssi)} │ {distance:12s}", end="")
            if lat and lng:
                sys.stdout.write(f" │ 📍 {lat:.5f},{lng:.5f}")
            sys.stdout.write("\n")
            sys.stdout.flush()

            # 写入 JSONL
            data["_collected_at"] = datetime.now(timezone.utc).isoformat()
            log_file.write(json.dumps(data, ensure_ascii=False) + "\n")
            log_file.flush()

            # 更新 GeoJSON 轨迹
            traj = data.get("trajectory", [])
            if len(traj) > last_traj_len:
                last_traj_len = len(traj)
                geo_features = [
                    {
                        "type": "Feature",
                        "geometry": {"type": "Point", "coordinates": [p["lng"], p["lat"]]},
                        "properties": {
                            "rssi": p["rssi"],
                            "accuracy": p.get("accuracy", 0),
                            "timestamp": p["timestamp"]
                        }
                    }
                    for p in traj
                ]
                geojson = {
                    "type": "FeatureCollection",
                    "features": geo_features
                }
                with open("trajectory.geojson", "w", encoding="utf-8") as f:
                    json.dump(geojson, f, ensure_ascii=False, indent=2)

            time.sleep(2)

    except KeyboardInterrupt:
        elapsed = time.time() - start_time
        print("\n")
        print("=" * 55)
        print(f"  采集完毕")
        print(f"  样本数: {sample_count}")
        print(f"  耗时: {elapsed:.0f} 秒")
        print(f"  输出文件:")
        print(f"    wifi_log.jsonl       — 原始日志")
        if geo_features:
            print(f"    trajectory.geojson   — GPS 轨迹 ({len(geo_features)} 点)")
            print(f"    → 可拖入 https://geojson.io 查看地图")
        print("=" * 55)
    finally:
        log_file.close()


if __name__ == "__main__":
    main()
