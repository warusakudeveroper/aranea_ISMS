# DD07: IS21 Baseline設計

作成日: 2026-01-10
最終更新: 2026-01-10
バージョン: 1.0.0

---

## 概要

IS21（Paraclate Inference Server）のベースライン設計。
設定管理、MQTT通信、API基盤、NPU推論統合の基本アーキテクチャを定義する。

---

## デバイス仕様

### ハードウェア

| 項目 | 仕様 |
|------|------|
| ボード | Orange Pi 5 Plus |
| SoC | RK3588 |
| CPU | 8コア（4x A76 + 4x A55） |
| RAM | 16GB LPDDR5 |
| NPU | RKNPU 6TOPS（3コア） |
| Storage | eMMC 64GB |
| Network | Gigabit Ethernet |

### ソフトウェア

| 項目 | 仕様 |
|------|------|
| OS | Armbian 25.11.2 noble |
| Python | 3.11+ |
| RKNN Runtime | 1.5.x |
| 推論フレームワーク | RKNN-Toolkit2 |

---

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                        IS21 Server                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐       │
│   │   Web API   │   │    MQTT     │   │   State     │       │
│   │  (FastAPI)  │   │   Manager   │   │  Reporter   │       │
│   └──────┬──────┘   └──────┬──────┘   └──────┬──────┘       │
│          │                 │                  │              │
│   ┌──────▼─────────────────▼──────────────────▼──────┐      │
│   │               InferenceService                     │      │
│   │  ┌────────────┐  ┌────────────┐  ┌────────────┐  │      │
│   │  │   Model    │  │   Queue    │  │   Result   │  │      │
│   │  │  Manager   │  │  Manager   │  │   Cache    │  │      │
│   │  └─────┬──────┘  └──────┬─────┘  └──────┬─────┘  │      │
│   └────────┼────────────────┼───────────────┼────────┘      │
│            │                │               │                │
│   ┌────────▼────────────────▼───────────────▼────────┐      │
│   │                   RKNPU Runtime                   │      │
│   └───────────────────────────────────────────────────┘      │
│                                                              │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐       │
│   │   Config    │   │  Hardware   │   │   Aranea    │       │
│   │   Manager   │   │    Info     │   │  Register   │       │
│   └─────────────┘   └─────────────┘   └─────────────┘       │
│                    (aranea_common)                           │
└─────────────────────────────────────────────────────────────┘
```

---

## ディレクトリ構造

```
/opt/is21/
├── src/
│   ├── main.py                 # エントリーポイント
│   ├── aranea_common/          # 共通モジュール
│   │   ├── __init__.py
│   │   ├── config.py           # ConfigManager
│   │   ├── lacis_id.py         # LacisIdGenerator
│   │   ├── aranea_register.py  # AraneaRegister
│   │   └── hardware_info.py    # HardwareInfo（NPU対応）
│   │
│   ├── inference/              # 推論モジュール
│   │   ├── __init__.py
│   │   ├── service.py          # InferenceService
│   │   ├── model_manager.py    # モデル管理
│   │   ├── queue_manager.py    # 推論キュー
│   │   └── result_cache.py     # 結果キャッシュ
│   │
│   ├── api/                    # Web API
│   │   ├── __init__.py
│   │   ├── routes.py           # APIルート
│   │   └── inference_routes.py # 推論API
│   │
│   ├── mqtt/                   # MQTT通信
│   │   ├── __init__.py
│   │   ├── manager.py          # MqttManager
│   │   └── handlers.py         # コマンドハンドラ
│   │
│   └── state/                  # 状態管理
│       ├── __init__.py
│       └── reporter.py         # StateReporter
│
├── config/                     # 設定ファイル
│   ├── is21_settings.json
│   └── aranea_registration.json
│
├── models/                     # RKNNモデル
│   ├── yolov8n.rknn
│   └── ...
│
└── logs/                       # ログファイル
```

---

## 設定スキーマ

### is21_settings.json

```json
{
  "device": {
    "name": "IS21-Inference-001",
    "location": "Server Room A"
  },
  "network": {
    "wifi": {
      "enabled": false,
      "ssid": "",
      "password": ""
    }
  },
  "inference": {
    "max_concurrent": 2,
    "timeout_sec": 30,
    "batch_size": 1,
    "priority_mode": "fifo",
    "model": {
      "default": "yolov8n.rknn",
      "detection_threshold": 0.5,
      "nms_threshold": 0.4,
      "max_detections": 20
    }
  },
  "mqtt": {
    "enabled": true,
    "broker_url": "mqtts://mqtt.example.com:8883",
    "base_topic": "aranea",
    "keepalive_sec": 60,
    "reconnect_interval_sec": 30
  },
  "state_report": {
    "enabled": true,
    "interval_sec": 300
  },
  "paraclate": {
    "enabled": true,
    "endpoint": "",
    "report_interval_minutes": 15
  }
}
```

---

## API設計

### 基本エンドポイント（共通）

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/status` | GET | サーバー状態 |
| `/api/hardware` | GET | ハードウェア情報 |
| `/api/config` | GET/PUT | 設定取得/更新 |
| `/api/register/status` | GET | 登録状態 |
| `/api/register/device` | POST | デバイス登録 |
| `/api/register` | DELETE | 登録クリア |

### 推論エンドポイント（IS21固有）

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/inference/detect` | POST | 物体検出 |
| `/api/inference/analyze` | POST | シーン分析 |
| `/api/inference/batch` | POST | バッチ推論 |
| `/api/inference/status` | GET | 推論キュー状態 |
| `/api/models` | GET | 利用可能モデル |
| `/api/models/{name}/load` | POST | モデルロード |
| `/api/models/{name}/unload` | POST | モデルアンロード |

### 推論リクエスト/レスポンス

**リクエスト**:
```json
{
  "image": "<base64 encoded image>",
  "model": "yolov8n.rknn",
  "options": {
    "threshold": 0.5,
    "max_detections": 20,
    "return_image": false
  },
  "callback_url": "http://is22:8080/api/inference/result"  // オプション
}
```

**レスポンス**:
```json
{
  "ok": true,
  "request_id": "req_abc123",
  "inference_time_ms": 45.2,
  "detections": [
    {
      "class": "person",
      "confidence": 0.92,
      "bbox": [100, 150, 200, 400]
    }
  ],
  "model": "yolov8n.rknn",
  "timestamp": "2026-01-10T12:34:56.789+09:00"
}
```

---

## MQTT設計

### トピック構造

```
aranea/{tid}/device/{lacisId}/
├── command/#           # [Subscribe] コマンド受信
│   ├── ping            # 生存確認
│   ├── set_config      # 設定変更
│   ├── inference       # 推論リクエスト
│   └── model           # モデル操作
├── status              # [Publish] 状態送信
└── response            # [Publish] レスポンス送信
```

### コマンドハンドラ

```python
async def handle_command(topic: str, payload: dict) -> None:
    cmd_type = payload.get("type") or payload.get("cmd")

    if cmd_type == "ping":
        # 生存確認
        return {"type": "ping", **get_status()}

    if cmd_type == "inference":
        # 推論リクエスト
        result = await inference_service.detect(payload.get("image"), payload.get("options"))
        return {"type": "inference_result", **result}

    if cmd_type == "set_config":
        # 設定変更
        config = payload.get("config", {})
        return await apply_config(config)

    if cmd_type == "model_load":
        # モデルロード
        return await model_manager.load(payload.get("model"))

    return {"status": "unsupported", "cmd": cmd_type}
```

---

## 状態報告

### StateReporter

```python
class StateReporter:
    """デバイス状態報告"""

    def __init__(
        self,
        lacis_id: str,
        cic: str,
        state_endpoint: str,
        interval_sec: int = 300
    ):
        self.lacis_id = lacis_id
        self.cic = cic
        self.state_endpoint = state_endpoint
        self.interval_sec = interval_sec

    async def report(self) -> None:
        """状態を報告"""
        state = await self._build_state()
        async with httpx.AsyncClient() as client:
            await client.post(
                self.state_endpoint,
                json=state,
                headers={"X-Lacis-Id": self.lacis_id, "X-CIC": self.cic}
            )

    async def _build_state(self) -> dict:
        """状態データを構築"""
        hw = hardware_info.get_summary()
        return {
            "lacisId": self.lacis_id,
            "cic": self.cic,
            "timestamp": datetime.now().isoformat(),
            "state": {
                "status": "running",
                "uptime_sec": hardware_info.get_uptime()["uptime_seconds"],
                "hardware": hw,
                "inference": {
                    "status": inference_service.status,
                    "model_loaded": inference_service.model_loaded,
                    "queue_size": inference_service.queue_size,
                    "total_inferences": inference_service.total_count,
                    "avg_latency_ms": inference_service.avg_latency_ms
                }
            }
        }

    async def start(self) -> None:
        """定期報告を開始"""
        while True:
            try:
                await self.report()
            except Exception as e:
                logger.error(f"State report failed: {e}")
            await asyncio.sleep(self.interval_sec)
```

---

## NPU推論サービス

### InferenceService

```python
class InferenceService:
    """NPU推論サービス"""

    def __init__(self, model_dir: str = "/opt/is21/models"):
        self.model_dir = Path(model_dir)
        self.model_manager = ModelManager(model_dir)
        self.queue = asyncio.Queue(maxsize=100)
        self.stats = InferenceStats()

    async def detect(
        self,
        image: Union[bytes, str],  # bytes or base64
        options: dict = None
    ) -> dict:
        """物体検出"""
        options = options or {}
        threshold = options.get("threshold", 0.5)
        max_detections = options.get("max_detections", 20)

        # 画像前処理
        img = self._preprocess(image)

        # RKNN推論
        start = time.perf_counter()
        outputs = await self.model_manager.infer(img)
        inference_ms = (time.perf_counter() - start) * 1000

        # 後処理（NMS等）
        detections = self._postprocess(outputs, threshold, max_detections)

        # 統計更新
        self.stats.record(inference_ms)

        return {
            "ok": True,
            "inference_time_ms": round(inference_ms, 2),
            "detections": detections
        }

    @property
    def status(self) -> str:
        return "ready" if self.model_manager.model_loaded else "no_model"

    @property
    def model_loaded(self) -> bool:
        return self.model_manager.model_loaded

    @property
    def queue_size(self) -> int:
        return self.queue.qsize()
```

### ModelManager

```python
class ModelManager:
    """RKNNモデル管理"""

    def __init__(self, model_dir: str):
        self.model_dir = Path(model_dir)
        self.rknn = None
        self.current_model = ""
        self.model_loaded = False

    async def load(self, model_name: str) -> bool:
        """モデルをロード"""
        model_path = self.model_dir / model_name
        if not model_path.exists():
            raise FileNotFoundError(f"Model not found: {model_name}")

        # 既存モデルをアンロード
        if self.rknn:
            self.rknn.release()

        # RKNNモデルをロード
        from rknnlite.api import RKNNLite
        self.rknn = RKNNLite()

        ret = self.rknn.load_rknn(str(model_path))
        if ret != 0:
            raise RuntimeError(f"Failed to load RKNN model: {ret}")

        ret = self.rknn.init_runtime(core_mask=RKNNLite.NPU_CORE_0_1_2)
        if ret != 0:
            raise RuntimeError(f"Failed to init runtime: {ret}")

        self.current_model = model_name
        self.model_loaded = True
        logger.info(f"Model loaded: {model_name}")
        return True

    async def infer(self, input_data) -> list:
        """推論実行"""
        if not self.model_loaded:
            raise RuntimeError("No model loaded")
        return self.rknn.inference(inputs=[input_data])

    def unload(self) -> None:
        """モデルをアンロード"""
        if self.rknn:
            self.rknn.release()
            self.rknn = None
        self.model_loaded = False
        self.current_model = ""
```

---

## IS22連携

### スナップショット推論リクエスト

IS22からIS21への推論リクエストフロー:

```
IS22                                    IS21
 │                                       │
 │  POST /api/inference/detect           │
 │  {image: base64, camera_id, options}  │
 │──────────────────────────────────────▶│
 │                                       │
 │                                   [NPU推論]
 │                                       │
 │  200 OK                               │
 │  {ok: true, detections: [...]}        │
 │◀──────────────────────────────────────│
 │                                       │
[検出ログ記録]                           │
[イベント処理]                           │
```

### 連携設定

IS22側（/opt/is22/config/）:
```json
{
  "inference": {
    "endpoint": "http://192.168.3.240:8080/api/inference/detect",
    "timeout_sec": 10,
    "retry_count": 2
  }
}
```

---

## 起動シーケンス

```python
async def main():
    # 1. 共通モジュール初期化
    config = ConfigManager("/opt/is21/config")
    config.begin("is21")

    hardware = HardwareInfo()

    # 2. デバイス登録確認
    register = AraneaRegister("/opt/is21/config")
    register.begin()
    if not register.is_registered():
        logger.warning("Device not registered - run /api/register/device")

    # 3. 推論サービス初期化
    inference_service = InferenceService()
    default_model = config.get_string("inference.model.default", "yolov8n.rknn")
    if default_model:
        await inference_service.model_manager.load(default_model)
        hardware.set_model_loaded(True, default_model)

    # 4. MQTT初期化
    if config.get_bool("mqtt.enabled", True) and register.is_registered():
        mqtt = MqttManager()
        await mqtt.begin(
            url=config.get_string("mqtt.broker_url"),
            username=register.get_saved_lacis_id(),
            password=register.get_saved_cic()
        )

    # 5. 状態報告開始
    if config.get_bool("state_report.enabled", True) and register.is_registered():
        reporter = StateReporter(
            lacis_id=register.get_saved_lacis_id(),
            cic=register.get_saved_cic(),
            state_endpoint=register.get_saved_state_endpoint(),
            interval_sec=config.get_int("state_report.interval_sec", 300)
        )
        asyncio.create_task(reporter.start())

    # 6. Web API起動
    app = create_app(config, inference_service, hardware, register)
    config_obj = uvicorn.Config(app, host="0.0.0.0", port=8080)
    server = uvicorn.Server(config_obj)
    await server.serve()
```

---

## デプロイメント

### systemdサービス

`/etc/systemd/system/is21.service`:
```ini
[Unit]
Description=IS21 Paraclate Inference Server
After=network.target

[Service]
Type=simple
User=mijeosadmin
WorkingDirectory=/opt/is21
ExecStart=/opt/is21/venv/bin/python -m src.main
Restart=always
RestartSec=5
Environment=PYTHONPATH=/opt/is21

[Install]
WantedBy=multi-user.target
```

### 環境設定

```bash
# 仮想環境作成
python3 -m venv /opt/is21/venv
source /opt/is21/venv/bin/activate

# 依存関係インストール
pip install fastapi uvicorn httpx aiomqtt rknnlite

# サービス有効化
sudo systemctl enable is21
sudo systemctl start is21
```

---

## 関連ドキュメント

- [DD06_LinuxCommonModules.md](./DD06_LinuxCommonModules.md) - 共通モジュール仕様
- [aranea_ar-is21.schema.json](../aranea_ar-is21.schema.json) - TypeDefinitionスキーマ
- [SCHEMA_DEFINITIONS.md](../SCHEMA_DEFINITIONS.md) - スキーマ定義集

---

## 更新履歴

| 日付 | バージョン | 更新内容 | 更新者 |
|------|-----------|---------|-------|
| 2026-01-10 | 1.0.0 | 初版作成 | Claude |
