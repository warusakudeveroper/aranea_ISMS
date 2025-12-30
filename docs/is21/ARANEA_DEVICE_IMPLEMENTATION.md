# is21 araneaDevice共通機能実装レポート

作成日: 2025-12-30
ステータス: 実装完了・動作確認済み

## 概要

is21（camimageEdge AI推論サーバ）にaraneaDevice共通機能を実装。ESP32 global/srcモジュールのPython移植版として、is20sの実装パターンを参考に開発。

## 実装モジュール

### aranea_common パッケージ

`/opt/is21/src/aranea_common/`

| ファイル | 行数 | ESP32対応元 | 機能 |
|---------|------|-------------|------|
| `__init__.py` | 18 | - | パッケージ初期化 |
| `config.py` | 127 | SettingManager.h/cpp | JSON永続化による設定管理 |
| `lacis_id.py` | 142 | LacisIDGenerator.h | lacisId生成（MAC12HEX方式） |
| `aranea_register.py` | 247 | AraneaRegister.h/cpp | araneaDeviceGate登録 |
| `hardware_info.py` | 414 | is20s/hardware_info.py | NPU対応ハードウェア情報 |

### main.py 拡張

既存の推論サーバAPIに以下を追加:

| エンドポイント | メソッド | 機能 |
|---------------|---------|------|
| `/api/status` | GET | デバイス状態（lacisId, CIC, 登録状態, 稼働統計） |
| `/api/hardware` | GET | 詳細ハードウェア情報（CPU, メモリ, NPU, 温度, ネットワーク） |
| `/api/hardware/summary` | GET | 軽量版ハードウェア概要 |
| `/api/register` | POST | araneaDeviceGate登録 |
| `/api/register` | DELETE | 登録情報クリア |
| `/api/config` | GET/POST | 設定取得・更新 |

## デバイス情報

```json
{
  "device": {
    "type": "ar-is21",
    "name": "camimageEdge AI",
    "productType": "021",
    "productCode": "0001",
    "firmwareVersion": "1.0.0"
  },
  "lacisId": "3021C0742BFCCF950001",
  "mac": "C0:74:2B:FC:CF:95"
}
```

## ハードウェア情報（抜粋）

```json
{
  "memory": {
    "total_mb": 7927,
    "used_mb": 416,
    "percent_used": 5.2
  },
  "cpu": {
    "cores": 8,
    "architecture": "aarch64",
    "load_1min": 0.11
  },
  "thermal": {
    "cpu_temp": 37.9,
    "npu_temp": 37.0,
    "gpu_temp": 37.0
  },
  "npu": {
    "available": true,
    "cores": 3,
    "model_loaded": true,
    "current_model": "yolov5s-640-640.rknn"
  }
}
```

## 設計原則

### ESP32互換性

- **LacisID形式**: `3` + productType(3桁) + MAC12HEX + productCode(4桁) = 20文字
- **araneaDeviceGate登録**: 同一ペイロード形式（lacisOath, userObject, deviceMeta）
- **設定永続化**: NVS相当の機能をJSON/ファイルで実現

### Orange Pi拡張

- **NPU監視**: RK3588 NPU（3コア）の状態・温度監視
- **詳細温度**: CPU/NPU/GPU/SOC個別監視（thermal_zone対応）
- **推論統計**: 推論回数・処理時間の追跡
- **ネットワーク詳細**: 複数NIC対応、RX/TX統計

## is22連携

is22（camServer）から以下のAPIを呼び出し可能:

1. **ヘルスチェック**: `GET /healthz` - 稼働確認
2. **デバイス状態**: `GET /api/status` - 登録状態・稼働統計
3. **ハードウェア監視**: `GET /api/hardware` - リソース使用状況
4. **推論能力**: `GET /v1/capabilities` - 対応イベント・推奨サイズ
5. **スキーマ管理**: `GET/PUT /v1/schema` - カメラ設定スキーマ
6. **推論実行**: `POST /v1/analyze` - 画像分析

## 依存関係

```
fastapi>=0.115.0
uvicorn<0.40
pydantic>=2.0
httpx>=0.28.0
numpy
opencv-python
rknnlite (RK3588 NPU SDK)
```

## 残課題

### 優先度: 高

1. **araneaDeviceGate URL確認**: 404エラー発生、本番URL要確認
2. **MQTT/deviceStateReport**: クラウド連携は未実装
3. **認証トークン管理**: JWT/セッション管理

### 優先度: 中

1. **OTA更新**: ファームウェア更新機構
2. **ログ収集**: クラウドへのログ送信
3. **異常検知**: 温度閾値・メモリ枯渇アラート

## ファイル配置

```
/opt/is21/
├── src/
│   ├── main.py                 # FastAPIサーバ + araneaDevice統合
│   └── aranea_common/          # 共通モジュール
│       ├── __init__.py
│       ├── config.py           # 設定管理
│       ├── lacis_id.py         # LacisID生成
│       ├── aranea_register.py  # Gate登録
│       └── hardware_info.py    # ハードウェア情報
├── config/
│   ├── schema.json             # カメラスキーマ（is22から受信）
│   ├── is21_settings.json      # デバイス設定
│   └── aranea_registration.json # 登録情報（CIC等）
├── models/
│   └── yolov5s-640-640.rknn    # 推論モデル
└── logs/
    └── is21-infer.log          # アプリログ
```

## 動作確認結果

| テスト項目 | 結果 | 備考 |
|-----------|------|------|
| サービス起動 | OK | systemd自動起動 |
| /api/status | OK | lacisId, MAC, 稼働時間取得 |
| /api/hardware | OK | CPU/NPU/メモリ/温度/ネットワーク |
| /healthz | OK | アップタイム表示 |
| /v1/capabilities | OK | デバイス情報含む |
| /api/register | NG | Gate側404（URL要確認）|

---

**実装者**: Claude Code
**レビュー**: 未実施
