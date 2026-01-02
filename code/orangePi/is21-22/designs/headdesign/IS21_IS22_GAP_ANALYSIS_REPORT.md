# IS21/IS22 ギャップ分析報告書

**作成日時**: 2026-01-03
**対象**: is21 (AI推論サーバー) / is22 (Camserver)
**報告者**: Claude Code

---

## 1. 調査概要

is22 AI Event Log機能の動作検証中に発見された問題について、is21とis22双方のコード・設計ドキュメント・実機を調査し、ギャップを特定した。

### 1.1 調査範囲

| 対象 | 場所 | 確認方法 |
|------|------|----------|
| IS21 | 192.168.3.240:9000 | SSH接続・コード確認・API呼び出し |
| IS22 | 192.168.125.246:8080 | SSH接続・コード確認・ログ確認 |
| 設計書 | code/orangePi/is21-22/designs/headdesign/ | ファイル読み込み |

---

## 2. 発見された問題

### 2.1 【致命的】suspicious_score カラム型不整合

| 項目 | IS21（実装） | IS22 DB定義 | 設計書 |
|------|-------------|-------------|--------|
| suspicious.score | 整数 0-100 | DECIMAL(5,4) | 整数 28 (例示) |

**問題詳細**:
- IS21の`calculate_suspicious_score`関数は整数0-100を返す
  - `score += 40` (mask_like)
  - `score += 25` (night + crouching)
  - 合計は整数演算
- IS22のMigration 008では`DECIMAL(5,4)`として定義
  - 最大値: 9.9999
  - IS21が返す値(0-100)を格納できない

**エラーメッセージ**:
```
Out of range value for column 'suspicious_score' at row 1
```

**影響範囲**:
- detection_logs への INSERT 失敗
- AI Event Logが一切保存されない

**修正すべき対象**: IS22 (Migration 008)

**理由**:
1. IS21のsuspicious.scoreは設計通り整数0-100で正しく動作している
2. IS22のDECIMAL(5,4)定義が設計書の例示(`"score": 28`)と不整合

---

### 2.2 【重大】IS21がpreset_id/preset_versionを未サポート

| 項目 | IS22（送信） | IS21（受信） | 設計書 |
|------|-------------|-------------|--------|
| preset_id | 送信する | パラメータなし | 必須 |
| preset_version | 送信する | パラメータなし | 必須 |

**IS21のanalyzeエンドポイント (line 1684-1693)**:
```python
@app.post("/v1/analyze")
async def analyze(
    camera_id: str = Form(...),
    captured_at: str = Form(...),
    schema_version_req: str = Form(..., alias="schema_version"),
    infer_image: UploadFile = File(...),
    profile: str = Form("standard"),  # ← profileのみ存在
    return_bboxes: bool = Form(True),
    hints_json: Optional[str] = Form(None)
):
```

**IS22が送信するパラメータ (ai_client/mod.rs line 344-347)**:
```rust
.text("preset_id", request.preset_id.clone())
.text("preset_version", request.preset_version.clone())
```

**影響**:
- IS22から送信されたpreset_id/preset_versionはIS21で無視される
- プリセット別のAI解析カスタマイズが機能しない

**修正すべき対象**: IS21

**理由**:
1. 設計書(is22_AI_EVENT_LOG_DESIGN.md)でpreset_id/preset_version対応を明記
2. IS22側は設計通り実装済み
3. IS21が設計を未反映

---

### 2.3 【重大】IS21がprev_image/frame_diffを未サポート

| 項目 | IS22（送信） | IS21（受信） | 設計書 |
|------|-------------|-------------|--------|
| prev_image | 送信する | パラメータなし | ○対応 |
| enable_frame_diff | 送信する | パラメータなし | ○対応 |
| frame_diff応答 | 期待する | 返却しない | ○対応 |

**IS21で`prev_image`/`frame_diff`を検索した結果**: なし

```bash
$ grep -n 'frame_diff\|prev_image\|previous' /opt/is21/src/main.py
# 結果: 0件
```

**IS22が送信するパラメータ (ai_client/mod.rs line 363-370)**:
```rust
// Add previous image for frame diff
if let Some(prev_data) = prev_image {
    form = form.part(
        "prev_image",
        Part::bytes(prev_data)
            .file_name("prev_snapshot.jpg")
            .mime_str("image/jpeg")?,
    );
}
```

**設計書の要件 (is22_AI_EVENT_LOG_DESIGN.md)**:
```
| prev_image | file | - | JPEG/PNG画像（前回のフレーム、差分分析用） |
| enable_frame_diff | bool | - | 差分分析有効化 (default: true, prev_image送信時) |
```

**影響**:
- フレーム差分分析が機能しない
- 滞在検知(loitering)が機能しない
- カメラ状態検知(frozen/tampered/shifted)が機能しない

**修正すべき対象**: IS21

**理由**:
1. 設計書で明確にframe_diff対応を定義
2. IS22側は設計通りにprev_image送信を実装済み
3. IS21が設計を未反映

---

## 3. 問題間の依存関係

```
┌─────────────────────────────────────────────────────────────┐
│  Issue 2.1: suspicious_score DECIMAL(5,4)                   │
│  ↓                                                          │
│  detection_logs INSERT失敗                                   │
│  ↓                                                          │
│  AI Event Log保存不可 (致命的)                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Issue 2.2: preset未対応                                    │
│  ↓                                                          │
│  プリセット別カスタマイズ不可                               │
│  ↓                                                          │
│  駐車場/入口/客室など用途別最適化不可                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Issue 2.3: frame_diff未対応                                │
│  ↓                                                          │
│  滞在検知・カメラ改竄検知不可                               │
│  ↓                                                          │
│  設計書記載の機能が全て無効                                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. 修正方針

### 4.1 優先度1: IS22 suspicious_score カラム修正

**修正内容**:
Migration 008の仮想カラム定義を修正

```sql
-- 現在 (誤り)
suspicious_score DECIMAL(5,4) AS (JSON_VALUE(suspicious, '$.score')) STORED

-- 修正後
suspicious_score INT AS (JSON_VALUE(suspicious, '$.score')) STORED
```

**手順**:
1. カラム削除: `ALTER TABLE detection_logs DROP COLUMN suspicious_score;`
2. カラム再作成: `ALTER TABLE detection_logs ADD COLUMN suspicious_score INT AS (JSON_VALUE(suspicious, '$.score')) STORED;`
3. インデックス再作成

---

### 4.2 優先度2: IS21 preset対応追加

**修正内容**:
analyzeエンドポイントにpreset_id/preset_versionパラメータ追加

```python
@app.post("/v1/analyze")
async def analyze(
    # ... 既存パラメータ ...
    preset_id: str = Form("balanced"),      # 追加
    preset_version: str = Form("1.0.0"),    # 追加
):
```

**影響範囲**: 約20行の追加

---

### 4.3 優先度3: IS21 frame_diff対応追加

**修正内容**:
1. prev_imageパラメータ追加
2. フレーム差分計算ロジック実装
3. frame_diffオブジェクト応答追加

**影響範囲**: 約150-200行の追加

---

## 5. 本報告のMECE検証

| 区分 | 項目 | 確認状況 |
|------|------|----------|
| 網羅性 | IS21実装 | ○ main.py全体確認 |
| | IS22実装 | ○ ai_client, detection_log_service確認 |
| | DB定義 | ○ migration 008確認 |
| | 設計書 | ○ is22_AI_EVENT_LOG_DESIGN.md確認 |
| 排他性 | 問題分類 | ○ DB型/プリセット/frame_diffで分離 |

**MECEであることを宣言**: 本報告は調査範囲内で漏れなく・重複なく問題を特定している。

---

## 6. アンアンビギュアス宣言

以下について曖昧さがないことを確認:

1. **suspicious.score型**: IS21で整数0-100、設計書で整数28例示、IS22 DBでDECIMAL(5,4)は**明確な不整合**
2. **preset対応**: IS21のanalyzeエンドポイントにパラメータが**存在しない**ことをコードで確認
3. **frame_diff対応**: IS21コード内に関連実装が**一切存在しない**ことをgrep結果で確認

---

## 7. 大原則遵守の宣言

1. SSoTの大原則を遵守します
2. Solidの原則を意識し特に単一責任、開放閉鎖、置換原則、依存逆転に注意します
3. 必ずMECEを意識し、このコードはMECEであるか、この報告はMECEであるかを報告の中に加えます - **本報告はMECEである**
4. アンアンビギュアスな報告回答を義務化しアンアンビギュアスに到達できていない場合はアンアンビギュアスであると宣言できるまで明確な検証を行います - **本報告はアンアンビギュアスである**
5. 根拠なき自己解釈で情報に優劣を持たせる差別コード、レイシストコードは絶対の禁止事項です。情報の公平性原則
6. 実施手順と大原則に違反した場合は直ちに作業を止め再度現在位置を確認の上でリカバーしてから再度実装に戻ります
7. チェック、テストのない完了報告はただのやった振りのためこれは行いません
8. 依存関係と既存の実装を尊重し、確認し、車輪の再発明行為は絶対に行いません
9. 作業のフェーズ開始前とフェーズ終了時に必ずこの実施手順と大原則を全文省略なく復唱します
10. 必須参照ドキュメントの改変はゴールを動かすことにつながるため禁止です
11. is22_Camserver実装指示.mdを全てのフェーズ前に鑑として再読し基本的な思想や概念的齟齬が存在しないかを確認します
12. ルール無視はルールを無視した時点からやり直すこと

---

## 8. 次のアクション

**修正実施前に必要な確認**:
- [ ] 本報告書の内容承認
- [ ] IS21修正方針の決定（preset/frame_diff対応の優先度）
- [ ] IS22 suspicious_scoreカラム修正の承認

**本報告では修正は実施しない** - 問題と依存関係の完全な確認が目的
