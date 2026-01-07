# AIEventLog タスクリスト

文書番号: Layout＆AIlog_Settings/AIEventLog_TaskList
作成日: 2026-01-08
関連Issue: #100
設計ドキュメント: AIEventLog_Redesign_v4.md
ステータス: 実装前

---

## 大原則の確認

本タスクリストはThe_golden_rules.mdに完全準拠:

- [x] Rule 15: 「タスクリスト、テストリストの無い受け入れ条件が曖昧な実装は禁止」
- [x] Rule 7: 「棚上げにして将来実装などというケースはありません」
- [x] Rule 9: 「チェック、テストのない完了報告は報告と見做されない」

---

## v5修正事項（設計解釈エラー）

### 修正対象: T1-4 cleanup_unknown_images

| 項目 | v4（誤り） | 修正後 |
|------|----------|--------|
| **目的** | unknownを常に自動削除 | 既存ゴミデータの**手動クリーンアップ** |
| **トリガー** | 自動（クォータ連動） | **手動のみ**（管理者操作） |
| **10%保持** | 分散サンプリング | **最新10%保持** |
| **Rule 5** | 違反（情報の自動削除） | 準拠（ユーザー確認権利尊重） |

**修正理由**: Rule 5「情報は如何なる場合においても等価であり、優劣をつけて自己判断で不要と判断した情報を握り潰すような仕様としてはならない」

---

## Phase 1: Critical - ストレージ管理

| ID | タスク | 依存 | ファイル | 完了 |
|----|--------|------|----------|------|
| T1-1 | StorageQuotaConfig構造体追加 | - | detection_log_service/mod.rs | [ ] |
| T1-2 | CleanupStats構造体追加 | - | detection_log_service/mod.rs | [ ] |
| T1-3 | should_save_image関数実装 | - | detection_log_service/mod.rs | [ ] |
| T1-4 | enforce_storage_quota実装（枚数+容量） | T1-1,T1-2 | detection_log_service/mod.rs | [ ] |
| T1-5 | **manual_cleanup_unknown_images実装**（手動のみ） | T1-1,T1-2 | detection_log_service/mod.rs | [ ] |
| T1-6 | ストレージ設定API追加（GET/PUT） | T1-1 | web_api/routes.rs | [ ] |
| T1-7 | **手動クリーンアップAPI追加**（プレビュー+実行） | T1-5 | web_api/routes.rs | [ ] |
| T1-8 | ストレージ設定UI追加 | T1-6 | SettingsModal.tsx | [ ] |
| T1-9 | **手動クリーンアップUI追加**（確認ダイアログ付き） | T1-7 | SettingsModal.tsx | [ ] |
| T1-10 | バックエンドビルド確認 | T1-5 | - | [ ] |

### T1-1 詳細: StorageQuotaConfig

```rust
#[derive(Debug, Clone)]
pub struct StorageQuotaConfig {
    pub max_images_per_camera: usize,      // デフォルト: 1000
    pub max_bytes_per_camera: u64,         // デフォルト: 500MB
    pub max_total_bytes: u64,              // デフォルト: 10GB
}
```

### T1-3 詳細: should_save_image真理値表

| severity | primary_event | unknown_flag | should_save | 理由 |
|----------|---------------|--------------|-------------|------|
| 0 | "none" | false | false | 何もない |
| 0 | "none" | true | false | none優先 |
| 0 | "unknown" | true | **true** | 不明だが何かある |
| 0 | "human" | false | true | イベントあり |
| 1+ | any | any | true | severity>0 |

### T1-5 詳細: manual_cleanup_unknown_images

```rust
/// 手動クリーンアップ（管理者専用）
///
/// WARNING: Rule 5準拠のため自動実行禁止
///
/// 動作:
/// 1. confirmed=false → プレビュー返却のみ
/// 2. confirmed=true → 最新10%保持、古い90%削除
pub async fn manual_cleanup_unknown_images(
    &self,
    camera_id: &str,
    confirmed: bool,
) -> Result<CleanupResult>
```

---

## Phase 2: High - 表示仕様準拠

| ID | タスク | 依存 | ファイル | 完了 |
|----|--------|------|----------|------|
| T2-1 | EVENT_COLORS定数追加（6色） | - | EventLogPane.tsx | [ ] |
| T2-2 | ICON_COLORS定数追加（6色） | - | EventLogPane.tsx | [ ] |
| T2-3 | getEventStyle関数実装 | T2-1 | EventLogPane.tsx | [ ] |
| T2-4 | getIconColor関数実装 | T2-2 | EventLogPane.tsx | [ ] |
| T2-5 | EVENT_LABELS定数追加（22ラベル） | - | EventLogPane.tsx | [ ] |
| T2-6 | getEventLabel関数修正 | T2-5 | EventLogPane.tsx | [ ] |
| T2-7 | SEVERITY_TOOLTIPS定数追加 | - | EventLogPane.tsx | [ ] |
| T2-8 | Severityツールチップ適用 | T2-7 | EventLogPane.tsx | [ ] |
| T2-9 | カメラ名表示（min-w/max-w/title） | - | EventLogPane.tsx | [ ] |
| T2-10 | DetectionLogItem統合修正 | T2-3,T2-4,T2-6,T2-8,T2-9 | EventLogPane.tsx | [ ] |
| T2-11 | フロントエンドビルド確認 | T2-10 | - | [ ] |

### T2-1 詳細: EVENT_COLORS

```typescript
const EVENT_COLORS = {
  human: { bg: '#FFFF00', text: '#222222', border: '#B8B800' },
  vehicle: { bg: '#6495ED', text: '#FFFFFF', border: '#4169E1' },
  alert: { bg: '#FF0000', text: '#FFFF00', border: '#CC0000' },
  unknown: { bg: '#D3D3D3', text: '#222222', border: '#A9A9A9' },
  camera_lost: { bg: '#333333', text: '#FFFFFF', border: '#1A1A1A' },
  camera_recovered: { bg: '#FFFFFF', text: '#444444', border: '#CCCCCC' },
} as const
```

### T2-5 詳細: EVENT_LABELS

```typescript
const EVENT_LABELS: Record<string, string> = {
  'human': '人物検知',
  'person': '人物検知',
  'humans': '複数人検知',
  'people': '複数人検知',
  'crowd': '群衆検知',
  'vehicle': '車両検知',
  'car': '車両検知',
  'truck': 'トラック検知',
  'suspicious': '不審行動',
  'alert': '警戒',
  'intrusion': '侵入検知',
  'fire': '火災検知',
  'smoke': '煙検知',
  'animal': '動物検知',
  'dog': '犬検知',
  'cat': '猫検知',
  'package': '荷物検知',
  'object': '物体検知',
  'motion': '動体検知',
  'movement': '動き検知',
  'unknown': '不明',
  'none': '検出なし',
  'camera_lost': '接続ロスト',
  'camera_recovered': '接続復帰',
}
```

---

## Phase 3: High - 画像可視性・ストリーム対応

| ID | タスク | 依存 | ファイル | 完了 |
|----|--------|------|----------|------|
| T3-1 | DiagnosticsResult構造体追加 | - | web_api/routes.rs | [ ] |
| T3-2 | 診断API実装（GET /api/diagnostics/images/{camera_id}） | T3-1 | web_api/routes.rs | [ ] |
| T3-3 | 診断UI追加 | T3-2 | SettingsModal.tsx | [ ] |
| T3-4 | RecoveryMode enum追加（3種） | - | web_api/routes.rs | [ ] |
| T3-5 | 復旧API実装（POST /api/recovery/images/{camera_id}） | T3-4 | web_api/routes.rs | [ ] |
| T3-6 | 復旧UI（ImageRecoveryDialog） | T3-5 | SettingsModal.tsx | [ ] |
| T3-7 | StreamSource enum追加 | - | snapshot_service/mod.rs | [ ] |
| T3-8 | get_snapshot_for_inference実装（フォールバック） | T3-7 | snapshot_service/mod.rs | [ ] |
| T3-9 | ストリームソースDB保存 | T3-8 | detection_log_service/mod.rs | [ ] |
| T3-10 | get_snapshot_with_validation実装 | - | snapshot_service/mod.rs | [ ] |
| T3-11 | 画像送信失敗ログ強化 | - | ai_client/mod.rs | [ ] |
| T3-12 | analyze_with_frame_diff実装（二枚画像） | - | ai_client/mod.rs | [ ] |
| T3-13 | PrevFrameCache改修（画像保持） | - | prev_frame_cache/mod.rs | [ ] |
| T3-14 | polling_orchestrator二枚画像対応 | T3-12,T3-13 | polling_orchestrator/mod.rs | [ ] |

### T3-4 詳細: RecoveryMode

```rust
pub enum RecoveryMode {
    RefetchCurrent,  // 現在のスナップショットで再保存
    FixPathOnly,     // DBパスのみ修正
    PurgeOrphans,    // 欠損レコードをDB削除
}
```

### T3-8 詳細: ストリーム選択フロー

```
メインストリーム試行 → 成功 → 推論+保存
        ↓ 失敗
サブストリーム試行 → 成功 → 推論+保存（警告ログ）
        ↓ 失敗
スキップ → エラーログ → 次サイクルでリトライ
```

---

## Phase 4: Medium - 監視・精度・バックエンド

| ID | タスク | 依存 | ファイル | 完了 |
|----|--------|------|----------|------|
| T4-1 | InferenceStats構造体追加 | - | web_api/routes.rs | [ ] |
| T4-2 | 推論統計API（GET /api/stats/inference） | T4-1 | web_api/routes.rs | [ ] |
| T4-3 | 推論統計タブUI | T4-2 | SettingsModal.tsx | [ ] |
| T4-4 | Is21Health構造体追加 | - | ai_client/mod.rs | [ ] |
| T4-5 | IS21ヘルスチェック実装 | T4-4 | ai_client/mod.rs | [ ] |
| T4-6 | MisdetectionFeedback構造体追加 | - | models/feedback.rs | [ ] |
| T4-7 | misdetection_feedbacksテーブル作成 | - | migrations/ | [ ] |
| T4-8 | 誤検知報告API（POST /api/feedback/misdetection） | T4-6,T4-7 | web_api/routes.rs | [ ] |
| T4-9 | 誤検知報告UI | T4-8 | DetectionDetailModal.tsx | [ ] |
| T4-10 | 誤検知統計API（GET /api/feedback/stats/{camera_id}） | T4-8 | web_api/routes.rs | [ ] |
| T4-11 | camera_thresholdsテーブル作成 | - | migrations/ | [ ] |
| T4-12 | threshold_change_historyテーブル作成 | - | migrations/ | [ ] |
| T4-13 | 閾値変更API（PUT /api/cameras/{camera_id}/threshold） | T4-11,T4-12 | web_api/routes.rs | [ ] |
| T4-14 | 閾値調整UI | T4-13 | SettingsModal.tsx | [ ] |
| T4-15 | 閾値の推論適用（conf_override） | T4-13 | polling_orchestrator/mod.rs | [ ] |
| T4-16 | 自動閾値調整ロジック（オプション） | T4-8,T4-13 | threshold_adjuster.rs | [ ] |

---

## Phase 5: 基盤整備

| ID | タスク | 依存 | ファイル | 完了 |
|----|--------|------|----------|------|
| T5-1 | autoAttunement TODOコメント追加 | - | polling_orchestrator/mod.rs | [ ] |
| T5-2 | autoAttunement TODOコメント追加 | - | detection_log_service/mod.rs | [ ] |
| T5-3 | autoAttunement TODOコメント追加 | - | preset_loader/mod.rs | [ ] |

### T5-1 詳細: TODOコメント

```rust
// TODO(autoAttunement): 検出結果をStatsCollectorに送信
// 参照: Layout＆AIlog_Settings/AIEventLog_Redesign_v4.md Section 5.6
```

---

## Phase 6: テスト

| ID | タスク | 依存 | テスト計画参照 | 完了 |
|----|--------|------|---------------|------|
| T6-1 | バックエンドユニットテスト | T1-10 | AIEventLog_TestPlan.md BE-01〜BE-20 | [ ] |
| T6-2 | フロントエンドビルドテスト | T2-11 | AIEventLog_TestPlan.md FE-01〜FE-02 | [ ] |
| T6-3 | Chrome UIテスト | T6-2 | AIEventLog_TestPlan.md UI-01〜UI-14 | [ ] |

---

## 依存関係図

```
Phase 1 (ストレージ)
T1-1 → T1-4 → T1-5 → T1-7 → T1-9
     ↘ T1-2 ↗       ↘ T1-6 → T1-8
T1-3 (独立)
T1-10 (ビルド確認)

Phase 2 (表示)
T2-1 → T2-3 ↘
T2-2 → T2-4 → T2-10 → T2-11
T2-5 → T2-6 ↗
T2-7 → T2-8 ↗
T2-9 ↗

Phase 3 (画像)
T3-1 → T3-2 → T3-3
T3-4 → T3-5 → T3-6
T3-7 → T3-8 → T3-9
T3-10, T3-11 (独立)
T3-12 → T3-14
T3-13 ↗

Phase 4 (監視)
T4-1 → T4-2 → T4-3
T4-4 → T4-5
T4-6 → T4-8 → T4-9, T4-10
T4-7 ↗
T4-11 → T4-13 → T4-14, T4-15
T4-12 ↗
T4-16 (T4-8, T4-13依存)

Phase 5 (基盤)
T5-1, T5-2, T5-3 (独立)

Phase 6 (テスト)
全Phase完了後
```

---

## 進捗管理

| Phase | タスク数 | 完了 | 進捗率 |
|-------|---------|------|--------|
| Phase 1 | 10 | 0 | 0% |
| Phase 2 | 11 | 0 | 0% |
| Phase 3 | 14 | 0 | 0% |
| Phase 4 | 16 | 0 | 0% |
| Phase 5 | 3 | 0 | 0% |
| Phase 6 | 3 | 0 | 0% |
| **合計** | **57** | **0** | **0%** |

---

## 実装順序

1. **Phase 1完了** → バックエンドビルド確認
2. **Phase 2完了** → フロントエンドビルド確認
3. **Phase 3完了** → 画像機能動作確認
4. **Phase 4完了** → 監視機能動作確認
5. **Phase 5完了** → TODOコメント確認
6. **Phase 6完了** → 全テスト実行・報告

---

**文書終端**
