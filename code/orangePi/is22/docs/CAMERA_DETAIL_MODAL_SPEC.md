# CameraDetailModal 設計書

**作成日**: 2026-01-01
**バージョン**: 1.0

---

## 1. 概要

カメラグリッドのカメラタイルクリック時に表示するカメラ詳細・編集モーダル。
カメラ設定の閲覧・編集・削除・再スキャン機能を提供する。

---

## 2. 要件定義

### 2.1 表示・編集フィールド

| フィールド | DB列名 | 型 | 編集可 | 説明 |
|-----------|--------|-----|-------|------|
| 表示名 | name | VARCHAR(128) | ✅ | カメラの表示名 |
| 部屋ID | rid | VARCHAR(32) | ✅ | 部屋名・設置場所識別子 |
| lacisID | lacis_id | VARCHAR(32) | ❌ | 自動取得（将来） |
| CIC | cic | CHAR(6) | ❌ | 6桁登録コード（将来自動取得） |
| カメラコンテキスト | camera_context | JSON | ✅ | LLM判定用の説明テキスト |
| 回転 | rotation | INT | ✅ | 0/90/180/270度 |
| ファシリティID | fid | VARCHAR(32) | ❌ | サブネットから継承 |
| テナントID | tid | VARCHAR(32) | ❌ | サブネットから継承 |
| 並び順 | sort_order | INT | ✅ | ドラッグ並べ替え用 |
| クレデンシャル | credential_* | VARCHAR | ✅ | username/password（非ブラインド） |

### 2.2 表示専用フィールド（基本情報）

| フィールド | DB列名 | 説明 |
|-----------|--------|------|
| IPアドレス | ip_address | 現在のIP |
| MACアドレス | mac_address | 固定識別子 |
| メーカー | manufacturer | 検出時取得 |
| モデル | model | 検出時取得 |
| ファミリー | family | tapo/vigi/nest等 |
| RTSP Main | rtsp_main | メインストリームURL |
| RTSP Sub | rtsp_sub | サブストリームURL |
| 登録日時 | created_at | カメラ登録日時 |

### 2.3 ONVIFデバイス情報

| フィールド | DB列名 | 説明 |
|-----------|--------|------|
| シリアル番号 | serial_number | デバイスシリアル |
| ハードウェアID | hardware_id | ハードウェア識別子 |
| ファームウェア | firmware_version | ファームウェアバージョン |
| ONVIFエンドポイント | onvif_endpoint | ONVIF WS URL |

### 2.4 ネットワーク情報

| フィールド | DB列名 | 説明 |
|-----------|--------|------|
| RTSPポート | rtsp_port | デフォルト554 |
| HTTPポート | http_port | デフォルト80 |
| ONVIFポート | onvif_port | デフォルト80 |

### 2.5 ビデオ能力

| フィールド | DB列名 | 説明 |
|-----------|--------|------|
| メイン解像度 | resolution_main | 例: 1920x1080 |
| メインコーデック | codec_main | H.264/H.265/MJPEG |
| メインFPS | fps_main | フレームレート |
| メインビットレート | bitrate_main | kbps |
| サブ解像度 | resolution_sub | 例: 640x480 |
| サブコーデック | codec_sub | H.264/H.265/MJPEG |
| サブFPS | fps_sub | フレームレート |
| サブビットレート | bitrate_sub | kbps |

### 2.6 PTZ能力

| フィールド | DB列名 | 説明 |
|-----------|--------|------|
| PTZ対応 | ptz_supported | boolean |
| 連続移動 | ptz_continuous | 連続パン/チルト対応 |
| 絶対位置 | ptz_absolute | 絶対座標指定対応 |
| 相対移動 | ptz_relative | 相対移動対応 |
| パン範囲 | ptz_pan_range | JSON {"min":-180,"max":180} |
| チルト範囲 | ptz_tilt_range | JSON {"min":-90,"max":90} |
| ズーム範囲 | ptz_zoom_range | JSON {"min":1,"max":10} |
| プリセット | ptz_presets | JSON配列 |
| ホームポジション | ptz_home_supported | boolean |

### 2.7 音声能力

| フィールド | DB列名 | 説明 |
|-----------|--------|------|
| 音声入力 | audio_input_supported | マイク対応 |
| 音声出力 | audio_output_supported | スピーカー対応 |
| 音声コーデック | audio_codec | G.711/AAC/PCM |

### 2.8 ONVIFプロファイル

| フィールド | DB列名 | 説明 |
|-----------|--------|------|
| プロファイル一覧 | onvif_profiles | JSON配列 |

### 2.9 検出メタ情報

| フィールド | DB列名 | 説明 |
|-----------|--------|------|
| 検出方法 | discovery_method | onvif/rtsp/manual |
| 最終疎通確認 | last_verified_at | 疎通確認日時 |
| 最終再スキャン | last_rescan_at | 再スキャン日時 |

### 2.3 アクション

| アクション | エンドポイント | 説明 |
|-----------|---------------|------|
| 保存 | PUT /api/cameras/:id | 編集内容保存 |
| 認証テスト | POST /api/cameras/:id/auth-test | クレデンシャル検証 |
| 再スキャン | POST /api/cameras/:id/rescan | DHCP逃げ対応IP再検出 |
| ソフト削除 | POST /api/cameras/:id/soft-delete | MAC保持で論理削除 |
| ハード削除 | DELETE /api/cameras/:id | 完全削除 |
| 復元 | POST /api/cameras/restore-by-mac | MACアドレスで復元 |

---

## 3. DBスキーマ拡張

### 3.1 マイグレーション: 007_camera_extended_fields.sql

```sql
-- カメラ拡張フィールド
ALTER TABLE cameras
ADD COLUMN rid VARCHAR(32) NULL AFTER location,
ADD COLUMN cic CHAR(6) NULL AFTER lacis_id,
ADD COLUMN rotation INT NOT NULL DEFAULT 0 AFTER camera_context,
ADD COLUMN fid VARCHAR(32) NULL AFTER rotation,
ADD COLUMN tid VARCHAR(32) NULL AFTER fid,
ADD COLUMN sort_order INT NOT NULL DEFAULT 0 AFTER tid,
ADD COLUMN credential_username VARCHAR(64) NULL,
ADD COLUMN credential_password VARCHAR(128) NULL,
ADD COLUMN deleted_at DATETIME(3) NULL;

-- インデックス
CREATE INDEX idx_cameras_rid ON cameras(rid);
CREATE INDEX idx_cameras_fid ON cameras(fid);
CREATE INDEX idx_cameras_tid ON cameras(tid);
CREATE INDEX idx_cameras_deleted ON cameras(deleted_at);
CREATE INDEX idx_cameras_sort ON cameras(sort_order);
```

---

## 4. API設計

### 4.1 PUT /api/cameras/:id (更新)

**Request:**
```json
{
  "name": "ロビーカメラ1",
  "rid": "LOBBY-01",
  "camera_context": {"description": "玄関入口を監視", "tags": ["入口", "セキュリティ"]},
  "rotation": 90,
  "sort_order": 5,
  "credential_username": "admin",
  "credential_password": "password123"
}
```

**Response:**
```json
{
  "ok": true,
  "data": { /* Camera object */ }
}
```

### 4.2 POST /api/cameras/:id/auth-test (認証テスト)

クレデンシャルでRTSP/ONVIF接続を試行し結果を返す。

**Request:**
```json
{
  "username": "admin",
  "password": "password123"
}
```

**Response:**
```json
{
  "ok": true,
  "data": {
    "rtsp_success": true,
    "onvif_success": true,
    "model": "Tapo C100",
    "message": "認証成功"
  }
}
```

### 4.3 POST /api/cameras/:id/rescan (単体再スキャン)

MACアドレスを基にネットワーク再スキャンし、新しいIPを検出。

**Response:**
```json
{
  "ok": true,
  "data": {
    "found": true,
    "old_ip": "192.168.125.100",
    "new_ip": "192.168.125.145",
    "updated": true
  }
}
```

### 4.4 POST /api/cameras/:id/soft-delete (ソフト削除)

`deleted_at` に現在時刻を設定。MACアドレスは保持。

**Response:**
```json
{
  "ok": true,
  "data": {
    "camera_id": "xxx",
    "mac_address": "AA:BB:CC:DD:EE:FF",
    "deleted_at": "2026-01-01T12:00:00Z",
    "recoverable": true
  }
}
```

### 4.5 DELETE /api/cameras/:id (ハード削除)

完全削除。復元不可。

### 4.6 POST /api/cameras/restore-by-mac (MAC復元)

ソフト削除されたカメラをMACアドレスで復元。

**Request:**
```json
{
  "mac_address": "AA:BB:CC:DD:EE:FF"
}
```

---

## 5. UI設計

### 5.1 モーダルレイアウト

```
+----------------------------------------------------------+
|  カメラ設定                                         [×]   |
+----------------------------------------------------------+
|                                                          |
|  [スナップショットプレビュー 16:9]                        |
|  +----------------------------------------------------+  |
|  |                                                    |  |
|  |              (回転プレビュー反映)                   |  |
|  |                                                    |  |
|  +----------------------------------------------------+  |
|  回転: [0°] [90°] [180°] [270°]                          |
|                                                          |
|  ── 基本情報 ───────────────────────────────             |
|  表示名:    [ロビーカメラ1              ]                 |
|  部屋ID:    [LOBBY-01                   ]                 |
|  IP:        192.168.125.45 (読取専用)                     |
|  MAC:       AA:BB:CC:DD:EE:FF (読取専用)                  |
|                                                          |
|  ── 識別情報 ───────────────────────────────             |
|  lacisID:   3001AABBCCDDEEFF0001 (自動)                   |
|  CIC:       123456 (自動)                                 |
|  FID:       0099 (継承)                                   |
|  TID:       T2025... (継承)                               |
|                                                          |
|  ── クレデンシャル ─────────────────────────             |
|  ユーザー名: [admin                     ]                 |
|  パスワード: [password123               ] 👁               |
|  [認証テスト]  ✓ 認証成功                                 |
|                                                          |
|  ── カメラコンテキスト ────────────────────              |
|  [                                      ]                 |
|  [  玄関入口を監視するカメラ。          ]                 |
|  [  不審者検知を重視。                  ]                 |
|  [                                      ]                 |
|                                                          |
|  ── デバイス情報 ───────────────────────────             |
|  メーカー:  TP-Link                                       |
|  モデル:    Tapo C100                                     |
|  ファミリー: tapo                                         |
|  RTSP Main: rtsp://192.168.125.45:554/stream1             |
|  登録日:    2026-01-01 10:30:00                           |
|                                                          |
+----------------------------------------------------------+
|  [再スキャン]  [削除▼]              [キャンセル] [保存]   |
+----------------------------------------------------------+
```

### 5.2 削除ドロップダウン

```
+------------------+
| 削除▼            |
+------------------+
| ソフト削除       |  ← MACベース復元可能
| ────────────     |
| 完全削除         |  ← 警告後に実行
+------------------+
```

### 5.3 削除確認ダイアログ

**ソフト削除:**
```
+------------------------------------------+
|  カメラを削除しますか？                   |
+------------------------------------------+
|                                          |
|  「ロビーカメラ1」をソフト削除します。   |
|                                          |
|  MACアドレス (AA:BB:CC:DD:EE:FF) を      |
|  保持するため、後から復元できます。       |
|                                          |
|  [キャンセル]         [削除する]          |
+------------------------------------------+
```

**ハード削除:**
```
+------------------------------------------+
|  ⚠️ 完全削除の確認                        |
+------------------------------------------+
|                                          |
|  「ロビーカメラ1」を完全に削除します。   |
|                                          |
|  この操作は取り消せません。              |
|  関連するイベント・フレームデータも      |
|  すべて削除されます。                    |
|                                          |
|  削除するには「完全削除」と入力:         |
|  [                    ]                  |
|                                          |
|  [キャンセル]         [完全削除]          |
+------------------------------------------+
```

---

## 6. フロントエンドコンポーネント

### 6.1 ファイル構成

```
src/components/
├── camera/
│   ├── CameraDetailModal.tsx    # メインモーダル
│   ├── CameraInfoSection.tsx    # 基本情報セクション
│   ├── CredentialSection.tsx    # クレデンシャルセクション
│   ├── ContextSection.tsx       # カメラコンテキスト編集
│   ├── RotationSelector.tsx     # 回転選択
│   └── DeleteDropdown.tsx       # 削除ドロップダウン
└── ...
```

### 6.2 状態管理

```typescript
interface CameraDetailModalState {
  isOpen: boolean;
  camera: Camera | null;
  editedFields: Partial<UpdateCameraRequest>;
  isLoading: boolean;
  isSaving: boolean;
  authTestResult: AuthTestResult | null;
  rescanResult: RescanResult | null;
  deleteConfirmType: 'soft' | 'hard' | null;
}
```

---

## 7. 実装チェックリスト

### 7.1 DB
- [ ] マイグレーション 007 作成
- [ ] マイグレーション実行

### 7.2 バックエンド
- [ ] Camera 構造体に新フィールド追加
- [ ] UpdateCameraRequest に新フィールド追加
- [ ] update_camera で新フィールド対応
- [ ] POST /api/cameras/:id/auth-test 実装
- [ ] POST /api/cameras/:id/rescan 実装
- [ ] POST /api/cameras/:id/soft-delete 実装
- [ ] POST /api/cameras/restore-by-mac 実装
- [ ] list_cameras で deleted_at IS NULL フィルタ

### 7.3 フロントエンド
- [ ] CameraDetailModal.tsx 実装
- [ ] types/api.ts に Camera 型拡張
- [ ] App.tsx に onCameraClick ハンドラ追加
- [ ] 削除確認ダイアログ実装
- [ ] 認証テストボタン実装
- [ ] 再スキャンボタン実装

---

## 8. MECE確認

- ✅ カメラ情報の表示: 全フィールド網羅
- ✅ カメラ情報の編集: 編集可能フィールド明確化
- ✅ カメラ削除: ソフト/ハード両対応
- ✅ 復元機能: MACベース復元
- ✅ 再スキャン: DHCP対応
- ✅ 認証: クレデンシャル保存・テスト
