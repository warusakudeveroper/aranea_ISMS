# 設計修正項目インデックス（ギャップ分析）

文書番号: DESIGN_GAP_ANALYSIS
作成日: 2025-12-31
ステータス: Phase 3完了

---

## 0. 文書の目的

本書は、旧設計ドキュメント（is22_01〜is22_08）と必須参照ドキュメント「is22_Camserver仕様案（基本設計書草案改訂版）」との間のギャップを詳細に分析し、設計修正が必要な項目を体系的に整理したものである。

### 比較対象

| 区分 | 文書 | 概要 |
|-----|------|------|
| 旧設計 | is22_01_概要.md | 責務定義・4サービス構成 |
| 旧設計 | is22_02_collector設計.md | RTSP収集・差分判定 |
| 旧設計 | is22_03_dispatcher設計.md | 推論呼出・イベント化 |
| 旧設計 | is22_04_web設計.md | UI/API・MediaProxy |
| 旧設計 | is22_05_streamer設計.md | RTSP→HLS変換 |
| 旧設計 | is22_06_DB設計.md | MariaDBスキーマ |
| 旧設計 | is22_07_Drive連携設計.md | Google Drive保存 |
| 旧設計 | is22_08_通知設計.md | Webhook・日次要約 |
| 必須参照 | is22_Camserver仕様案 | 新仕様（SSoT） |

---

## 1. アーキテクチャ差異【重大】

### 1.1 サービス構成

| 観点 | 旧設計 | 新仕様 | ギャップ影響 |
|-----|--------|--------|-------------|
| 構成 | 4サービス（collector, dispatcher, web, streamer） | 11コンポーネント | **構造的変更必須** |
| 責務分離 | サービス単位 | 機能コンポーネント単位 | 再設計必要 |
| 依存方向 | サービス間直接呼出 | Adapter/Interface指向 | SOLID適用必要 |

#### 旧設計の4サービス
```
collector → dispatcher → web/streamer
     ↓          ↓
   MariaDB    is21
```

#### 新仕様の11コンポーネント
```
ConfigStore（SSoT）
SnapshotService
AIClient（is21 Adapter）
PollingOrchestrator
EventLogService
SuggestEngine
StreamGatewayAdapter
AdmissionController
RealtimeHub
WebAPI
IpcamScan
```

### 1.2 修正方針

| 項目 | 対応 |
|-----|------|
| collector | SnapshotService + PollingOrchestrator に分解 |
| dispatcher | AIClient + EventLogService + SuggestEngine に分解 |
| web | WebAPI + RealtimeHub に分解 |
| streamer | StreamGatewayAdapter としてAdapter化 |

### 1.3 新規追加コンポーネント

| コンポーネント | 旧設計での言及 | 新仕様での役割 |
|--------------|---------------|---------------|
| ConfigStore | なし | カメラ台帳・設定のSSoT |
| AdmissionController | なし | 負荷制御・モーダル許可/拒否 |
| SuggestEngine | なし | サジェスト対象決定 |
| IpcamScan | なし | カメラ自動発見 |

---

## 2. UI設計差異【重大】

### 2.1 UI基盤

| 観点 | 旧設計 | 新仕様 | ギャップ影響 |
|-----|--------|--------|-------------|
| フレームワーク | AraneaWebUI準拠 | React + shadcn/ui | 完全再設計 |
| レイアウト | Status/Network/Cloud/Tenant/System | 3ペイン（サジェスト/サムネ/ログ） | 概念変更 |
| ページタイトル | 未定義 | mobes AIcam control Tower (mAcT) | 新規 |

### 2.2 レイアウト変更

#### 旧設計（AraneaWebUIパターン）
- Status: デバイス状態表示
- Network: WiFi設定
- Cloud: クラウド接続状態
- Tenant: テナント設定
- System: システム管理

#### 新仕様（3ペイン）
- 左ペイン: サジェストライブ動画（全ユーザー共通）
- 中央ペイン: カメラサムネ羅列（ライブ中ハイライト）
- 右ペイン: AI解析ログ（リングバッファ表示）

### 2.3 新規UI機能

| 機能 | 旧設計 | 新仕様 | 対応 |
|-----|--------|--------|------|
| サジェスト再生 | なし | サーバー決定・全員共通 | 新規実装 |
| モーダル再生 | なし | ユーザー個別・TTL制限 | 新規実装 |
| 混雑時拒否メッセージ | なし | 明示的なUXフロー | 新規実装 |
| 長時間運用対策 | なし | リングバッファ・クリーンアップ | 新規実装 |

---

## 3. ストリーミング設計差異【中】

### 3.1 配信方式

| 観点 | 旧設計 | 新仕様 | ギャップ影響 |
|-----|--------|--------|-------------|
| 実装 | Rust + go2rtc固定 | StreamGatewayAdapter（差し替え可能） | Adapter化必要 |
| 方式 | HLS | WebRTC/HLS（選択可能） | 拡張設計必要 |
| 制御 | 直接API | Adapter経由 | インターフェース設計 |

### 3.2 修正項目

- [ ] go2rtcをAdapter経由で利用する設計に変更
- [ ] WebRTC対応の検討
- [ ] プリウォーム機能の設計

---

## 4. 負荷制御設計差異【重大・新規】

### 4.1 旧設計

- 明示的なAdmission Controlなし
- 上限設定なし
- 長時間運用対策なし

### 4.2 新仕様（AdmissionController）

| 機能 | 詳細 |
|-----|------|
| 予算モデル | TOTAL_STREAM_UNITS - RESERVED - SUGGEST = MODAL_BUDGET |
| 固定枠 | MAX_UI_USERS=10、サジェスト予約分 |
| CPU/メモリ監視 | 過負荷時はモーダル拒否 |
| ヒステリシス | フラップ防止 |

### 4.3 Lease/Heartbeat

| 項目 | 新仕様 |
|-----|--------|
| ModalLease | サーバー発行、TTLで失効 |
| Heartbeat | 15〜30秒周期 |
| 自動回収 | タブ放置・切断・閉じ忘れを回収 |

### 4.4 新規実装項目

- [ ] AdmissionController全体設計
- [ ] 予算モデル実装
- [ ] Lease管理テーブル
- [ ] Heartbeat API
- [ ] 拒否メッセージUI

---

## 5. スキャン機能差異【重大・新規】

### 5.1 旧設計

- IpcamScanへの言及なし
- カメラ登録は手動前提

### 5.2 新仕様（IpcamScan）

| Stage | 内容 |
|-------|------|
| 0 | 準備（CIDR正規化、既存DB優先） |
| 1 | Host Discovery（ICMP/TCP） |
| 2 | MAC/OUI取得（L2のみ） |
| 3 | ポート指紋（554/2020/80/443等） |
| 4 | ディスカバリ（WS-Discovery/SSDP/mDNS） |
| 5 | スコアリング |
| 6 | 確証（正規クレデンシャル） |

### 5.3 Phase 2検証結果との整合

| Phase 2で判明した事実 | IpcamScan設計への反映 |
|---------------------|---------------------|
| Tapo: 554+2020 open | Stage 3ポートセットに含む |
| Nest: 443+8443 open, RTSP/ONVIF無し | Stage 4 mDNS強化必要 |
| VIGI NVR: 80+443+8000+8443 | Stage 3拡張ポートセット |
| L3越しMAC取得不可 | Stage 2 mac=null許容確認 |
| WS-Discovery失敗 | Stage 4 unsupported記録 |

### 5.4 新規実装項目

- [ ] IpcamScanモジュール全体設計
- [ ] Stage 0〜6の実装
- [ ] スコアリングアルゴリズム
- [ ] Job/Device/Evidenceデータモデル
- [ ] API設計（POST/GET jobs, devices）

---

## 6. DB設計差異【中】

### 6.1 既存テーブル（維持）

| テーブル | 旧設計 | 新仕様 | 対応 |
|---------|--------|--------|------|
| cameras | ○ | ○ | スキーマ拡張 |
| frames | ○ | ○ | 維持 |
| events | ○ | ○ | 維持 |
| schema_versions | ○ | ○ | 維持（SSoT） |
| inference_jobs | ○ | 言及なし | 維持検討 |

### 6.2 新規テーブル

| テーブル | 新仕様 | 用途 |
|---------|--------|------|
| modal_leases | ○ | Lease管理 |
| ipcamscan_jobs | ○ | スキャンジョブ |
| ipcamscan_devices | ○ | 発見デバイス |
| ipcamscan_evidence | ○ | 証拠オブジェクト |
| policy_config | ○ | ポリシー設定 |
| thumb_cache | ○ | サムネキャッシュ |

### 6.3 camerasテーブル拡張

| カラム | 旧設計 | 新仕様 |
|--------|--------|--------|
| aiContextHint | なし | 追加（is21補助情報） |
| suggestPolicyWeight | なし | 追加（優先度調整） |
| vendorHint | なし | 追加（Tapo/VIGI/Nest/Other） |
| thumbSource | なし | 追加（sub or dedicated snapshot） |

---

## 7. 推論連携差異【中】

### 7.1 処理方式

| 観点 | 旧設計 | 新仕様 | ギャップ影響 |
|-----|--------|--------|-------------|
| 方式 | dispatcher→is21（DBキュー経由） | AIClient（同期Adapter） | 設計変更 |
| キュー | inference_jobs | 使用しない（同期優先） | 変更 |
| バックプレッシャー | 明示なし | 1台ずつ順次処理 | 明示化 |

### 7.2 AIClient設計

```
[PollingOrchestrator]
       ↓ 1台ずつ順次
[SnapshotService] → 画像取得
       ↓
[AIClient（Adapter）] → is21呼出
       ↓
[EventLogService] → イベント記録
       ↓
[SuggestEngine] → サジェスト判定
```

### 7.3 修正項目

- [ ] inference_jobsテーブルの廃止検討
- [ ] AIClient Adapter実装
- [ ] 同期処理フロー設計
- [ ] aiContextHint送信仕様確定

---

## 8. Drive連携・通知設計差異【低】

### 8.1 Drive連携

| 観点 | 旧設計 | 新仕様 | 判断 |
|-----|--------|--------|------|
| 設計詳細 | 詳細あり（is22_07） | 言及なし | **旧設計を維持** |
| フォルダ構成 | 定義済み | - | 維持 |
| TTL運用 | 定義済み | - | 維持 |

### 8.2 通知設計

| 観点 | 旧設計 | 新仕様 | 判断 |
|-----|--------|--------|------|
| 設計詳細 | 詳細あり（is22_08） | 概念的言及のみ | **旧設計を維持** |
| Webhook | 定義済み | - | 維持 |
| 日次要約 | 定義済み | - | 維持 |
| VLMエスカレーション | 定義済み | - | 維持 |

---

## 9. 修正優先度マトリクス

### 9.1 優先度定義

| 優先度 | 定義 | 対応時期 |
|--------|------|---------|
| P0 | 必須・ブロッカー | Phase 4前 |
| P1 | 重要・コア機能 | Phase 5-6 |
| P2 | 推奨・拡張機能 | Phase 7-8 |
| P3 | 任意・将来検討 | Phase 9以降 |

### 9.2 修正項目一覧

| ID | 項目 | 優先度 | 影響範囲 | 工数目安 |
|----|------|--------|---------|---------|
| GAP-001 | アーキテクチャ再設計（11コンポーネント化） | P0 | 全体 | 大 |
| GAP-002 | AdmissionController新規設計 | P0 | 負荷制御 | 中 |
| GAP-003 | UI 3ペイン設計（React+shadcn） | P1 | フロント | 大 |
| GAP-004 | IpcamScan新規実装 | P1 | スキャン | 大 |
| GAP-005 | Lease/Heartbeat機構 | P1 | 負荷制御 | 中 |
| GAP-006 | サジェストエンジン設計 | P1 | 表示制御 | 中 |
| GAP-007 | StreamGatewayAdapter化 | P2 | 配信 | 中 |
| GAP-008 | AIClient Adapter設計 | P1 | 推論連携 | 中 |
| GAP-009 | camerasテーブル拡張 | P1 | DB | 小 |
| GAP-010 | 新規テーブル追加（modal_leases等） | P1 | DB | 中 |
| GAP-011 | ConfigStore SSoT実装 | P0 | 設定管理 | 中 |
| GAP-012 | EventLogServiceリングバッファ | P1 | ログ | 小 |

---

## 10. 設計整合性確認

### 10.1 維持する旧設計

| 文書 | 判断理由 |
|-----|---------|
| is22_06_DB設計.md | 基本スキーマは新仕様でも必要 |
| is22_07_Drive連携設計.md | 新仕様でスコープ外のため維持 |
| is22_08_通知設計.md | 新仕様でスコープ外のため維持 |

### 10.2 全面改訂する旧設計

| 文書 | 改訂理由 |
|-----|---------|
| is22_01_概要.md | 11コンポーネント構成へ変更 |
| is22_02_collector設計.md | SnapshotService + PollingOrchestratorへ分解 |
| is22_03_dispatcher設計.md | 複数コンポーネントへ分解 |
| is22_04_web設計.md | 3ペインUIへ全面変更 |
| is22_05_streamer設計.md | Adapter化 |

### 10.3 新規作成する設計

| 文書 | 内容 |
|-----|------|
| is22_09_IpcamScan設計.md | 多段エビデンス方式詳細 |
| is22_10_AdmissionControl設計.md | 予算モデル・Lease |
| is22_11_Suggest設計.md | サジェストエンジン |
| is22_12_長時間運用設計.md | リングバッファ・クリーンアップ |

---

## 11. 次フェーズへの引き継ぎ

### Phase 4: 設計ドキュメント整備

本ギャップ分析に基づき、以下の順序で設計ドキュメントを整備する：

1. 旧設計ドキュメントの更新（is22_01〜05）
2. 新規設計ドキュメント作成（is22_09〜12）
3. 全体整合性チェック
4. 設計レビュー

### Phase 5: 実装計画・Issue登録

GAP-001〜012の各項目をGitHub Issueとして登録し、マイルストーンを設定する。

---

## 付録A: 12大原則との整合性

| 原則 | ギャップ分析での適用 |
|-----|---------------------|
| SSoT | ConfigStore設計で適用 |
| SOLID | コンポーネント分離で適用 |
| MECE | 機能分割で適用 |
| アンアンビギュアス | 状態・TTL・拒否条件の明示 |
| 事実ベース | Phase 2検証結果の反映 |
| 特記仕様遵守 | DDL維持 |
| 必須参照 | is22_Camserver仕様案をSSoT |
| 既存活用 | is21既存実装との連携 |
| 推論サイド | AIClient Adapter設計 |
| UI原則 | React+shadcn適用 |
| 車輪禁止 | go2rtc活用 |
| 負荷制御 | AdmissionController設計 |

---

## 更新履歴

| 日付 | バージョン | 内容 |
|-----|-----------|------|
| 2025-12-31 | 1.0 | 初版作成（Phase 3完了） |
