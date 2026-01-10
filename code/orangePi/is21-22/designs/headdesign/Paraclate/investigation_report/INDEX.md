# Paraclate実装フェーズ 調査報告インデックス

作成日: 2026-01-10
ステータス: 調査完了・統括アドバイザー指示待ち

## 大原則宣言

本調査は`The_golden_rules.md`に従い、以下の原則を遵守して実施:
- SSoT: 既存実装・ドキュメントとの整合性を優先
- MECE: 漏れなく重複なく項目を網羅
- アンアンビギュアス: 曖昧さを排除した明確な報告
- 車輪の再発明禁止: 既存ESP32/is20s実装パターンを参照

---

## 調査報告一覧

| # | ファイル | 内容 | MECE分類 |
|---|---------|------|----------|
| 1 | [01_is22_status.md](./01_is22_status.md) | IS22実装状態調査 | 現状把握 |
| 2 | [02_is21_status.md](./02_is21_status.md) | IS21実装状態調査 | 現状把握 |
| 3 | [03_authentication.md](./03_authentication.md) | AraneaRegister/LacisOath認証要件 | 機能要件 |
| 4 | [04_summary_api.md](./04_summary_api.md) | Summary/GrandSummary API要件 | 機能要件 |
| 5 | [05_paraclate_integration.md](./05_paraclate_integration.md) | Paraclate APP連携要件 | 機能要件 |
| 6 | [06_implementation_tasks.md](./06_implementation_tasks.md) | 実装タスク一覧（依存順） | 実装計画 |

---

## 調査サマリー

### IS22 (Paraclate Server) 実装状態

| カテゴリ | 実装状況 | 詳細 |
|---------|---------|------|
| コアモジュール | ✅ 28モジュール | polling, suggest, stream等 |
| APIエンドポイント | ✅ 117個 | カメラ管理、統計、SDM等 |
| DBスキーマ | ✅ 19マイグレーション | ai_summary_cache等定義済 |
| AraneaRegister | ❌ 未実装 | LacisOath認証なし |
| Summary API | ❌ 未実装 | DB定義のみ、エンドポイントなし |
| Paraclate連携 | ⚠️ プレースホルダー | フロントエンドUI実装済 |

### IS21 (ParaclateEdge) 実装状態

| カテゴリ | 実装状況 | 詳細 |
|---------|---------|------|
| AI推論 | ✅ v1.8.0 | YOLO+PAR、camera_context対応 |
| aranea_common | ✅ 実装済 | 認証登録フロー実装 |
| 管理WebUI | ❌ なし | APIのみ |
| 認証フロー | ⚠️ 検証不足 | araneaDeviceGate接続未確認 |

### 未実装項目（依存順）

```
1. [is22] AraneaRegister (Rust版)
   ↓ 依存
2. [is22] TID/FID権限管理
   ↓ 依存
3. [is22] Summary/GrandSummary API
   ↓ 依存
4. [is22] Paraclate APP連携
   ↓ 依存
5. [is21] 管理WebUI
```

---

## 関連ドキュメント参照

### Paraclate設計
- [Paraclate_DesignOverview.md](../Paraclate_DesignOverview.md) - 設計概要（本調査の指示元）

### 既存設計書
- `is22_AI_EVENT_LOG_DESIGN.md` - AI Event Log設計
- `Layout＆AIlog_Settings/` - AIチャット・サジェスト設計

### 参照実装
- `code/ESP32/global/src/AraneaRegister.cpp` - ESP32認証登録実装
- `code/orangePi/is21/src/aranea_common/` - Python版認証実装

---

## 次のアクション

**統括アドバイザーへの確認事項:**

1. **認証基盤優先度**: AraneaRegister実装とParaclate機能実装の優先順位
2. **mobes2.0連携範囲**: 今フェーズで実装すべきParaclate APP連携の範囲
3. **TID移行**: 市山水産(fid9000)からmijeo.inc(tid)への移行タイミング
4. **BigQuery同期**: Summary送信前にBQ同期を完成させるか

---

## MECE確認

本調査は以下の観点でMECEを満たす:
- **現状把握**: is21/is22の全モジュール・API・DBスキーマを網羅
- **機能要件**: 認証、Summary、Paraclate連携を分離して記述
- **実装計画**: 依存関係に基づく実装順序を明確化
- **未カバー領域**: mobes2.0リポジトリ（プライベート）の詳細仕様は範囲外として明示
