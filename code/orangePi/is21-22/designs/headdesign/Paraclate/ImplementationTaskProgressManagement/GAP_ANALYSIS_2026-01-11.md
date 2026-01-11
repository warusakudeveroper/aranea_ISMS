# Paraclate実装 抜け漏れ分析レポート

作成日: 2026-01-11
作成者: Claude
目的: Paraclate_DesignOverview.mdの要件と実装の完全性チェック

---

## 1. 要件抽出（Paraclate_DesignOverview.mdより）

### 1.1 UI表記要件

| ID | 要件 | 実装状態 | 確認方法 |
|----|------|---------|---------|
| R01 | ダッシュボードページの表記はParaclate | ✅ 実装済み | Chrome確認: ヘッダー「Paraclate」 |
| R02 | サブタイトルはmobes AI control Tower | ✅ 実装済み | Chrome確認: ヘッダー下「mobes AI control Tower」 |
| R03 | ブラウザタブタイトル | ✅ 実装済み | Chrome確認: 「Paraclate - is22 CamServer」 |
| R04 | ファビコン変更 | ✅ 実装済み | Chrome確認: カメラアイコン |

### 1.2 is22デバイス登録要件

| ID | 要件 | 実装状態 | 詳細 |
|----|------|---------|------|
| R05 | aranearegister機能実装 | ✅ 実装済み | src/aranea_register/ |
| R06 | TypeDomain=araneaDevice | ✅ 実装済み | types.rs |
| R07 | Type=ar-is22CamServer | ⚠️ 部分的 | コード上は`aranea_ar-is22` |
| R08 | Prefix=3, ProductType=022 | ✅ 実装済み | types.rs |
| R09 | tid権限境界管理 | ✅ 実装済み | 登録時にtid保存 |
| R10 | **fid=施設管理** | ❌ **未実装** | RegistrationStatusにfidなし |

### 1.3 lacisOath認証要件

| ID | 要件 | 実装状態 | 詳細 |
|----|------|---------|------|
| R11 | lacisID送信 | ✅ 実装済み | paraclate_client/types.rs |
| R12 | tid送信 | ✅ 実装済み | paraclate_client/types.rs |
| R13 | cic送信 | ✅ 実装済み | paraclate_client/types.rs |
| R14 | blessing（越境トークン） | ✅ 実装済み | 構造あり、将来拡張 |
| R15 | Authorization: LacisOath形式 | ✅ 実装済み | to_headers()実装 |

### 1.4 Summary/GrandSummary要件

| ID | 要件 | 実装状態 | 詳細 |
|----|------|---------|------|
| R16 | Summary間隔ベース生成 | ✅ 実装済み | summary_service/scheduler.rs |
| R17 | GrandSummary時間指定生成 | ✅ 実装済み | summary_service/grand_summary.rs |
| R18 | Summary送信形式（summaryOverview） | ⚠️ 要確認 | payload_builder.rs確認必要 |
| R19 | Summary送信形式（cameraContext） | ⚠️ 要確認 | payload_builder.rs確認必要 |
| R20 | Summary送信形式（cameraDetection） | ⚠️ 要確認 | payload_builder.rs確認必要 |
| R21 | DB整備 | ✅ 実装済み | ai_summary_cache テーブル |

### 1.5 カメラ管理要件

| ID | 要件 | 実装状態 | 詳細 |
|----|------|---------|------|
| R22 | is801 paraclateCameraとしてlacisID付与 | ✅ 実装済み | camera_registry/lacis_id.rs |
| R23 | lacisID形式: 3801{MAC}{ProductCode} | ✅ 実装済み | 3801プレフィックス |
| R24 | カメラもTID,FIDに所属 | ⚠️ 部分的 | fidはあるがtidがnull |
| R25 | CelestialGlobeとの連携 | ❌ **未実装** | 設計のみ |

### 1.6 設定同期要件

| ID | 要件 | 実装状態 | 詳細 |
|----|------|---------|------|
| R26 | 設定の双方向同期 | ✅ 実装済み | config_sync.rs |
| R27 | Pub/Subでのステータス同期 | ✅ 実装済み | pubsub_subscriber.rs |

### 1.7 AIアシスタント機能要件

| ID | 要件 | 実装状態 | 詳細 |
|----|------|---------|------|
| R28 | Paraclate APPからのチャットボット機能 | ❌ **未実装** | UIプレースホルダーのみ |
| R29 | 検出特徴の人物のカメラ間移動把握 | ❌ **未実装** | 将来機能 |
| R30 | カメラ不調の傾向把握 | ❌ **未実装** | 将来機能 |
| R31 | 過去の記録参照・対話機能 | ❌ **未実装** | 将来機能 |

### 1.8 is21連携要件

| ID | 要件 | 実装状態 | 詳細 |
|----|------|---------|------|
| R32 | is21の正式名称ParaclateEdge | ✅ 実装済み | UI表示確認済み |
| R33 | is21はT9999...システムテナント | ✅ 設計済み | DD07参照 |
| R34 | is21はPermission71 | ✅ 設計済み | DD07参照 |
| R35 | is21管理ページ実装 | ⚠️ 部分的 | 基本UIあり、詳細未実装 |
| R36 | is22からis21へのアクティベート | ❌ **未実装** | 設計のみ |

### 1.9 データ永続化要件

| ID | 要件 | 実装状態 | 詳細 |
|----|------|---------|------|
| R37 | スナップショットのLacisFiles保存 | ❌ **未実装** | 構造のみ |
| R38 | 検知データのBQ保存 | ✅ 実装済み | bq_sync_service |

---

## 2. フロントエンド-バックエンド連動問題

### 2.1 発見された問題

| 問題ID | 問題内容 | 影響 | 深刻度 |
|--------|---------|------|--------|
| P01 | **登録タブにFID設定UIがない** | Paraclate APIにfid送信不可 | 🔴 Critical |
| P02 | **バックエンドRegistrationStatusにfidがない** | フロントエンドでfid取得不可 | 🔴 Critical |
| P03 | **Paraclateタブで「登録必要」表示** | 登録済みなのに未接続と誤表示 | 🟡 Medium |
| P04 | useParaclateStatusがfid='0000'で呼び出し | API失敗（FID不一致） | 🔴 Critical |

### 2.2 問題の連鎖

```
登録タブにfid設定UIなし (P01)
      ↓
バックエンドにfid保存されない (P02)
      ↓
フロントエンドがfid='0000'でAPI呼び出し (P04)
      ↓
Paraclate API「FID 0000 does not belong to TID」エラー
      ↓
paraclate.status?.connected = false
      ↓
isParaclateEnabled = false
      ↓
「デバイス登録が必要です」表示 (P03)
```

---

## 3. コードとChrome確認結果

### 3.1 SettingsModal.tsx確認

| 項目 | コード | Chrome | 整合性 |
|------|--------|--------|--------|
| Paraclateタブ名 | `Paraclate` (line 830) | ✅ 表示確認 | ✅ |
| ページヘッダー | `Paraclate` + `mobes AI control Tower` | ✅ 表示確認 | ✅ |
| fid取得 | `araneaRegistrationStatus?.fid \|\| '0000'` | fid=0000 | ❌ 常にデフォルト |
| isParaclateEnabled | `registered && connected` | false | ❌ 連動エラー |

### 3.2 登録タブ確認

| 項目 | 表示 | 問題 |
|------|------|------|
| デバイス登録状態 | 登録済み | - |
| LacisID | 3022E051D815448B0001 | - |
| CIC | 605123 | - |
| テナントID | T2025120621041161827 | - |
| **FID設定** | **なし** | ❌ 設定UIなし |

### 3.3 Paraclateタブ確認

| 項目 | 表示 | 問題 |
|------|------|------|
| Paraclate API接続状態 | 未接続 | 登録済みなのに未接続 |
| メッセージ | デバイス登録が必要です | 誤解を招く表示 |
| 定時報告設定 | 60分、09:00/17:00/21:00 | UI表示のみ、連動未確認 |

---

## 4. カメラ設定モーダル確認（未実施）

TODO: カメラ設定モーダルでのlacisID、fid、tid表示確認が必要

---

## 5. 未実装・要修正項目一覧

### 5.1 Critical（接続テストに必須）

| 優先度 | 項目 | 対象ファイル |
|--------|------|-------------|
| P0 | FID設定UI追加（登録タブ） | SettingsModal.tsx |
| P0 | RegistrationStatusにfid追加 | aranea_register/types.rs |
| P0 | 登録時にfid保存 | aranea_register/service.rs |
| P0 | /api/register/status でfid返却 | web_api/register_routes.rs |

### 5.2 High（機能完成に必要）

| 優先度 | 項目 | 対象ファイル |
|--------|------|-------------|
| P1 | Summary送信形式の仕様確認 | payload_builder.rs |
| P1 | カメラのtid設定 | camera_registry/service.rs |
| P1 | Paraclate接続ボタン連動確認 | SettingsModal.tsx |

### 5.3 Medium（完全性に必要）

| 優先度 | 項目 | 対象ファイル |
|--------|------|-------------|
| P2 | is22→is21アクティベート | 新規実装 |
| P2 | CelestialGlobe連携 | 新規実装 |
| P2 | LacisFiles保存 | 新規実装 |

### 5.4 Low（将来機能）

| 優先度 | 項目 | 対象ファイル |
|--------|------|-------------|
| P3 | AIチャットボット機能 | 将来実装 |
| P3 | カメラ間人物追跡 | 将来実装 |
| P3 | カメラ不調検知 | 将来実装 |

---

## 6. MECE確認

### 要件カバレッジ

- 全要件数: 38
- 実装済み: 22 (58%)
- 部分実装: 6 (16%)
- 未実装: 10 (26%)

### 漏れなし確認

| カテゴリ | 要件数 | 確認済み |
|---------|--------|---------|
| UI表記 | 4 | ✅ |
| デバイス登録 | 6 | ✅ |
| lacisOath認証 | 5 | ✅ |
| Summary | 6 | ✅ |
| カメラ管理 | 4 | ✅ |
| 設定同期 | 2 | ✅ |
| AIアシスタント | 4 | ✅ |
| is21連携 | 5 | ✅ |
| データ永続化 | 2 | ✅ |
| **合計** | **38** | ✅ |

---

## 7. 次のアクション

1. **即座に修正必要（P0）**:
   - バックエンドRegistrationStatusにfid追加
   - 登録タブにfid設定UI追加
   - フロントエンド-バックエンド連動修正

2. **接続テスト前に確認必要（P1）**:
   - Summary送信形式が仕様と一致しているか
   - Paraclate接続ボタンの動作確認

3. **ドキュメント更新**:
   - MASTER_INDEX.mdのタスク完了ステータス修正
   - 漏れタスクの追加

---

## 8. 結論

**タスクリストでは100%完了とマークされているが、実際には以下の重大な漏れがある:**

1. **FID管理が未実装** - Paraclate連携の根本要件
2. **フロントエンド-バックエンド連動不整合** - 登録済みでもParaclateタブで未登録扱い
3. **将来機能とされている項目** - AIチャットボット等はスコープ外として明示されていない

**接続テストを行うには、最低限P0項目の修正が必須。**
