# IS21 プラットフォーム状態報告書

バージョン: 1.8.0
作成日: 2025-12-30
デバイス: Orange Pi 5 Plus (RK3588)

---

## 1. 実装状態サマリ

| カテゴリ | 状態 | 備考 |
|---------|------|------|
| **AI推論 (YOLO/PAR)** | ✅ 完了 | v1.8.0 camera_context対応済み |
| **基本API** | ✅ 完了 | /api/status, /v1/analyze 等 |
| **デバイス登録** | ⚠️ 一部問題 | 認証フロー要検証 |
| **設定管理** | ⚠️ 未完成 | AI設定UIが最新仕様に未対応 |
| **MQTT連携** | ❌ 未実装 | 将来対応予定 |
| **OTA更新** | ❌ 未実装 | 将来対応予定 |

---

## 2. API実装状態

### 2.1 基本エンドポイント

| エンドポイント | 状態 | 説明 |
|---------------|------|------|
| `GET /healthz` | ✅ | ヘルスチェック |
| `GET /api/status` | ✅ | デバイス状態取得 |
| `GET /api/hardware` | ✅ | ハードウェア情報 |
| `GET /api/hardware/summary` | ✅ | ハードウェア概要 |
| `GET /v1/capabilities` | ✅ | 機能一覧 |

### 2.2 推論エンドポイント

| エンドポイント | 状態 | 説明 |
|---------------|------|------|
| `POST /v1/analyze` | ✅ | 画像解析（YOLO+PAR） |
| `GET /v1/schema` | ✅ | スキーマ取得 |
| `PUT /v1/schema` | ✅ | スキーマ更新 |

### 2.3 デバイス管理エンドポイント

| エンドポイント | 状態 | 説明 |
|---------------|------|------|
| `POST /api/register` | ⚠️ | デバイス登録（認証問題あり） |
| `DELETE /api/register` | ✅ | 登録クリア |
| `GET /api/config` | ✅ | 設定取得 |
| `POST /api/config` | ✅ | 設定更新 |

---

## 3. 既知の問題

### 3.1 認証関連 (🔴 重要)

**問題:**
- `AraneaRegister` による認証フローが正常に動作しない可能性
- `TenantPrimaryAuth` の検証が不十分

**現状:**
```python
# main.py:1069-1085
def init_aranea_device():
    config_manager.begin("is21")
    lacis_id = lacis_generator.generate()

    tenant_auth = TenantPrimaryAuth(
        lacis_id=TENANT_LACIS_ID,
        user_id=TENANT_EMAIL,
        cic=TENANT_CIC
    )
    aranea_register.set_tenant_primary(tenant_auth)

    if aranea_register.is_registered():
        logger.info(f"Device registered: CIC={aranea_register.get_saved_cic()}")
    else:
        logger.info("Device not registered yet")
```

**必要な対応:**
1. araneaDeviceGateとの認証フロー検証
2. 認証エラー時のハンドリング強化
3. トークン更新メカニズムの実装

### 3.2 AI設定UI (🟡 中優先)

**問題:**
- AI関連の設定画面が最新仕様で構築されていない
- WebUI経由での設定変更インターフェースが未実装

**現在のAI設定項目:**

```python
# 現在はハードコードされている
CONF_THRESHOLD = 0.33
NMS_THRESHOLD = 0.40
PAR_ENABLED = True
PAR_MAX_PERSONS = 10
PAR_THRESHOLD = 0.5
```

**必要な設定UI項目:**

| 項目 | 型 | 範囲 | 説明 |
|-----|-----|------|------|
| `conf_threshold` | float | 0.2-0.8 | YOLO信頼度閾値 |
| `nms_threshold` | float | 0.2-0.8 | NMS閾値 |
| `par_enabled` | bool | - | PAR有効/無効 |
| `par_max_persons` | int | 1-20 | PAR最大処理人数 |
| `par_threshold` | float | 0.3-0.8 | PAR属性閾値 |

### 3.3 WebUI未実装

**問題:**
- is21はAPIサーバーのみで、設定用WebUIが存在しない
- IS22との連携設定をAPIでのみ行う必要がある

**対応案:**
1. 最小限のWebUI実装（設定画面のみ）
2. IS22側で集中管理するアプローチ

---

## 4. aranea_commonモジュール状態

### 4.1 使用モジュール

| モジュール | 状態 | 用途 |
|-----------|------|------|
| `ConfigManager` | ✅ | 設定ファイル管理 |
| `LacisIdGenerator` | ✅ | lacisID生成 |
| `AraneaRegister` | ⚠️ | デバイス登録（要検証） |
| `TenantPrimaryAuth` | ⚠️ | テナント認証（要検証） |
| `HardwareInfo` | ✅ | ハードウェア情報取得 |

### 4.2 aranea_commonパス

```
/opt/is21/src/aranea_common/
├── __init__.py
├── config_manager.py
├── lacis_id_generator.py
├── aranea_register.py
├── tenant_primary_auth.py
└── hardware_info.py
```

---

## 5. 設定ファイル

### 5.1 ファイル構成

```
/opt/is21/
├── config/
│   ├── schema.json        # 推論スキーマ
│   └── device.json        # デバイス設定（aranea_common経由）
├── models/
│   ├── yolov5s-640-640.rknn
│   └── par_resnet50_pa100k.rknn
└── src/
    ├── main.py
    ├── par_inference.py
    └── aranea_common/
```

### 5.2 デフォルト設定値

```python
# テナント設定
DEFAULT_TID = "T2025120608261484221"
TENANT_LACIS_ID = "12767487939173857894"
TENANT_EMAIL = "info+ichiyama@neki.tech"
TENANT_CIC = "263238"

# デバイス識別
PRODUCT_TYPE = "021"
PRODUCT_CODE = "0001"
DEVICE_TYPE = "ar-is21"
DEVICE_NAME = "camimageEdge AI"
```

---

## 6. サービス管理

### 6.1 systemd設定

```bash
# サービス確認
sudo systemctl status is21-infer.service

# ログ確認
sudo journalctl -u is21-infer.service -f

# 再起動
sudo systemctl restart is21-infer.service
```

### 6.2 推奨サービスファイル

```ini
# /etc/systemd/system/is21-infer.service
[Unit]
Description=IS21 Inference Server
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/is21/src
ExecStart=/usr/bin/python3 main.py
Restart=always
RestartSec=5
Environment=PYTHONPATH=/opt/is21/src

[Install]
WantedBy=multi-user.target
```

---

## 7. 今後の対応事項

### 7.1 短期 (P0-P1)

| # | 項目 | 優先度 | 工数見積 |
|---|------|--------|---------|
| 1 | 認証フロー検証・修正 | P0 | 1-2日 |
| 2 | AI設定API追加 (`PUT /api/config/ai`) | P1 | 0.5日 |
| 3 | エラーハンドリング強化 | P1 | 0.5日 |

### 7.2 中期 (P2)

| # | 項目 | 優先度 |
|---|------|--------|
| 1 | 設定用WebUI実装 | P2 |
| 2 | MQTT連携 | P2 |
| 3 | OTA更新機能 | P2 |

---

## 8. テスト状態

### 8.1 完了テスト

- [x] YOLO推論 (画像解析)
- [x] PAR推論 (人物属性認識)
- [x] camera_context フィルタリング
- [x] suspicious_score 計算
- [x] ヘルスチェックAPI

### 8.2 未完了テスト

- [ ] AraneaRegister デバイス登録
- [ ] TenantPrimaryAuth 認証フロー
- [ ] 長時間稼働安定性
- [ ] メモリリーク確認

---

## 9. 補足情報

### 9.1 関連ドキュメント

- `IS21_AI_IMPLEMENTATION_REPORT.md` - AI機能実装報告
- `IS22_CAMERA_CONTEXT_GUIDE.md` - camera_context送信ガイド

### 9.2 デプロイ環境

| 項目 | 値 |
|-----|-----|
| IP | 192.168.3.116 |
| Port | 9000 |
| OS | Ubuntu 22.04 (Armbian) |
| Python | 3.10+ |
| FastAPI | 0.100+ |

---

作成: Claude Code
ステータス: 開発中 (認証・設定UI要対応)
