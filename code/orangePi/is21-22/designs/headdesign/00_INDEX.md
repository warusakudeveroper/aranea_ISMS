# is21-22 基本設計図書 インデックス

作成日: 2025-12-29
更新日: 2026-01-07
ステータス: Phase 6進行中（GoogleDevices SDM/Nest統合）

---

## 文書構成

### is21（camimageEdge AI - 推論サーバ）
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| is21_01 | 概要・責務定義 | is21の役割と責務境界 |
| is21_02 | API設計 | 推論API・スキーマAPI仕様 |
| is21_03 | 推論パイプライン | 検出・属性抽出・タグ生成 |
| is21_04 | 運用設計 | systemd・ログ・ヘルスチェック |

### is22（camServer - メインサーバ）

#### 旧設計（レガシー・参照用）
| 文書番号 | 文書名 | 概要 | 状態 |
|---------|--------|------|------|
| is22_01 | 概要・責務定義 | is22の役割と責務境界 | 要更新 |
| is22_02 | collector設計 | RTSP収集・差分判定 | 要更新 |
| is22_03 | dispatcher設計 | 推論呼出・イベント化・通知 | 要更新 |
| is22_04 | web設計 | UI/API・MediaProxy | 要更新 |
| is22_05 | streamer設計 | RTSP→HLS変換 | 要更新 |
| is22_06 | DB設計 | MariaDBスキーマ・運用 | 維持 |
| is22_07 | Drive連携設計 | Google Drive保存・隔離 | 維持 |
| is22_08 | 通知設計 | Webhook・日次要約・クールダウン | 維持 |

#### 新規設計（Phase 4成果物）
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| is22_09 | IpcamScan設計 | 多段エビデンス方式のカメラ自動発見 |
| is22_10 | AdmissionControl設計 | 予算モデル・Lease/Heartbeat |
| is22_11 | Suggest設計 | サジェストエンジン・スコアリング |
| is22_12 | 長時間運用設計 | リングバッファ・クリーンアップ |

#### Google/Nest (SDM) 追加設計
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| GD-01 | GoogleDevices_introduction_BasicDesign | Nest Doorbell を SDM で統合する基本設計 |
| GD-02 | GoogleDevices_introduction_Environment | SDM 設定の事前準備・認可取得手順（コード外） |
| GD-03 | GoogleDevices_introduction_DetailedDesign | SDM 連携の詳細設計（バックエンド/フロント/運用/テスト） |
| GD-04 | GoogleDevices_camscan_alignment | SDM/Nest と IpcamScan（Camscan）設計の統合ポイント |
| GD-05 | GoogleDevices_settings_wizard_spec | 設定モーダルの Google/NestタブとSDMウィザードの詳細設計 |
| GD-06 | GoogleDevices_review | 設計レビュー結果（The_golden_rules準拠確認） |
| GD-07 | TASK_INDEX | 実装タスクインデックス（依存関係・実装順・Issue対応表） |
| GD-08 | TEST_CHECKLIST | テストチェックリスト（98項目・MECE・アンアンビギュアス） |

#### Layout＆AIlog_Settings（UI改善・設定機能）
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| LAS-01 | UI_Review1 | カメラタイル表示仕様準拠設計（GAP-T01〜T04修正） |
| LAS-02 | AIEventlog | AIイベントログ設定タブ追加設計 |

### 共通
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| COMMON_01 | UIガイドライン | araneaDevices共通UI原則 |
| COMMON_02 | セキュリティ設計 | 認証・署名・秘匿 |
| COMMON_03 | 運用設計 | ログ・監視・バックアップ |

### 計画・設計管理
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| INVESTIGATION_INDEX | 調査項目インデックス | 24項目の調査・Issue管理 |
| DETAILED_DESIGN | 詳細設計書 | Phase0-2の実装詳細 |
| TEST_PLAN | テスト計画書 | 単体・統合・E2E・性能テスト |
| IMPLEMENTATION_TASKS | 実装タスク算定 | 24タスク・マイルストーン |
| DESIGN_GAP_ANALYSIS | 設計修正項目インデックス | 旧設計と新仕様のギャップ分析（Phase 3成果物） |
| ISSUE_REGISTRY | Issue登録・管理 | GAP-001〜012のIssue化・進捗管理（Phase 5成果物） |

### 実装報告
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| IS22_UI_IMPLEMENTATION_REPORT | UI実装完了報告 | IS22-003/004のUI実装・テスト結果（Phase 5成果物） |

### 検証記録
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| is22_Phase2_カメラ疎通検証記録 | Phase 2検証記録 | カメラ検出・疎通テスト結果 |

### 必須参照（改変禁止）
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| is22_Camserver実装指示 | 実装指示書 | 実装手順・大原則 |
| is22_Camserver仕様案(基本設計書草案改訂版) | 基本設計書 | IS22全体仕様 |

---

## 設計原則

### SSoT（Single Source of Truth）
- スキーマ定義: is22のschema_versionsテーブルが唯一の情報源
- カメラ設定: is22のcamerasテーブルが唯一の情報源
- is21は設定を「受け取る」のみ、自ら管理しない

### SOLID原則の適用
- **S（単一責任）**: is21=推論のみ、is22=その他全て
- **O（開放閉鎖）**: タグはschema.jsonで拡張、コード変更不要
- **L（置換原則）**: 推論モデルは差し替え可能（APIは同一）
- **D（依存逆転）**: is22→is21への依存のみ（逆方向禁止）

### 車輪の再発明禁止
- 既存AraneaWebUIパターンを踏襲（Status/Network/Cloud/Tenant/System）
- 既存is20sのPython実装パターンを参照
- DDL/OpenAPI定義は特記仕様書を正とする

---

## 依存関係図

### 新アーキテクチャ（11コンポーネント）

```
[RTSPカメラ×30] ←────────── [IpcamScan] ← カメラ自動発見
       ↓
[SnapshotService] ← 画像取得
       ↓
[PollingOrchestrator] ← 1台ずつ順次
       ↓
[AIClient(Adapter)] ─────────→ [is21] ← 推論リクエスト
       ↓
[EventLogService] ← イベント記録（リングバッファ）
       ↓
┌──────┴──────┐
↓              ↓
[SuggestEngine]  [MariaDB] ← ConfigStore(SSoT)
↓              ↓
[RealtimeHub] ← WebSocket/SSE配信
↓
[WebAPI] ← REST API
       ↓
[StreamGatewayAdapter] ─→ [go2rtc] ← ストリーム配信
       ↓
[AdmissionController] ← 負荷制御・Lease管理
       ↓
   [ブラウザ] ← 3ペインUI (React+shadcn)
```

### 旧アーキテクチャ（参照用）

```
[RTSPカメラ×30]
       ↓
   [is22.collector]  ← RTSP取得・差分判定
       ↓
   [is22.dispatcher] ← DBキュー
       ↓
   [is21] ← 推論リクエスト（infer画像）
       ↓
   [is22.dispatcher] ← 結果取得
       ↓
   [MariaDB] ← frames/events保存
       ↓
   [Google Drive] ← フル画像保存（条件付き）
       ↓
   [is22.web] ← UI/API提供
       ↓
   [ブラウザ]
```

---

## 参照文書
- 基本設計書: `../is22CamServer_is21CamImageEdge 基本設計書.md`
- 特記仕様1: DDL・schema.json・OpenAPI
- 特記仕様2: プロキシURL・イベント化ロジック・Drive隔離
- 特記仕様3: Rustワークスペース・ほぼ実装テンプレート
