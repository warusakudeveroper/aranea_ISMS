import os
import socket
from typing import Dict, Optional


def uptime_seconds() -> Optional[float]:
    try:
        with open("/proc/uptime", "r") as f:
            return float(f.read().split()[0])
    except Exception:
        return None


def load_average():
    try:
        return os.getloadavg()
    except Exception:
        return None


def memory_info() -> Dict[str, int]:
    info: Dict[str, int] = {}
    try:
        with open("/proc/meminfo", "r") as f:
            for line in f:
                key, value = line.split(":")
                info[key.strip()] = int(value.strip().split()[0])  # kB
    except Exception:
        pass
    return info


def system_status() -> Dict[str, object]:
    return {
        "hostname": socket.gethostname(),
        "uptimeSec": uptime_seconds(),
        "loadavg": load_average(),
        "meminfo": memory_info(),
    }


__all__ = ["system_status", "uptime_seconds", "load_average", "memory_info"]
