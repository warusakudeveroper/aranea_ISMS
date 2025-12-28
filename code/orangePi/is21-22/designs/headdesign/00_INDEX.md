# is21-22 基本設計図書 インデックス

作成日: 2025-12-29
ステータス: Draft

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
| 文書番号 | 文書名 | 概要 |
|---------|--------|------|
| is22_01 | 概要・責務定義 | is22の役割と責務境界 |
| is22_02 | collector設計 | RTSP収集・差分判定 |
| is22_03 | dispatcher設計 | 推論呼出・イベント化・通知 |
| is22_04 | web設計 | UI/API・MediaProxy |
| is22_05 | streamer設計 | RTSP→HLS変換 |
| is22_06 | DB設計 | MariaDBスキーマ・運用 |
| is22_07 | Drive連携設計 | Google Drive保存・隔離 |
| is22_08 | 通知設計 | Webhook・日次要約・クールダウン |

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
