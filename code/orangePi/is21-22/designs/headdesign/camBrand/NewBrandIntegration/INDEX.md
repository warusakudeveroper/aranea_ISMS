# 新規ブランド統合ガイド

## 概要

IS22システムに新規カメラブランドを追加するための包括的ガイド。

---

## ドキュメント構成

### 1. 技術解析
| ドキュメント | 内容 |
|--------------|------|
| [REGISTRATION_FLOW_ANALYSIS.md](./REGISTRATION_FLOW_ANALYSIS.md) | IS22カメラ登録フロー詳細解析 |

### 2. 実装ガイド
| ドキュメント | 内容 |
|--------------|------|
| [IMPLEMENTATION_TASKS.md](./IMPLEMENTATION_TASKS.md) | 実装タスク一覧 |
| [CREDENTIAL_HANDLING.md](./CREDENTIAL_HANDLING.md) | クレデンシャル管理詳細（作成予定） |
| [API_EXTENSION_GUIDE.md](./API_EXTENSION_GUIDE.md) | API拡張ガイド（作成予定） |

### 3. 検証プロセス
| ドキュメント | 内容 |
|--------------|------|
| [../NewBrandValidation/VALIDATION_PROCEDURE.md](../NewBrandValidation/VALIDATION_PROCEDURE.md) | 検証手順 |
| [../NewBrandValidation/CHECKLIST.md](../NewBrandValidation/CHECKLIST.md) | 統合チェックリスト |
| [../NewBrandValidation/scripts/](../NewBrandValidation/scripts/) | 検証スクリプト |

---

## クイックリファレンス

### 新規ブランド追加の最小手順

#### Step 1: OUI調査
```bash
# カメラのMACアドレスからOUIを特定
./scripts/onvif_discovery.sh --ip 192.168.x.x
```

#### Step 2: DBにブランド登録
```sql
-- 1. ブランド追加
INSERT INTO camera_brands (name, display_name, category)
VALUES ('NewBrand', 'New Brand Display Name', 'consumer');

-- 2. OUIエントリ追加
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status)
SELECT 'XX:XX:XX', id, 'New camera model', 20, 'confirmed'
FROM camera_brands WHERE name = 'NewBrand';

-- 3. RTSPテンプレート追加
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default)
SELECT id, 'NewBrand Standard', '/stream1', '/stream2', 554, TRUE
FROM camera_brands WHERE name = 'NewBrand';
```

#### Step 3: キャッシュリフレッシュ
IS22サービスを再起動するか、API経由でキャッシュをリフレッシュ:
```bash
curl -X POST http://192.168.125.246:8080/api/settings/camera-brands/refresh-cache
```

#### Step 4: スキャン実行
UI経由でスキャンを実行し、新規ブランドが検出されることを確認。

---

## 関連ディレクトリ

```
camBrand/
├── NewBrandIntegration/        # ← このディレクトリ
│   ├── INDEX.md
│   ├── REGISTRATION_FLOW_ANALYSIS.md
│   └── IMPLEMENTATION_TASKS.md
├── NewBrandValidation/         # 検証プロセス
│   ├── VALIDATION_PROCEDURE.md
│   ├── CHECKLIST.md
│   └── scripts/
├── accessAbsorber/             # 接続制限機能
│   ├── SPECIFICATION.md
│   ├── DB_SCHEMA.md
│   └── UI_FEEDBACK_SPEC.md
├── JOOAN/                      # ブランド別詳細
│   ├── IS22_BRAND_INTEGRATION_PROPOSAL.md
│   └── STRESS_TEST_REPORT.md
├── TP_link(Tapo)/
│   ├── STREAM_ARCHITECTURE.md
│   └── STRESS_TEST_REPORT.md
└── TP_link(VIGI)/
    └── STRESS_TEST_REPORT.md
```

---

## 主要コード参照

| コンポーネント | ファイル | 説明 |
|---------------|----------|------|
| CameraFamily enum | `is22/src/ipcam_scan/types.rs:29-41` | ハードコードされたファミリー定義 |
| RtspTemplate | `is22/src/ipcam_scan/types.rs:52-133` | RTSP URLテンプレート |
| CameraBrandService | `is22/src/camera_brand/service.rs` | ブランド管理サービス |
| ARPスキャナ | `is22/src/lost_cam_tracker/arp_scanner.rs` | L2スキャン |
| DBスキーマ | `is22/migrations/015_oui_rtsp_ssot.sql` | ブランドSSoTテーブル |

---

## 現在の課題と対応状況

| 課題 | 状態 | 対応方針 |
|------|------|----------|
| NVT/JOOANブランド未対応 | 🔴 未対応 | マイグレーション追加 |
| 空ユーザー名問題 | 🟡 部分対応 | UI/バックエンド拡張 |
| CameraFamily二重管理 | 🟡 部分対応 | DB優先化 |
| Access Absorber未実装 | 🔴 未対応 | 新規実装必要 |

---

## 変更履歴

| 日付 | 変更内容 |
|------|----------|
| 2026-01-17 | 初版作成 |
