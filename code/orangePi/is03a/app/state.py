from dataclasses import dataclass, field, asdict
from typing import Any, Dict, Optional


@dataclass
class RuntimeState:
    lacis_id: Optional[str] = None
    cic: Optional[str] = None
    mac12: Optional[str] = None
    state_endpoint: Optional[str] = None
    mqtt_endpoint: Optional[str] = None
    mqtt_connected: bool = False
    last_forward_ok_at: Optional[str] = None
    forward_failures: int = 0
    last_command: Optional[Dict[str, Any]] = None
    last_command_status: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


__all__ = ["RuntimeState"]
