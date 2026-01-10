# IS21 (ParaclateEdge) 実装状態調査

作成日: 2026-01-10

## 1. 概要

IS21は画像解析推論サーバーであり、Paraclate設計においてはParaclateEdgeとして位置付けられる。

- **IP**: 192.168.3.240
- **言語**: Python (FastAPI)
- **ハードウェア**: Orange Pi 5 Plus (RK3588)
- **バージョン**: v1.8.0

---

## 2. ディレクトリ構成

```
/opt/is21/
├── src/
│   ├── main.py                    # FastAPIサーバー
│   ├── par_inference.py           # PAR推論モジュール
│   ├── preset_cache.py            # プリセットキャッシュ
│   └── aranea_common/             # araneaDevice共通機能
│       ├── __init__.py
│       ├── aranea_register.py     # デバイス登録
│       ├── lacis_id.py            # lacisID生成
│       ├── config.py              # 設定管理
│       └── hardware_info.py       # ハードウェア情報
├── config/
│   ├── is21_settings.json
│   └── aranea_registration.json
├── models/
│   ├── yolov5s-640-640.rknn       # 物体検出
│   └── par_resnet50_pa100k.rknn   # 人物属性認識
└── systemd/
    └── is21-infer.service
```

---

## 3. AI推論機能

### 3.1 YOLOv5s（物体検出）

| 項目 | 値 |
|------|-----|
| 入力サイズ | 640x640 |
| クラス数 | 80 (COCO) |
| 処理時間 | 40-70ms |
| デフォルト信頼度 | 0.33 |

### 3.2 PAR（人物属性認識）

| 項目 | 値 |
|------|-----|
| モデル | ResNet50 (PA100K) |
| 入力サイズ | 192x256 RGB |
| 属性数 | 26 |
| 処理時間 | 20-30ms |
| 最大同時処理 | 10人 |

### 3.3 出力タグ例

```
appearance.*  - hat_like, mask_like, glasses, uniform_like
carry.*       - backpack, bag, holding
outfit.*      - coat_like, dress, pants, shorts, boots
gender.*      - male, female
age.*         - child_teen, adult, elderly
top_color.*   - red, blue, black, white, gray
bottom_color.* - black, blue, gray, white
```

---

## 4. APIエンドポイント

### 4.1 状態・情報取得

| エンドポイント | メソッド | 機能 |
|---------------|---------|------|
| /healthz | GET | ヘルスチェック |
| /api/status | GET | デバイス状態 |
| /api/hardware | GET | ハードウェア詳細 |
| /v1/capabilities | GET | 機能一覧 |

### 4.2 推論エンドポイント

```
POST /v1/analyze
- infer_image: UploadFile      # 推論対象画像
- camera_id: str               # カメラID
- captured_at: str             # タイムスタンプ
- schema_version: str          # スキーマバージョン
- hints_json: Optional[str]    # camera_context
- prev_image: Optional[File]   # 前フレーム
```

### 4.3 設定・登録

| エンドポイント | メソッド | 機能 |
|---------------|---------|------|
| /api/config | GET/POST | 設定取得・更新 |
| /api/register | POST | デバイス登録 |
| /api/register | DELETE | 登録クリア |
| /v1/schema | GET/PUT | スキーマ管理 |

---

## 5. 認証・登録系実装状態

### 5.1 aranea_common パッケージ

| ファイル | ESP32対応元 | 機能 | 状態 |
|---------|------------|------|------|
| config.py | SettingManager | JSON永続化設定 | ✅ |
| lacis_id.py | LacisIDGenerator | lacisID生成 | ✅ |
| aranea_register.py | AraneaRegister | Gate登録 | ⚠️ |
| hardware_info.py | - | NPU対応HW情報 | ✅ |

### 5.2 既知の問題

1. **araneaDeviceGate URL**: 404エラー発生、本番URL要確認
2. **トークン更新**: JWT/セッション管理未実装
3. **認証エラー処理**: ハンドリング不十分

### 5.3 デフォルト認証情報

```python
# main.py より
DEFAULT_TID = "T2025120608261484221"  # 市山水産
TENANT_LACIS_ID = "12767487939173857894"
TENANT_EMAIL = "info+ichiyama@neki.tech"
TENANT_CIC = "263238"
```

**注意**: Paraclate_DesignOverview.mdによりmijeo.inc tid への移行が必要

---

## 6. is22との連携インターフェース

### 6.1 リクエスト

```json
POST /v1/analyze
{
  "camera_id": "string",
  "captured_at": "ISO8601",
  "schema_version": "2025-12-29.1",
  "hints_json": {
    "location_type": "entrance",
    "distance": "medium",
    "excluded_objects": ["cat"],
    "expected_objects": ["human"],
    "conf_override": 0.4,
    "nms_threshold": 0.45
  }
}
```

### 6.2 レスポンス

```json
{
  "primary_event": "human|vehicle|animal|none|unknown",
  "tags": ["count.person_1", "appearance.mask_like"],
  "severity": 0-3,
  "suspicious": {
    "score": 0-100,
    "level": "normal|low|medium|high|critical",
    "factors": ["mask_worn", "night_time"]
  },
  "performance": {
    "inference_ms": 85,
    "yolo_ms": 55,
    "par_ms": 30
  }
}
```

---

## 7. 管理WebUI

**状態: ❌ 未実装**

is21はAPIサーバーのみで管理UIなし。全ての設定はAPI経由で行う必要がある。

Paraclate_DesignOverview.md より:
> is21に関しても管理ページ(is20s相当)を持つ必要があるがそれらは仮実装であり現時点以降を持って設定管理を実装する必要がある。

---

## 8. Paraclate設計での位置付け

### 8.1 正式名称
- **ParaclateEdge** (旧: is21 camimageEdge)

### 8.2 権限設定（設計上）
- TID: T9999999999999999999（システムテナント）
- FID: 9966（システム専用論理施設）
- Permission: 71（クロステナント権限、全施設アクセス）

### 8.3 is22との連携
- is22からis21のエンドポイントにアクセス
- is22のtid, lacisID, cicでアクティベート
- is21はレスポンスを返すのみ（is21側にis22の情報登録不要）

---

## 9. MECE確認

- **網羅性**: API、モジュール、認証機能を全調査
- **重複排除**: is22との機能重複なし（推論特化）
- **未カバー領域**: 管理WebUI、MQTT連携、OTA更新が明確に未実装として識別
