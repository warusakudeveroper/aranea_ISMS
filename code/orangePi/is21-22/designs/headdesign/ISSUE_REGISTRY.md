# IS22 Issue Registry（実装計画）

文書番号: ISSUE_REGISTRY
作成日: 2025-12-31
ステータス: Phase 5（実装計画・Issue登録）

---

## 0. 文書の目的

DESIGN_GAP_ANALYSIS.mdで特定された12項目のギャップ（GAP-001〜012）をIssueとして管理し、実装進捗を追跡する。

---

## 1. Issue一覧

### P0（必須・ブロッカー）

| Issue ID | GAP ID | タイトル | ステータス | 担当 | 備考 |
|----------|--------|---------|-----------|------|------|
| IS22-001 | GAP-001 | アーキテクチャ再設計（11コンポーネント化） | IN_PROGRESS | - | バックエンドRust実装中 |
| IS22-002 | GAP-002 | AdmissionController新規設計 | DONE | - | 実装済み（#28統合） |
| IS22-011 | GAP-011 | ConfigStore SSoT実装 | DONE | - | cameras.rsで実装済み |

### P1（重要・コア機能）

| Issue ID | GAP ID | タイトル | ステータス | 担当 | 備考 |
|----------|--------|---------|-----------|------|------|
| IS22-003 | GAP-003 | UI 3ペイン設計（React+shadcn） | IN_PROGRESS | - | フロントエンド実装中 |
| IS22-004 | GAP-004 | IpcamScan新規実装 | IN_PROGRESS | - | バックエンド・フロント両方 |
| IS22-005 | GAP-005 | Lease/Heartbeat機構 | DONE | - | AdmissionController統合 |
| IS22-006 | GAP-006 | サジェストエンジン設計 | PENDING | - | 未着手 |
| IS22-008 | GAP-008 | AIClient Adapter設計 | IN_PROGRESS | - | ai_client/mod.rs |
| IS22-009 | GAP-009 | camerasテーブル拡張 | DONE | - | migration済み |
| IS22-010 | GAP-010 | 新規テーブル追加 | DONE | - | migration済み |
| IS22-012 | GAP-012 | EventLogServiceリングバッファ | IN_PROGRESS | - | 部分実装 |

### P2（推奨・拡張機能）

| Issue ID | GAP ID | タイトル | ステータス | 担当 | 備考 |
|----------|--------|---------|-----------|------|------|
| IS22-007 | GAP-007 | StreamGatewayAdapter化 | PENDING | - | go2rtc連携 |

---

## 2. UI実装（IS22-003）サブタスク

### 2.1 実装済みサブタスク

| サブID | 内容 | ファイル | ステータス | 設計整合性 |
|--------|------|---------|-----------|-----------|
| IS22-003-01 | 3ペインレイアウト（25%/50%/25%） | App.tsx | DONE | 要確認 |
| IS22-003-02 | EventLogPaneカラーリング | EventLogPane.tsx | DONE | 要確認 |
| IS22-003-03 | CameraGrid 3列デフォルト | CameraGrid.tsx | DONE | 要確認 |
| IS22-003-04 | BlankCard（カメラ0件時） | App.tsx | DONE | 要確認 |
| IS22-003-05 | ScanModal（レーダーUI） | ScanModal.tsx | DONE | 要確認 |
| IS22-003-06 | ConfirmDialog | ConfirmDialog.tsx | DONE | 要確認 |
| IS22-003-07 | dialog基盤コンポーネント | ui/dialog.tsx | DONE | 要確認 |

### 2.2 未実装サブタスク

| サブID | 内容 | 優先度 | 設計参照 |
|--------|------|--------|---------|
| IS22-003-08 | サジェストペイン動画再生 | P1 | is22_11_Suggest設計 |
| IS22-003-09 | モーダル動画再生 | P1 | is22_10_AdmissionControl設計 |
| IS22-003-10 | 混雑時拒否メッセージUI | P1 | is22_10_AdmissionControl設計 |
| IS22-003-11 | カメラ詳細設定モーダル | P2 | is22_04_web設計 |
| IS22-003-12 | ダークモード対応 | P2 | COMMON_01_UIガイドライン |

---

## 3. IpcamScan実装（IS22-004）サブタスク

### 3.1 バックエンド（Rust）

| サブID | 内容 | ステータス | ファイル |
|--------|------|-----------|---------|
| IS22-004-BE-01 | IpcamScanモジュール構造 | DONE | ipcam_scan/mod.rs |
| IS22-004-BE-02 | scanner実装 | DONE | ipcam_scan/scanner.rs |
| IS22-004-BE-03 | types定義 | DONE | ipcam_scan/types.rs |
| IS22-004-BE-04 | API routes | DONE | web_api/routes.rs |
| IS22-004-BE-05 | DetectionReason構造体 | DONE | ipcam_scan/types.rs |

### 3.2 フロントエンド（React）

| サブID | 内容 | ステータス | ファイル |
|--------|------|-----------|---------|
| IS22-004-FE-01 | ScanModal UI | DONE | ScanModal.tsx |
| IS22-004-FE-02 | RadarSpinner | DONE | ScanModal.tsx |
| IS22-004-FE-03 | PhaseIndicator | DONE | ScanModal.tsx |
| IS22-004-FE-04 | DeviceCard | DONE | ScanModal.tsx |
| IS22-004-FE-05 | 一括登録機能 | DONE | ScanModal.tsx |
| IS22-004-FE-06 | DetectionReason型 | DONE | types/api.ts |

---

## 4. 設計ドキュメントとの整合性チェック項目

### 4.1 必須参照との整合性

| 項目 | 仕様書記載 | 実装状態 | 整合性 |
|-----|-----------|---------|--------|
| ページタイトル | mobes AIcam control Tower (mAcT) | App.tsx L108 | 要確認 |
| 左ペイン | サジェストライブ動画 | SuggestPane.tsx | 要確認 |
| 中央ペイン | カメラサムネ羅列 | CameraGrid.tsx | 要確認 |
| 右ペイン | AI解析ログ | EventLogPane.tsx | 要確認 |
| ペイン比率 | 左「やや大きい」 | 25%/50%/25% | 要検証 |

### 4.2 GAP-003設計との整合性

| 項目 | DESIGN_GAP_ANALYSIS記載 | 実装状態 | 整合性 |
|-----|------------------------|---------|--------|
| フレームワーク | React + shadcn/ui | 使用中 | OK |
| 3ペイン構造 | サジェスト/サムネ/ログ | 実装済み | OK |
| サジェスト再生 | サーバー決定・全員共通 | 未実装 | PENDING |
| モーダル再生 | ユーザー個別・TTL制限 | 未実装 | PENDING |
| 混雑時拒否 | 明示的なUXフロー | 未実装 | PENDING |

---

## 5. テスト計画

### 5.1 UI単体テスト

| テストID | 対象 | テスト内容 | ステータス |
|----------|------|-----------|-----------|
| UI-001 | App.tsx | 3ペインレイアウト表示 | PENDING |
| UI-002 | EventLogPane | ログカラーリング（severity 0-3） | PENDING |
| UI-003 | CameraGrid | 3列表示 | PENDING |
| UI-004 | BlankCard | カメラ0件時表示 | PENDING |
| UI-005 | ScanModal | スキャン開始・進捗表示 | PENDING |
| UI-006 | DeviceCard | デバイス選択・解除 | PENDING |
| UI-007 | ConfirmDialog | 確認・キャンセル動作 | PENDING |

### 5.2 統合テスト

| テストID | 対象 | テスト内容 | ステータス |
|----------|------|-----------|-----------|
| INT-001 | ScanModal→API | スキャンJob作成 | PENDING |
| INT-002 | ScanModal→API | デバイス一括登録 | PENDING |
| INT-003 | EventLogPane→API | イベント取得・表示 | PENDING |
| INT-004 | CameraGrid→API | カメラ一覧取得・表示 | PENDING |

---

## 6. 更新履歴

| 日付 | バージョン | 内容 |
|-----|-----------|------|
| 2025-12-31 | 1.0 | 初版作成（Phase 5開始） |
