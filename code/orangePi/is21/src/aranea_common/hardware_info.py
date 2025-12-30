"""
HardwareInfo - ハードウェア情報取得（NPU対応）
is20s hardware_info.pyの拡張版

Orange Pi 5 Plus (RK3588) 向けの詳細情報:
- CPU: 8コア（4x A76 + 4x A55）
- NPU: RKNPU 6TOPS
- RAM: 16GB LPDDR5
- 温度: CPU/NPU/GPU個別監視
"""

import os
import logging
import subprocess
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, field
from datetime import datetime

logger = logging.getLogger(__name__)


@dataclass
class MemoryInfo:
    """メモリ情報"""
    total_mb: int = 0
    used_mb: int = 0
    free_mb: int = 0
    available_mb: int = 0
    percent_used: float = 0.0


@dataclass
class StorageInfo:
    """ストレージ情報"""
    path: str = "/"
    total_gb: float = 0.0
    used_gb: float = 0.0
    free_gb: float = 0.0
    percent_used: float = 0.0


@dataclass
class CpuInfo:
    """CPU情報"""
    model: str = ""
    cores: int = 0
    architecture: str = ""
    load_1min: float = 0.0
    load_5min: float = 0.0
    load_15min: float = 0.0
    usage_percent: float = 0.0


@dataclass
class ThermalInfo:
    """温度情報"""
    cpu_temp: float = 0.0
    npu_temp: float = 0.0
    gpu_temp: float = 0.0
    soc_temp: float = 0.0
    # 各ゾーンの詳細
    zones: Dict[str, float] = field(default_factory=dict)


@dataclass
class NpuInfo:
    """NPU情報"""
    available: bool = False
    driver_version: str = ""
    cores: int = 3  # RK3588は3コア
    model_loaded: bool = False
    current_model: str = ""
    inference_count: int = 0
    last_inference_ms: float = 0.0
    total_inference_ms: float = 0.0


@dataclass
class NetworkInterface:
    """ネットワークインターフェース情報"""
    name: str = ""
    mac: str = ""
    ipv4: str = ""
    ipv6: str = ""
    is_up: bool = False
    rx_bytes: int = 0
    tx_bytes: int = 0


class HardwareInfo:
    """
    ハードウェア情報取得クラス
    Orange Pi 5 Plus (RK3588) 向け最適化
    """

    def __init__(self):
        self._npu_info = NpuInfo()
        self._inference_stats: Dict[str, Any] = {
            "count": 0,
            "total_ms": 0.0,
            "last_ms": 0.0
        }

    def get_memory(self) -> MemoryInfo:
        """メモリ情報を取得"""
        info = MemoryInfo()
        try:
            with open("/proc/meminfo", "r") as f:
                meminfo = {}
                for line in f:
                    parts = line.split(":")
                    if len(parts) == 2:
                        key = parts[0].strip()
                        val = parts[1].strip().split()[0]  # kB単位
                        meminfo[key] = int(val)

                info.total_mb = meminfo.get("MemTotal", 0) // 1024
                info.free_mb = meminfo.get("MemFree", 0) // 1024
                info.available_mb = meminfo.get("MemAvailable", 0) // 1024
                info.used_mb = info.total_mb - info.available_mb
                if info.total_mb > 0:
                    info.percent_used = round((info.used_mb / info.total_mb) * 100, 1)

        except Exception as e:
            logger.error(f"[HW] Failed to get memory info: {e}")

        return info

    def get_storage(self, path: str = "/") -> StorageInfo:
        """ストレージ情報を取得"""
        info = StorageInfo(path=path)
        try:
            stat = os.statvfs(path)
            total = stat.f_blocks * stat.f_frsize
            free = stat.f_bavail * stat.f_frsize
            used = total - free

            info.total_gb = round(total / (1024**3), 2)
            info.free_gb = round(free / (1024**3), 2)
            info.used_gb = round(used / (1024**3), 2)
            if info.total_gb > 0:
                info.percent_used = round((info.used_gb / info.total_gb) * 100, 1)

        except Exception as e:
            logger.error(f"[HW] Failed to get storage info for {path}: {e}")

        return info

    def get_cpu(self) -> CpuInfo:
        """CPU情報を取得"""
        info = CpuInfo()
        try:
            # モデル名
            with open("/proc/cpuinfo", "r") as f:
                for line in f:
                    if line.startswith("Model") or line.startswith("model name"):
                        info.model = line.split(":")[1].strip()
                        break
                    elif line.startswith("Hardware"):
                        info.model = line.split(":")[1].strip()

            # コア数
            info.cores = os.cpu_count() or 0

            # アーキテクチャ
            try:
                result = subprocess.run(["uname", "-m"], capture_output=True, text=True)
                info.architecture = result.stdout.strip()
            except:
                pass

            # 負荷（load average）
            with open("/proc/loadavg", "r") as f:
                parts = f.read().split()
                info.load_1min = float(parts[0])
                info.load_5min = float(parts[1])
                info.load_15min = float(parts[2])

            # CPU使用率（簡易計算）
            if info.cores > 0:
                info.usage_percent = round((info.load_1min / info.cores) * 100, 1)
                if info.usage_percent > 100:
                    info.usage_percent = 100.0

        except Exception as e:
            logger.error(f"[HW] Failed to get CPU info: {e}")

        return info

    def get_thermal(self) -> ThermalInfo:
        """温度情報を取得"""
        info = ThermalInfo()
        try:
            thermal_base = "/sys/class/thermal"
            if os.path.isdir(thermal_base):
                for zone in os.listdir(thermal_base):
                    if zone.startswith("thermal_zone"):
                        zone_path = os.path.join(thermal_base, zone)
                        try:
                            # ゾーン名
                            type_path = os.path.join(zone_path, "type")
                            temp_path = os.path.join(zone_path, "temp")

                            zone_type = ""
                            if os.path.exists(type_path):
                                with open(type_path, "r") as f:
                                    zone_type = f.read().strip()

                            temp = 0.0
                            if os.path.exists(temp_path):
                                with open(temp_path, "r") as f:
                                    temp = float(f.read().strip()) / 1000.0

                            info.zones[zone_type or zone] = round(temp, 1)

                            # タイプ別に振り分け
                            zone_lower = zone_type.lower()
                            if "cpu" in zone_lower or "littlecore" in zone_lower or "bigcore" in zone_lower:
                                if temp > info.cpu_temp:
                                    info.cpu_temp = round(temp, 1)
                            elif "npu" in zone_lower:
                                info.npu_temp = round(temp, 1)
                            elif "gpu" in zone_lower:
                                info.gpu_temp = round(temp, 1)
                            elif "soc" in zone_lower or "center" in zone_lower:
                                info.soc_temp = round(temp, 1)

                        except Exception as e:
                            logger.debug(f"[HW] Error reading thermal zone {zone}: {e}")

        except Exception as e:
            logger.error(f"[HW] Failed to get thermal info: {e}")

        return info

    def get_npu(self) -> NpuInfo:
        """NPU情報を取得"""
        info = NpuInfo()

        # RKNNデバイスの存在確認
        rknpu_devices = [
            "/dev/rknpu",
            "/dev/dri/renderD128"  # RK3588のNPUはDRMデバイス経由
        ]

        for dev in rknpu_devices:
            if os.path.exists(dev):
                info.available = True
                break

        if not info.available:
            # カーネルモジュール確認
            try:
                result = subprocess.run(["lsmod"], capture_output=True, text=True)
                if "rknpu" in result.stdout:
                    info.available = True
            except:
                pass

        # RKNNドライババージョン
        try:
            dmesg_result = subprocess.run(
                ["dmesg"],
                capture_output=True,
                text=True
            )
            for line in dmesg_result.stdout.split("\n"):
                if "RKNPU" in line and "driver" in line.lower():
                    # 例: "RKNPU driver version: 0.8.8"
                    if "version" in line.lower():
                        parts = line.split(":")
                        if len(parts) >= 2:
                            info.driver_version = parts[-1].strip()
                            break
        except:
            pass

        # 推論統計を反映
        info.inference_count = self._inference_stats.get("count", 0)
        info.last_inference_ms = self._inference_stats.get("last_ms", 0.0)
        info.total_inference_ms = self._inference_stats.get("total_ms", 0.0)
        info.model_loaded = self._npu_info.model_loaded
        info.current_model = self._npu_info.current_model

        return info

    def update_inference_stats(self, inference_ms: float, model_name: str = "") -> None:
        """推論統計を更新（main.pyから呼び出し）"""
        self._inference_stats["count"] += 1
        self._inference_stats["last_ms"] = inference_ms
        self._inference_stats["total_ms"] += inference_ms
        if model_name:
            self._npu_info.model_loaded = True
            self._npu_info.current_model = model_name

    def set_model_loaded(self, loaded: bool, model_name: str = "") -> None:
        """モデルロード状態を設定"""
        self._npu_info.model_loaded = loaded
        self._npu_info.current_model = model_name

    def get_network_interfaces(self) -> List[NetworkInterface]:
        """ネットワークインターフェース情報を取得"""
        interfaces = []
        try:
            net_dir = "/sys/class/net"
            if os.path.isdir(net_dir):
                for iface_name in os.listdir(net_dir):
                    if iface_name == "lo":
                        continue

                    iface = NetworkInterface(name=iface_name)
                    iface_path = os.path.join(net_dir, iface_name)

                    # MAC
                    try:
                        with open(os.path.join(iface_path, "address"), "r") as f:
                            iface.mac = f.read().strip().upper()
                    except:
                        pass

                    # 状態
                    try:
                        with open(os.path.join(iface_path, "operstate"), "r") as f:
                            iface.is_up = f.read().strip().lower() == "up"
                    except:
                        pass

                    # RX/TXバイト
                    try:
                        with open(os.path.join(iface_path, "statistics/rx_bytes"), "r") as f:
                            iface.rx_bytes = int(f.read().strip())
                        with open(os.path.join(iface_path, "statistics/tx_bytes"), "r") as f:
                            iface.tx_bytes = int(f.read().strip())
                    except:
                        pass

                    # IPアドレス（ip コマンド使用）
                    try:
                        result = subprocess.run(
                            ["ip", "-j", "addr", "show", iface_name],
                            capture_output=True,
                            text=True
                        )
                        import json
                        data = json.loads(result.stdout)
                        if data and len(data) > 0:
                            for addr_info in data[0].get("addr_info", []):
                                if addr_info.get("family") == "inet":
                                    iface.ipv4 = addr_info.get("local", "")
                                elif addr_info.get("family") == "inet6":
                                    if not addr_info.get("local", "").startswith("fe80"):
                                        iface.ipv6 = addr_info.get("local", "")
                    except:
                        pass

                    interfaces.append(iface)

        except Exception as e:
            logger.error(f"[HW] Failed to get network interfaces: {e}")

        return interfaces

    def get_uptime(self) -> Dict[str, Any]:
        """稼働時間を取得"""
        info = {
            "uptime_seconds": 0,
            "uptime_formatted": "",
            "boot_time": ""
        }
        try:
            with open("/proc/uptime", "r") as f:
                uptime_seconds = float(f.read().split()[0])
                info["uptime_seconds"] = int(uptime_seconds)

                # フォーマット
                days = int(uptime_seconds // 86400)
                hours = int((uptime_seconds % 86400) // 3600)
                minutes = int((uptime_seconds % 3600) // 60)
                if days > 0:
                    info["uptime_formatted"] = f"{days}d {hours}h {minutes}m"
                else:
                    info["uptime_formatted"] = f"{hours}h {minutes}m"

                # ブート時刻
                from datetime import datetime, timedelta
                boot_time = datetime.now() - timedelta(seconds=uptime_seconds)
                info["boot_time"] = boot_time.isoformat()

        except Exception as e:
            logger.error(f"[HW] Failed to get uptime: {e}")

        return info

    def get_all(self) -> Dict[str, Any]:
        """全ハードウェア情報を取得"""
        memory = self.get_memory()
        storage = self.get_storage()
        cpu = self.get_cpu()
        thermal = self.get_thermal()
        npu = self.get_npu()
        uptime = self.get_uptime()
        interfaces = self.get_network_interfaces()

        return {
            "timestamp": datetime.now().isoformat(),
            "memory": {
                "total_mb": memory.total_mb,
                "used_mb": memory.used_mb,
                "free_mb": memory.free_mb,
                "available_mb": memory.available_mb,
                "percent_used": memory.percent_used
            },
            "storage": {
                "path": storage.path,
                "total_gb": storage.total_gb,
                "used_gb": storage.used_gb,
                "free_gb": storage.free_gb,
                "percent_used": storage.percent_used
            },
            "cpu": {
                "model": cpu.model,
                "cores": cpu.cores,
                "architecture": cpu.architecture,
                "load_1min": cpu.load_1min,
                "load_5min": cpu.load_5min,
                "load_15min": cpu.load_15min,
                "usage_percent": cpu.usage_percent
            },
            "thermal": {
                "cpu_temp": thermal.cpu_temp,
                "npu_temp": thermal.npu_temp,
                "gpu_temp": thermal.gpu_temp,
                "soc_temp": thermal.soc_temp,
                "zones": thermal.zones
            },
            "npu": {
                "available": npu.available,
                "driver_version": npu.driver_version,
                "cores": npu.cores,
                "model_loaded": npu.model_loaded,
                "current_model": npu.current_model,
                "inference_count": npu.inference_count,
                "last_inference_ms": npu.last_inference_ms,
                "total_inference_ms": npu.total_inference_ms
            },
            "uptime": uptime,
            "network": [
                {
                    "name": iface.name,
                    "mac": iface.mac,
                    "ipv4": iface.ipv4,
                    "ipv6": iface.ipv6,
                    "is_up": iface.is_up,
                    "rx_bytes": iface.rx_bytes,
                    "tx_bytes": iface.tx_bytes
                }
                for iface in interfaces
            ]
        }

    def get_summary(self) -> Dict[str, Any]:
        """概要情報（軽量版）"""
        memory = self.get_memory()
        cpu = self.get_cpu()
        thermal = self.get_thermal()

        return {
            "memory_percent": memory.percent_used,
            "cpu_load": cpu.load_1min,
            "cpu_temp": thermal.cpu_temp,
            "npu_temp": thermal.npu_temp,
            "uptime": self.get_uptime()["uptime_formatted"]
        }
