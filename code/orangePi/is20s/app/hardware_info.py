"""
Hardware Information Module for Orange Pi Zero3
システムハードウェア情報（RAM、温度、CPU、ストレージ、ネットワーク）を取得
"""
import logging
import os
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional

logger = logging.getLogger(__name__)


@dataclass
class MemoryInfo:
    """メモリ情報"""
    total_mb: int
    used_mb: int
    free_mb: int
    available_mb: int
    usage_percent: float


@dataclass
class StorageInfo:
    """ストレージ情報"""
    mount_point: str
    total_gb: float
    used_gb: float
    free_gb: float
    usage_percent: float


@dataclass
class NetworkInterface:
    """ネットワークインターフェース情報"""
    name: str
    ip_address: Optional[str]
    mac_address: Optional[str]
    is_up: bool
    is_connected: bool  # IPアドレスがあるか


@dataclass
class HardwareStatus:
    """ハードウェア総合ステータス"""
    memory: MemoryInfo
    cpu_usage_percent: float
    cpu_temp_celsius: Optional[float]
    storage: List[StorageInfo]
    network: List[NetworkInterface]
    load_average: tuple  # (1min, 5min, 15min)


def get_memory_info() -> MemoryInfo:
    """メモリ情報を取得"""
    try:
        with open("/proc/meminfo", "r") as f:
            meminfo = {}
            for line in f:
                parts = line.split()
                if len(parts) >= 2:
                    key = parts[0].rstrip(":")
                    value = int(parts[1])  # kB単位
                    meminfo[key] = value

        total_kb = meminfo.get("MemTotal", 0)
        free_kb = meminfo.get("MemFree", 0)
        available_kb = meminfo.get("MemAvailable", free_kb)
        buffers_kb = meminfo.get("Buffers", 0)
        cached_kb = meminfo.get("Cached", 0)

        # used = total - free - buffers - cached (実際の使用量)
        used_kb = total_kb - available_kb

        return MemoryInfo(
            total_mb=total_kb // 1024,
            used_mb=used_kb // 1024,
            free_mb=free_kb // 1024,
            available_mb=available_kb // 1024,
            usage_percent=round((used_kb / total_kb) * 100, 1) if total_kb > 0 else 0.0,
        )
    except Exception as e:
        logger.warning("Failed to get memory info: %s", e)
        return MemoryInfo(0, 0, 0, 0, 0.0)


def get_cpu_usage() -> float:
    """CPU使用率を取得（簡易版: 1秒間の変化を測定）"""
    try:
        # /proc/statから取得
        def read_cpu_stat():
            with open("/proc/stat", "r") as f:
                line = f.readline()
                if line.startswith("cpu "):
                    parts = line.split()[1:]
                    return [int(x) for x in parts]
            return None

        stat1 = read_cpu_stat()
        if not stat1:
            return 0.0

        # 即時の使用率を計算（前回との差分ではなく累積から）
        # user, nice, system, idle, iowait, irq, softirq, steal
        if len(stat1) >= 4:
            idle = stat1[3]
            if len(stat1) >= 5:
                idle += stat1[4]  # iowait
            total = sum(stat1[:8]) if len(stat1) >= 8 else sum(stat1)
            if total > 0:
                return round(((total - idle) / total) * 100, 1)
        return 0.0
    except Exception as e:
        logger.warning("Failed to get CPU usage: %s", e)
        return 0.0


def get_cpu_temperature() -> Optional[float]:
    """CPU温度を取得（℃）"""
    thermal_zones = [
        "/sys/class/thermal/thermal_zone0/temp",
        "/sys/class/thermal/thermal_zone1/temp",
        "/sys/devices/virtual/thermal/thermal_zone0/temp",
    ]

    for zone in thermal_zones:
        try:
            if Path(zone).exists():
                with open(zone, "r") as f:
                    temp = int(f.read().strip())
                    # millidegree -> degree
                    return round(temp / 1000.0, 1)
        except Exception:
            continue

    # vcgencmd (Raspberry Pi / some SBCs)
    try:
        result = subprocess.run(
            ["vcgencmd", "measure_temp"],
            capture_output=True, text=True, timeout=2
        )
        if result.returncode == 0:
            match = re.search(r"temp=(\d+\.?\d*)", result.stdout)
            if match:
                return float(match.group(1))
    except Exception:
        pass

    return None


def get_storage_info() -> List[StorageInfo]:
    """ストレージ情報を取得"""
    result = []
    target_mounts = ["/", "/var/lib/is20s"]

    for mount_point in target_mounts:
        try:
            if os.path.exists(mount_point):
                stat = os.statvfs(mount_point)
                total = stat.f_blocks * stat.f_frsize
                free = stat.f_bavail * stat.f_frsize
                used = total - free

                result.append(StorageInfo(
                    mount_point=mount_point,
                    total_gb=round(total / (1024 ** 3), 2),
                    used_gb=round(used / (1024 ** 3), 2),
                    free_gb=round(free / (1024 ** 3), 2),
                    usage_percent=round((used / total) * 100, 1) if total > 0 else 0.0,
                ))
        except Exception as e:
            logger.debug("Failed to get storage info for %s: %s", mount_point, e)

    return result


def get_network_interfaces() -> List[NetworkInterface]:
    """ネットワークインターフェース情報を取得"""
    interfaces = []
    target_ifaces = ["eth0", "end0", "wlan0"]

    for iface in target_ifaces:
        try:
            # インターフェースの存在確認
            sys_path = Path(f"/sys/class/net/{iface}")
            if not sys_path.exists():
                continue

            # MACアドレス
            mac = None
            mac_path = sys_path / "address"
            if mac_path.exists():
                mac = mac_path.read_text().strip()
                if mac == "00:00:00:00:00:00":
                    mac = None

            # 状態 (UP/DOWN)
            is_up = False
            operstate_path = sys_path / "operstate"
            if operstate_path.exists():
                state = operstate_path.read_text().strip()
                is_up = state in ("up", "unknown")

            # IPアドレス
            ip_addr = None
            try:
                result = subprocess.run(
                    ["ip", "-4", "addr", "show", iface],
                    capture_output=True, text=True, timeout=2
                )
                if result.returncode == 0:
                    match = re.search(r"inet (\d+\.\d+\.\d+\.\d+)", result.stdout)
                    if match:
                        ip_addr = match.group(1)
            except Exception:
                pass

            interfaces.append(NetworkInterface(
                name=iface,
                ip_address=ip_addr,
                mac_address=mac,
                is_up=is_up,
                is_connected=ip_addr is not None,
            ))
        except Exception as e:
            logger.debug("Failed to get interface info for %s: %s", iface, e)

    return interfaces


def get_load_average() -> tuple:
    """ロードアベレージを取得"""
    try:
        return os.getloadavg()
    except Exception:
        return (0.0, 0.0, 0.0)


def get_hardware_status() -> HardwareStatus:
    """ハードウェア総合ステータスを取得"""
    return HardwareStatus(
        memory=get_memory_info(),
        cpu_usage_percent=get_cpu_usage(),
        cpu_temp_celsius=get_cpu_temperature(),
        storage=get_storage_info(),
        network=get_network_interfaces(),
        load_average=get_load_average(),
    )


def get_hardware_status_dict() -> Dict[str, Any]:
    """ハードウェアステータスを辞書形式で取得（API用）"""
    status = get_hardware_status()
    return {
        "memory": {
            "total_mb": status.memory.total_mb,
            "used_mb": status.memory.used_mb,
            "free_mb": status.memory.free_mb,
            "available_mb": status.memory.available_mb,
            "usage_percent": status.memory.usage_percent,
        },
        "cpu": {
            "usage_percent": status.cpu_usage_percent,
            "temperature_celsius": status.cpu_temp_celsius,
            "load_average": {
                "1min": round(status.load_average[0], 2),
                "5min": round(status.load_average[1], 2),
                "15min": round(status.load_average[2], 2),
            },
        },
        "storage": [
            {
                "mount": s.mount_point,
                "total_gb": s.total_gb,
                "used_gb": s.used_gb,
                "free_gb": s.free_gb,
                "usage_percent": s.usage_percent,
            }
            for s in status.storage
        ],
        "network": [
            {
                "name": n.name,
                "ip": n.ip_address,
                "mac": n.mac_address,
                "is_up": n.is_up,
                "is_connected": n.is_connected,
            }
            for n in status.network
        ],
    }


__all__ = [
    "get_hardware_status",
    "get_hardware_status_dict",
    "get_memory_info",
    "get_cpu_usage",
    "get_cpu_temperature",
    "get_storage_info",
    "get_network_interfaces",
    "get_load_average",
]
