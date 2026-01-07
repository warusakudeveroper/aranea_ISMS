# Camscan機能 実装報告書

## 基本情報

| 項目 | 内容 |
|------|------|
| プロジェクト | is22 RTSPカメラ総合管理サーバー |
| 実装期間 | 2026-01-06 〜 2026-01-07 |
| 実装者 | Claude Code |
| 検証環境 | http://192.168.125.246:3000 |
| 関連Issue | #80〜#90 (全11件クローズ済み) |
| 最終コミット | d655292 |

---

## 設計書対応表

Camscan_designers_review.md に記載された11項目の実装状況:

| Item | 設計要件 | 実装状況 | 関連ファイル |
|------|----------|----------|--------------|
| 1 | OUI判定リストDB化・主要ブランド追加 | ✅ 完了 | `scanner/oui_data.rs`, `017_oui_expansion.sql` |
| 2 | カテゴリ表示改善 | ✅ 完了 | `types/api.ts`, `ScanModal.tsx` |
| 3 | バックエンド単一実行化 | ✅ 完了 | `ipcam_scan/mod.rs`, `routes.rs` |
| 4 | Brute Force Modeトグル | ✅ 完了 | `ScanModal.tsx`, `routes.rs` |
| 5 | StrayChildScan (IP変更検出) | ✅ 完了 | `scanner/mod.rs`, カテゴリF実装 |
| 6 | 登録済みカメラ名表示 | ✅ 完了 | `registered_camera_name`属性追加 |
| 7 | %表示改善 | ✅ 完了 | 6フェーズ進捗表示 (ARP/Port/OUI/ONVIF/RTSP/照合) |
| 8 | スキャン結果文言改善 | ✅ 完了 | DEVICE_CATEGORIES定義 |
| 9 | スキャン開始ボタン化 | ✅ 完了 | 「スキャン開始 (Nサブネット)」形式 |
| 10 | カテゴリC強制登録・ボタン文言変更 | ✅ 完了 | `pending_auth`ステータス追加 |
| 11 | サブネット削除時クリーンアップ | ✅ 完了 | サブネット削除API連携 |

---

## 実装詳細

### 1. OUI判定リストDB化・主要ブランド追加

**目的**: TP-LINK以外のカメラブランドも検出対象とする

**実装内容**:
- `oui_data.rs`: ハードコードOUIを削除、DB駆動型に変更
- 対応ブランド:
  - TP-LINK (複数OUI)
  - Hikvision
  - Dahua
  - Reolink
  - Amcrest
  - EZVIZ
  - その他主要ブランド

**検証結果**:
```
192.168.125.78: Hikvision (MAC: A8:42:A1:B9:53:23)
192.168.125.80: TP-LINK (MAC: D8:07:B6:xx:xx:xx)
192.168.125.52: LLA:モバイルデバイス (ローカルアドレス検出)
```

---

### 2. カテゴリ表示改善

**目的**: ユーザーが理解しやすいカテゴリ分類と表示

**実装内容** (`types/api.ts`):

| カテゴリ | ラベル | 背景色 | 折りたたみ |
|----------|--------|--------|------------|
| A | 登録済みカメラ | 緑 | 展開 |
| B | 新規登録可能 | 青 | 展開 |
| C | 認証情報の設定が必要 | 黄 | 展開 |
| D | ネットワーク機器 | グレー | **折りたたみ** |
| E | その他のデバイス | グレー | **折りたたみ** |
| F | 要確認: 通信できません | 赤 | 展開 |

**サブカテゴリ詳細** (`DeviceCategoryDetail`):
- `RegisteredAuthenticated`: 登録済み・認証済み
- `RegisteredAuthIssue`: 登録済み・認証問題あり
- `Registrable`: 登録可能
- `AuthRequired`: 認証必要
- `PossibleCamera`: カメラ可能性あり
- `NetworkEquipment`: ネットワーク機器
- `IoTDevice`: IoTデバイス
- `UnknownDevice`: 不明デバイス
- `NonCamera`: 非カメラ
- `LostConnection`: 通信不能
- `StrayChild`: IP変更検出

---

### 3. バックエンド単一実行化

**目的**: スキャンプロセスの安定性確保、重複実行防止

**実装内容** (`ipcam_scan/mod.rs`):
```rust
// スキャン状態管理
pub struct ScanState {
    is_running: AtomicBool,
    current_scan_id: RwLock<Option<String>>,
    // ...
}

// 重複実行チェック
if self.is_running.compare_exchange(...) {
    return Err("スキャン実行中".into());
}
```

**動作**:
- モーダルを閉じてもバックエンドでスキャン継続
- 別タブから再実行時は「実行中」ステータス表示
- 「スキャンを中止して結果表示」で早期終了可能

---

### 4. Brute Force Modeトグル

**目的**: 不要な認証試行を省略し、スキャン時間短縮

**実装内容**:
```typescript
// ScanModal.tsx
<Checkbox
  checked={bruteForceMode}
  onChange={(e) => setBruteForceMode(e.target.checked)}
/>
カメラの可能性が低い検出対象にも全認証を試行します
※非常に時間がかかります
```

**動作**:
- デフォルト: OFF
- OFF時: カテゴリD/Eデバイスへの認証試行をスキップ
- ON時: 全デバイスに認証試行 (時間大幅増加)

---

### 5. StrayChildScan (IP変更検出)

**目的**: DHCPでIP変更されたカメラの追跡

**実装内容**:
```rust
// scanner/mod.rs
fn detect_stray_children(&self, devices: &[ScannedDevice]) -> Vec<StrayChild> {
    // MAC照合で登録済みカメラのIP変更を検出
    for device in devices {
        if let Some(registered) = self.find_registered_by_mac(&device.mac) {
            if registered.ip != device.ip {
                // StrayChild検出
            }
        }
    }
}
```

**表示**: カテゴリFに `ipChanged: true` フラグ付きで表示

---

### 6. 登録済みカメラ名表示

**目的**: スキャン結果で登録済みカメラを識別しやすくする

**実装内容**:
```typescript
// CategorizedDevice型
interface CategorizedDevice extends ScannedDevice {
  registeredCameraName?: string;  // 表示名
  ipChanged?: boolean;            // IP変更フラグ
}
```

**表示**: カテゴリA/Fで登録名を太字表示

---

### 7. %表示改善

**目的**: 「止まってる？」感の解消

**実装内容** (`ScanModal.tsx`):
```typescript
const PHASE_WEIGHTS = {
  arp: 15,      // Stage 1: ARPスキャン
  port: 25,     // Stage 2-3: ポートスキャン
  oui: 5,       // Stage 4: OUI照合
  onvif: 20,    // Stage 5: ONVIFプローブ
  rtsp: 30,     // Stage 6: RTSP認証試行
  match: 5,     // Stage 7: カメラマッチング
};
// 合計: 100%
```

**動作**:
- リアルタイム進捗更新
- フェーズ別バー表示
- ログ表示でアクティビティ可視化

---

### 8. スキャン結果文言改善

**目的**: 専門用語を減らし、ユーザーフレンドリーに

**実装内容** (`DEVICE_CATEGORIES`):
```typescript
{
  id: 'c',
  label: '認証情報の設定が必要',
  description: 'カメラとして検出されましたが、認証に失敗しました。正しいユーザー名・パスワードを設定してください。',
  // ...
}
```

**追加機能**: 試行クレデンシャル表示
```typescript
interface TriedCredentialResult {
  username: string;
  password: string;  // 平文表示 (設計書High #1準拠)
  result: 'success' | 'failed' | 'timeout';
}
```

---

### 9. スキャン開始ボタン化

**実装内容**:
- テキストリンク → ボタン形式に変更
- 「スキャン開始 (N サブネット)」形式で対象数明示
- 視認性向上

---

### 10. カテゴリC強制登録・ボタン文言変更

**目的**: 認証失敗カメラも登録可能にする

**実装内容**:
```typescript
// CameraStatus型に追加
type CameraStatus = 'active' | 'inactive' | 'pending_auth' | 'maintenance';
```

**ボタン文言**:
- 「カメラを全選択」→「認証済カメラを選択」
- 追加: 「認証待ちも含めて選択」

**動作**: カテゴリCカメラは `pending_auth` ステータスで登録

---

### 11. サブネット削除時クリーンアップ

**目的**: 管理対象外サブネットのデバイス残留防止

**実装内容** (`routes.rs`):
```rust
// サブネット削除時
async fn delete_subnet(subnet_id: i32) {
    // 1. サブネット削除
    // 2. 該当サブネットのスキャン結果削除
    // 3. 登録済みカメラは維持 (ユーザー操作に委ねる)
}
```

---

## テスト結果サマリー

| カテゴリ | 項目数 | PASS | N/A | N/T |
|----------|--------|------|-----|-----|
| UI確認 | 20 | 20 | 0 | 0 |
| CRUD操作 | 4 | 2 | 0 | 2 |
| 機能動作 | 16 | 15 | 1 | 0 |
| 導線確認 | 5 | 5 | 0 | 0 |
| **合計** | **45** | **42** | **1** | **2** |

**詳細**: [TEST_CHECKLIST.md](./TEST_CHECKLIST.md) 参照

---

## 技術スタック

### バックエンド (Rust)
- `is22/src/ipcam_scan/`: スキャンエンジン
- `is22/src/ipcam_scan/scanner/`: ネットワークスキャナ
- `is22/src/camera_brand/`: ブランドサービス
- `is22/src/web_api/routes.rs`: APIエンドポイント

### フロントエンド (React/TypeScript)
- `frontend/src/components/ScanModal.tsx`: スキャンモーダル
- `frontend/src/components/SuggestPane.tsx`: 結果表示ペイン
- `frontend/src/types/api.ts`: 型定義
- `frontend/src/utils/deviceCategorization.ts`: カテゴリ分類ロジック

### データベース
- `migrations/017_oui_expansion.sql`: OUI拡張マイグレーション

---

## 既知の制限事項

1. **オフラインカメラテスト未実施** (5-3)
   - テスト環境に通信不能カメラが存在しなかったため
   - 実装は完了、動作は型定義・コードレビューで確認

2. **サブネット削除テスト未実施** (11-2, 11-4)
   - 本番環境のため実際の削除操作は控えた
   - バックエンド実装は完了・コードレビュー済み

---

## 後続タスク向け情報

### API エンドポイント

| メソッド | パス | 説明 |
|----------|------|------|
| POST | `/api/scan/start` | スキャン開始 |
| GET | `/api/scan/status` | スキャン状態取得 |
| POST | `/api/scan/stop` | スキャン中止 |
| GET | `/api/scan/results` | スキャン結果取得 |
| POST | `/api/cameras/register` | カメラ登録 |

### 型定義

```typescript
// スキャン結果デバイス
interface ScannedDevice {
  device_id: string;
  ip: string;
  mac: string;
  oui_vendor: string | null;
  category_detail?: DeviceCategoryDetail;
  credential_status: 'not_tried' | 'success' | 'failed' | 'timeout';
  tried_credentials?: TriedCredentialResult[];
  registered_camera_name?: string;
  ip_changed?: boolean;
  // ...
}

// カテゴリ分類後デバイス
interface CategorizedDevice extends ScannedDevice {
  category: 'a' | 'b' | 'c' | 'd' | 'e' | 'f';
  isRegistered: boolean;
  registeredCameraName?: string;
  ipChanged?: boolean;
}
```

### スキャンフェーズ

| Stage | フェーズ | 重み | 説明 |
|-------|----------|------|------|
| 1 | HostDiscovery | 15% | ARPスキャン |
| 2-3 | PortScan | 25% | ポートスキャン |
| 4 | OuiLookup | 5% | OUI照合 |
| 5 | OnvifProbe | 20% | ONVIFプローブ |
| 6 | RtspAuth | 30% | RTSP認証試行 |
| 7 | CameraMatching | 5% | カメラマッチング |

---

## 参照ドキュメント

- [Camscan_designers_review.md](./Camscan_designers_review.md) - 設計レビュー原本
- [TEST_CHECKLIST.md](./TEST_CHECKLIST.md) - テストチェックリスト
- [TASK_INDEX.md](./TASK_INDEX.md) - タスクインデックス
- [The_golden_rules.md](/The_golden_rules.md) - 開発ルール

---

## 承認

| 役割 | 担当 | 日付 | 署名 |
|------|------|------|------|
| 実装者 | Claude Code | 2026-01-07 | ✅ |
| レビュー | - | - | |
| 承認 | - | - | |
