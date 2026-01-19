# DD06: Linux共通モジュール詳細設計

作成日: 2026-01-10
最終更新: 2026-01-10
バージョン: 1.0.0

---

## 概要

Linux系デバイス（IS20S/IS21/IS22）で共通利用する基盤モジュールの詳細設計。
ESP32 global/srcをリファレンスとし、Python/Rust両言語での実装パターンを定義する。

---

## 設計原則

### SSoT（Single Source of Truth）

| 責務 | モジュール | 管理対象 |
|------|-----------|---------|
| 設定永続化 | ConfigManager | JSON設定ファイル |
| デバイス識別 | LacisIdGenerator | LacisID |
| クラウド登録 | AraneaRegister | CIC/エンドポイント |
| 状態監視 | HardwareInfo | システムリソース |
| MQTT通信 | MqttManager | Pub/Sub |
| 状態報告 | StateReporter | デバイス状態 |

### MECE分類

```
Linux共通モジュール
├── 基盤モジュール（全デバイス必須）
│   ├── ConfigManager       # 設定管理
│   ├── LacisIdGenerator    # LacisID生成
│   ├── AraneaRegister      # デバイス登録
│   └── HardwareInfo        # ハードウェア情報
│
├── 通信モジュール（選択的）
│   ├── MqttManager         # MQTTクライアント
│   └── StateReporter       # 状態報告
│
└── UIモジュール（オプション）
    ├── AraneaWebUI         # Web UI CSS/テンプレート
    └── ApiRoutes           # REST API基盤
```

---

## モジュール詳細

### 1. ConfigManager（設定管理）

**ESP32参照**: `SettingManager.cpp`

#### インターフェース（Python）

```python
class ConfigManager:
    """
    JSON永続化による設定管理
    ESP32 SettingManager（NVS）のPython移植版
    """

    def __init__(self, config_dir: str = "/opt/{device}/config"):
        """初期化"""

    def begin(self, namespace: str) -> bool:
        """名前空間を指定して初期化"""

    def get_string(self, key: str, default: str = "") -> str:
        """文字列設定を取得"""

    def set_string(self, key: str, value: str) -> None:
        """文字列設定を保存"""

    def get_int(self, key: str, default: int = 0) -> int:
        """整数設定を取得"""

    def set_int(self, key: str, value: int) -> None:
        """整数設定を保存"""

    def get_bool(self, key: str, default: bool = False) -> bool:
        """bool設定を取得"""

    def set_bool(self, key: str, value: bool) -> None:
        """bool設定を保存"""

    def get_dict(self, key: str, default: dict = None) -> dict:
        """辞書設定を取得"""

    def set_dict(self, key: str, value: dict) -> None:
        """辞書設定を保存"""

    def has_key(self, key: str) -> bool:
        """キーの存在確認"""

    def remove(self, key: str) -> bool:
        """キーを削除"""

    def clear(self) -> None:
        """全設定をクリア"""
```

#### ファイル構造

```
/opt/{device}/config/
├── {namespace}_settings.json    # メイン設定
├── aranea_registration.json     # 登録情報
└── wifi_config.json             # WiFi設定（複数）
```

#### 現状実装

| デバイス | ファイル | 状態 |
|---------|---------|------|
| IS20S | (araneapi依存) | 外部パッケージ |
| IS21 | `aranea_common/config.py` | ✅ 実装済み |
| IS22 | `config_store/service.rs` | ✅ 実装済み |

---

### 2. LacisIdGenerator（LacisID生成）

**ESP32参照**: `LacisIDGenerator.cpp`

#### フォーマット

```
[Prefix=1桁][ProductType=3桁][MAC=12桁hex][ProductCode=4桁]
= 20文字

例: 3022E051D815448B0001
    │ │   │           │
    │ │   │           └── ProductCode: 0001
    │ │   └────────────── MAC: E0:51:D8:15:44:8B
    │ └────────────────── ProductType: 022 (IS22)
    └──────────────────── Prefix: 3 (araneaDevice)
```

#### インターフェース（Python）

```python
class LacisIdGenerator:
    """LacisID生成器"""

    PREFIX: str = "3"

    @staticmethod
    def generate(
        product_type: str,
        mac_address: str,
        product_code: str = "0001"
    ) -> str:
        """
        LacisIDを生成

        Args:
            product_type: 3桁の製品タイプ（例: "022"）
            mac_address: MACアドレス（コロン区切りまたは連続）
            product_code: 4桁の製品コード

        Returns:
            20桁のLacisID
        """
        mac_hex = mac_address.replace(":", "").replace("-", "").upper()
        if len(mac_hex) != 12:
            raise ValueError(f"Invalid MAC address: {mac_address}")
        return f"{LacisIdGenerator.PREFIX}{product_type}{mac_hex}{product_code}"

    @staticmethod
    def parse(lacis_id: str) -> dict:
        """LacisIDをパース"""
        if len(lacis_id) != 20:
            raise ValueError(f"Invalid LacisID length: {len(lacis_id)}")
        return {
            "prefix": lacis_id[0],
            "product_type": lacis_id[1:4],
            "mac": lacis_id[4:16],
            "product_code": lacis_id[16:20]
        }
```

#### 現状実装

| デバイス | ファイル | 状態 |
|---------|---------|------|
| IS20S | (araneapi依存) | 外部パッケージ |
| IS21 | `aranea_common/lacis_id.py` | ✅ 実装済み |
| IS22 | `aranea_register/lacis_id.rs` | ✅ 実装済み |

---

### 3. AraneaRegister（デバイス登録）

**ESP32参照**: `AraneaRegister.cpp`

#### 登録フロー

```
1. TenantPrimaryAuth設定
   └── テナント管理者のlacisId/cic/userId

2. register_device()呼び出し
   ├── LacisID生成
   ├── araneaDeviceGate POST
   └── CIC/エンドポイント取得

3. 登録情報永続化
   └── aranea_registration.json
```

#### インターフェース（Python）

```python
@dataclass
class TenantPrimaryAuth:
    """テナント管理者認証"""
    lacis_id: str
    user_id: str
    cic: str

@dataclass
class RegisterResult:
    """登録結果"""
    ok: bool = False
    cic_code: str = ""
    state_endpoint: str = ""
    mqtt_endpoint: str = ""
    error: str = ""

class AraneaRegister:
    """araneaDeviceGate登録管理"""

    DEFAULT_GATE_URL = "https://us-central1-metatron-one.cloudfunctions.net/araneaDeviceGate"

    def begin(self, gate_url: str = None) -> None:
        """初期化"""

    def set_tenant_primary(self, auth: TenantPrimaryAuth) -> None:
        """テナント管理者認証を設定"""

    def register_device(
        self,
        tid: str,
        device_type: str,
        lacis_id: str,
        mac_address: str,
        product_type: str,
        product_code: str,
        timeout: float = 15.0
    ) -> RegisterResult:
        """デバイス登録"""

    def is_registered(self) -> bool:
        """登録済みか確認"""

    def get_saved_cic(self) -> str:
        """保存されたCICを取得"""

    def clear_registration(self) -> None:
        """登録情報をクリア"""
```

#### 現状実装

| デバイス | ファイル | 状態 |
|---------|---------|------|
| IS20S | (araneapi依存) | 外部パッケージ |
| IS21 | `aranea_common/aranea_register.py` | ✅ 実装済み |
| IS22 | `aranea_register/service.rs` | ✅ 実装済み |

---

### 4. HardwareInfo（ハードウェア情報）

**ESP32参照**: なし（Linux固有）

#### 取得項目

| カテゴリ | 項目 | 取得元 |
|---------|------|-------|
| メモリ | total/used/free/available | `/proc/meminfo` |
| CPU | model/cores/load/usage | `/proc/cpuinfo`, `/proc/stat` |
| 温度 | cpu/gpu/npu/soc | `/sys/class/thermal/` |
| ストレージ | total/used/free | `os.statvfs()` |
| ネットワーク | mac/ip/state | `/sys/class/net/` |
| 稼働時間 | uptime | `/proc/uptime` |
| NPU（IS21固有） | available/model/inference | `/dev/rknpu`, `dmesg` |

#### インターフェース（Python）

```python
@dataclass
class MemoryInfo:
    total_mb: int = 0
    used_mb: int = 0
    free_mb: int = 0
    available_mb: int = 0
    percent_used: float = 0.0

@dataclass
class CpuInfo:
    model: str = ""
    cores: int = 0
    architecture: str = ""
    load_1min: float = 0.0
    load_5min: float = 0.0
    load_15min: float = 0.0
    usage_percent: float = 0.0

@dataclass
class ThermalInfo:
    cpu_temp: float = 0.0
    npu_temp: float = 0.0  # IS21固有
    gpu_temp: float = 0.0
    soc_temp: float = 0.0
    zones: Dict[str, float] = field(default_factory=dict)

@dataclass
class NpuInfo:  # IS21固有
    available: bool = False
    driver_version: str = ""
    cores: int = 3  # RK3588
    model_loaded: bool = False
    current_model: str = ""
    inference_count: int = 0

class HardwareInfo:
    """ハードウェア情報取得"""

    def get_memory(self) -> MemoryInfo: ...
    def get_cpu(self) -> CpuInfo: ...
    def get_thermal(self) -> ThermalInfo: ...
    def get_storage(self, path: str = "/") -> StorageInfo: ...
    def get_network_interfaces(self) -> List[NetworkInterface]: ...
    def get_uptime(self) -> Dict[str, Any]: ...
    def get_npu(self) -> NpuInfo: ...  # IS21固有
    def get_all(self) -> Dict[str, Any]: ...
    def get_summary(self) -> Dict[str, Any]: ...
```

#### 現状実装

| デバイス | ファイル | 特徴 |
|---------|---------|------|
| IS20S | `hardware_info.py` | Zero3向け、基本項目 |
| IS21 | `aranea_common/hardware_info.py` | 5 Plus向け、NPU対応 |
| IS22 | なし | Rust実装が必要 |

---

### 5. MqttManager（MQTTクライアント）

**ESP32参照**: `MqttManager.cpp`

#### 機能要件

| 機能 | 説明 |
|------|------|
| 接続管理 | TLS接続、自動再接続 |
| Pub/Sub | トピック購読、メッセージ発行 |
| コマンド受信 | `{base}/{tid}/device/{lacisId}/command/#` |
| 状態報告 | `{base}/{tid}/device/{lacisId}/status` |
| レスポンス | `{base}/{tid}/device/{lacisId}/response` |

#### トピック構造

```
aranea/{tid}/device/{lacisId}/
├── command/        # [Subscribe] コマンド受信
│   ├── ping        # 生存確認
│   ├── set_config  # 設定変更
│   └── {custom}    # デバイス固有コマンド
├── status          # [Publish] 状態送信
└── response        # [Publish] レスポンス送信
```

#### インターフェース（Python）

```python
class MqttManager:
    """MQTTクライアント管理"""

    def __init__(self):
        self.connected: bool = False
        self._handlers: Dict[str, Callable] = {}

    async def begin(
        self,
        url: str,
        username: str,
        password: str,
        client_id: str = None
    ) -> bool:
        """接続開始"""

    async def stop(self) -> None:
        """接続停止"""

    async def publish(
        self,
        topic: str,
        payload: Union[str, dict],
        qos: int = 1,
        retain: bool = False
    ) -> bool:
        """メッセージ発行"""

    async def subscribe(
        self,
        topic: str,
        handler: Callable[[str, str], Awaitable[None]],
        qos: int = 1
    ) -> bool:
        """トピック購読"""

    async def unsubscribe(self, topic: str) -> bool:
        """購読解除"""

    def on_message(self, handler: Callable) -> None:
        """メッセージハンドラ登録"""
```

#### 現状実装

| デバイス | ファイル | 状態 |
|---------|---------|------|
| IS20S | `mqtt_handlers.py` + araneapi | ✅ 部分実装 |
| IS21 | なし | ⏳ 実装必要 |
| IS22 | なし | ⏳ 実装必要（Rust） |

---

### 6. StateReporter（状態報告）

**ESP32参照**: `StateReporter.cpp`

#### 報告データ構造

```json
{
  "lacisId": "3022E051D815448B0001",
  "cic": "605123",
  "timestamp": "2026-01-10T12:34:56.789+09:00",
  "state": {
    "status": "running",
    "uptime_sec": 86400,
    "hardware": {
      "memory_percent": 45.2,
      "cpu_load": 0.8,
      "cpu_temp": 52.3
    },
    "device_specific": { ... }  // 機種固有データ
  }
}
```

#### 報告先

| 報告先 | エンドポイント | 頻度 |
|-------|---------------|------|
| State API | `{stateEndpoint}` | 設定可能（デフォルト5分） |
| MQTT | `{base}/{tid}/device/{lacisId}/status` | イベント駆動 |

---

## UI共通部分（AraneaWebUI）

### 共通CSS（ESP32 AraneaWebUI.cpp準拠）

IS20S `ui.py`で使用されている`COMMON_CSS`がESP32準拠。

#### 共通コンポーネント

| コンポーネント | 説明 |
|---------------|------|
| `.header` | ヘッダー（ロゴ、タイトル、バージョン） |
| `.device-banner` | デバイス情報バナー |
| `.tabs` / `.tab-content` | タブナビゲーション |
| `.card` / `.card-title` | カードコンポーネント |
| `.status-grid` / `.status-item` | ステータス表示グリッド |
| `.form-group` / `.form-row` | フォーム要素 |
| `.btn` / `.btn-primary` | ボタン |
| `.toast` | 通知トースト |
| `.live-box` | ライブログ表示 |

### 共通タブ

| タブ | 内容 | 共通/固有 |
|-----|------|---------|
| Status | デバイス情報、ハードウェア、ネットワーク | **共通** |
| Network | WiFi設定、NTP設定 | **共通** |
| Cloud | MQTT状態、登録情報 | **共通** |
| Tenant | テナント設定 | **共通** |
| System | 再起動、ファクトリーリセット | **共通** |
| Capture | パケットキャプチャ | IS20S固有 |
| Monitor | トラフィック監視 | IS20S固有 |
| Cameras | カメラ管理 | IS22固有 |
| Inference | 推論設定 | IS21固有 |

---

## デバイス固有部分

### IS20S固有

| モジュール | 機能 |
|-----------|------|
| `capture.py` | tsharkパケットキャプチャ |
| `ingest.py` | イベントIngestion |
| `domain_services.py` | ドメイン分類 |
| `asn_lookup.py` | ASN/Geo検索 |
| `threat_intel.py` | 脅威インテリジェンス |

### IS21固有

| モジュール | 機能 |
|-----------|------|
| `par_inference.py` | NPU推論実行 |
| `preset_cache.py` | 推論プリセットキャッシュ |
| NPU温度監視 | RKNPU温度取得 |
| モデル管理 | RKNN/ONNX管理 |

### IS22固有

| モジュール | 機能 |
|-----------|------|
| `ipcam_scan/` | IPカメラスキャン |
| `rtsp_manager/` | RTSP接続管理 |
| `snapshot_service/` | スナップショット取得 |
| `detection_log_service/` | 検出ログ記録 |
| `suggest_engine/` | 検出傾向サジェスト |

---

## 共有ライブラリ化計画

### Phase A: Pythonパッケージ作成

```
aranea_common/  # 共有パッケージ
├── __init__.py
├── config.py           # ConfigManager
├── lacis_id.py         # LacisIdGenerator
├── aranea_register.py  # AraneaRegister
├── hardware_info.py    # HardwareInfo（基本）
├── mqtt_manager.py     # MqttManager
├── state_reporter.py   # StateReporter
└── web_ui/
    ├── __init__.py
    ├── common_css.py   # 共通CSS
    └── common_routes.py # 共通APIルート
```

### Phase B: 各デバイスへの適用

1. **IS21**: 既存`aranea_common`を拡張
2. **IS20S**: araneapi依存から移行
3. **IS22**: Rust crateとして別途実装

---

## 関連ドキュメント

- [DD01_AraneaRegister.md](./DD01_AraneaRegister.md) - Rust実装詳細
- [SharedLibrary_AraneaRegister.md](../SharedLibrary_AraneaRegister.md) - Rust共有ライブラリ設計
- ESP32 global/src - リファレンス実装

---

## 更新履歴

| 日付 | バージョン | 更新内容 | 更新者 |
|------|-----------|---------|-------|
| 2026-01-10 | 1.0.0 | 初版作成 | Claude |
