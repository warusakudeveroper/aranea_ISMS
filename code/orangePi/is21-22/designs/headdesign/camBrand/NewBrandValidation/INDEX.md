# 新規カメラブランド検証手順書

## 概要

新規カメラブランドをIS22に導入する際の検証手順書。接続テストからストレステストまでの全工程を網羅し、間違いなく導入できるようにする。

## 関連ドキュメント

### 検証ドキュメント（本ディレクトリ）

| ファイル | 内容 |
|----------|------|
| [VALIDATION_PROCEDURE.md](./VALIDATION_PROCEDURE.md) | 検証手順書（メイン） |
| [scripts/](./scripts/) | テストスクリプト一式 |
| [TEMPLATE_STRESS_TEST_REPORT.md](./TEMPLATE_STRESS_TEST_REPORT.md) | ストレステストレポートテンプレート |
| [CHECKLIST.md](./CHECKLIST.md) | 検証チェックリスト |

### 実装ドキュメント（NewBrandIntegration/）

| ファイル | 内容 |
|----------|------|
| [../NewBrandIntegration/INDEX.md](../NewBrandIntegration/INDEX.md) | 統合ガイド総合 |
| [../NewBrandIntegration/REGISTRATION_FLOW_ANALYSIS.md](../NewBrandIntegration/REGISTRATION_FLOW_ANALYSIS.md) | IS22カメラ登録フロー詳細解析 |
| [../NewBrandIntegration/IMPLEMENTATION_TASKS.md](../NewBrandIntegration/IMPLEMENTATION_TASKS.md) | 実装タスク一覧 |
| [../NewBrandIntegration/CREDENTIAL_HANDLING.md](../NewBrandIntegration/CREDENTIAL_HANDLING.md) | クレデンシャル管理詳細 |

### Access Absorber（accessAbsorber/）

| ファイル | 内容 |
|----------|------|
| [../accessAbsorber/SPECIFICATION.md](../accessAbsorber/SPECIFICATION.md) | 接続制限機能仕様 |
| [../accessAbsorber/DB_SCHEMA.md](../accessAbsorber/DB_SCHEMA.md) | DBスキーマ |
| [../accessAbsorber/UI_FEEDBACK_SPEC.md](../accessAbsorber/UI_FEEDBACK_SPEC.md) | UIフィードバック仕様 |

## 検証フロー概要

```
1. 事前情報収集
   ├── 製品情報確認
   └── ネット上の技術情報調査
          ↓
2. 基本接続テスト
   ├── ポートスキャン
   ├── OUI調査
   ├── ONVIF接続確認
   └── RTSP接続確認
          ↓
3. ストレステスト
   ├── 再接続テスト
   ├── 並列接続テスト
   ├── 長時間ストリームテスト
   └── HTTP/ONVIF負荷テスト
          ↓
4. 結果分析・設定値決定
   ├── Access Absorber設定値決定
   └── RTSPテンプレート作成
          ↓
5. ドキュメント作成
   └── レポート・チェックリスト完成
```
