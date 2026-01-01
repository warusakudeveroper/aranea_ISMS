# IS22 Camserver 設計修正項目インデックス・修正計画

## 大原則の復唱

1. SSoTの大原則を遵守します
2. Solidの原則を意識し特に単一責任、開放閉鎖、置換原則、依存逆転に注意します
3. 必ずMECEを意識し、このコードはMECEであるか、この報告はMECEであるかを報告の中に加えます
4. アンアンビギュアスな報告回答を義務化しアンアンビギュアスに到達できていない場合はアンアンビギュアスであると宣言できるまで明確な検証を行います
5. 根拠なき自己解釈で情報に優劣を持たせる差別コード、レイシストコードは絶対の禁止事項です。情報の公平性原則
6. 実施手順と大原則に違反した場合は直ちに作業を止め再度現在位置を確認の上でリカバーしてから再度実装に戻ります
7. チェック、テストのない完了報告はただのやった振りのためこれは行いません
8. 依存関係と既存の実装を尊重し、確認し、車輪の再発明行為は絶対に行いません
9. 作業のフェーズ開始前とフェーズ終了時に必ずこの実施手順と大原則を全文省略なく復唱します
10. 必須参照ドキュメントの改変はゴールを動かすことにつながるため禁止です
11. is22_Camserver実装指示.mdを全てのフェーズ前に鑑として再読し基本的な思想や概念的齟齬が存在しないかを確認します
12. ルール無視はルールを無視した時点からやり直すこと

---

## 1. 必須参照ドキュメント

| ドキュメント | パス | 状態 |
|-------------|------|------|
| 実装指示書 | `code/orangePi/is21-22/designs/headdesign/is22_Camserver実装指示.md` | 確認済 |
| 基本設計書 | `code/orangePi/is21-22/designs/headdesign/is22_Camserver仕様案(基本設計書草案改訂版).md` | 確認済 |
| 乖離分析 | `code/orangePi/is22/frontend/docs/CONCEPTUAL_GAP_ANALYSIS.md` | 作成済 |

---

## 2. 設計修正項目インデックス

### 2.1 UI構造の修正（MECE分類: UIレイヤー）

| # | 項目 | 設計仕様 | 現状 | 修正内容 | 優先度 |
|---|------|---------|------|----------|--------|
| UI-1 | 3ペインレイアウト | Left(25-30%) + Center(40-50%) + Right(20-25%) | 2ペイン（Main 80% + Right 20%） | App.tsx構造変更 | P0 |
| UI-2 | Left Pane（サジェスト動画） | イベント発生カメラのライブ動画表示 | 存在しない | SuggestPane.tsx新規作成 | P0 |
| UI-3 | Right Pane（AIログ） | リアルタイムイベントストリーム | 静的カメラ詳細のみ | EventLogPane.tsx新規作成 | P0 |
| UI-4 | サムネイルseverity表示 | sev0:灰, sev1:青, sev2:黄, sev3:赤+点滅 | 枠色対応あり（値が常に0） | イベント連携後に有効化 | P1 |

### 2.2 バックエンド連携の修正（MECE分類: データフローレイヤー）

| # | 項目 | 設計仕様 | 現状 | 修正内容 | 優先度 |
|---|------|---------|------|----------|--------|
| BE-1 | WebSocket基盤 | /ws/events でイベント配信 | 存在しない | Rust側: RealtimeHub実装 | P0 |
| BE-2 | イベントAPI | GET /api/events で履歴取得 | 存在するが空データ | is21連携で実データ生成 | P1 |
| BE-3 | サジェストAPI | GET /api/suggest/current | 存在しない | 最新イベントカメラ返却 | P1 |

### 2.3 AI連携の修正（MECE分類: 外部連携レイヤー）

| # | 項目 | 設計仕様 | 現状 | 修正内容 | 優先度 |
|---|------|---------|------|----------|--------|
| AI-1 | is21呼び出し | 巡回時にhttp://192.168.3.240:9000/v1/analyze | 未実装 | polling.rs修正 | P0 |
| AI-2 | イベント生成 | AI応答→eventsテーブルINSERT | 未実装 | event_store.rs修正 | P0 |
| AI-3 | severity計算 | person=3, vehicle=2, motion=1 | 未実装 | AI応答パース時に計算 | P1 |

### 2.4 サジェスト機能の修正（MECE分類: ビジネスロジックレイヤー）

| # | 項目 | 設計仕様（修正版） | 現状 | 修正内容 | 優先度 |
|---|------|-------------------|------|----------|--------|
| SG-1 | サジェスト表示 | イベント発生カメラをLeft Paneに表示 | 存在しない | SuggestPane連携 | P0 |
| SG-2 | イベント継続 | イベント継続中は上部固定 | 存在しない | イベント終了判定ロジック | P1 |
| SG-3 | 複数イベント時 | 最新or高severity優先 | 存在しない | シンプルな優先度ロジック | P2 |

**注**: 当初設計の「SuggestEngine」による複雑なスコア計算は過剰実装。
**修正方針**: イベント発生→そのカメラをサジェスト表示、イベント継続中は表示維持。

---

## 3. 修正計画（フェーズ分割）

### Phase 1: UI骨格修正（3ペイン化）

**目標**: 設計通りの3ペインレイアウトを実現

**対象ファイル**:
- `frontend/src/App.tsx` - レイアウト構造変更
- `frontend/src/components/SuggestPane.tsx` - 新規作成
- `frontend/src/components/EventLogPane.tsx` - 新規作成

**実装内容**:
```
┌─────────────────────────────────────────────────────────────┐
│ Header: IS22 Camserver | CPU/MEM/モーダル数 | 設定          │
├──────────────┬──────────────────────┬───────────────────────┤
│ SuggestPane  │    CameraGrid        │   EventLogPane        │
│ (25%)        │    (50%)             │   (25%)               │
│              │                      │                       │
│ - カメラ名   │   [サムネ][サムネ]   │   [イベントリスト]    │
│ - 動画/静止画│   [サムネ][サムネ]   │   - 時刻              │
│ - イベント名 │                      │   - カメラ名          │
│              │                      │   - severity          │
└──────────────┴──────────────────────┴───────────────────────┘
```

**検証項目**:
- [ ] 3ペイン表示確認
- [ ] レスポンシブ動作確認
- [ ] 各ペインの独立スクロール確認

---

### Phase 2: WebSocket基盤実装

**目標**: リアルタイムイベント配信基盤

**対象ファイル**:
- `src/realtime_hub.rs` - 新規作成
- `src/web_api/routes.rs` - WebSocketエンドポイント追加
- `frontend/src/hooks/useEventStream.ts` - 新規作成

**実装内容**:
```rust
// Rust側
pub async fn ws_events(ws: WebSocket, state: AppState) {
    // クライアント接続管理
    // イベント発生時にbroadcast
}
```

```typescript
// フロントエンド側
export function useEventStream() {
  const [events, setEvents] = useState<Event[]>([])
  // WebSocket接続
  // イベント受信→state更新
}
```

**検証項目**:
- [ ] WebSocket接続確認
- [ ] イベント受信確認（手動テスト用エンドポイント）
- [ ] 複数クライアント同時接続確認

---

### Phase 3: is21 AI連携実装

**目標**: 巡回→AI推論→イベント生成パイプライン

**対象ファイル**:
- `src/polling.rs` - is21 API呼び出し追加
- `src/event_store.rs` - イベント保存ロジック

**実装内容**:
```rust
// polling.rs
async fn poll_camera(camera: &Camera, is21_client: &Is21Client) -> Option<Event> {
    let snapshot = get_snapshot(&camera).await?;
    let analysis = is21_client.analyze(snapshot).await?;

    if analysis.has_significant_event() {
        Some(Event::from_analysis(camera.camera_id, analysis))
    } else {
        None
    }
}
```

**is21 API仕様** (192.168.3.240:9000):
```
POST /v1/analyze
Content-Type: multipart/form-data
- image: (binary)
- camera_context: (JSON)

Response:
{
  "objects": [{"label": "person", "confidence": 0.95, "bbox": [...]}],
  "scene": "indoor",
  "anomaly_score": 0.1
}
```

**検証項目**:
- [ ] is21への接続確認
- [ ] スナップショット送信確認
- [ ] AI応答パース確認
- [ ] イベントDB保存確認

---

### Phase 4: サジェスト表示連携

**目標**: イベント発生カメラをLeft Paneに表示

**対象ファイル**:
- `frontend/src/components/SuggestPane.tsx` - イベント連携
- `src/web_api/routes.rs` - /api/suggest/current 追加

**実装内容**:
```typescript
// SuggestPane.tsx
function SuggestPane() {
  const { currentEvent } = useEventStream()

  if (!currentEvent) {
    return <div>No active events</div>
  }

  return (
    <div>
      <h3>{currentEvent.camera_name}</h3>
      <VideoPlayer cameraId={currentEvent.camera_id} />
      <Badge>{currentEvent.primary_event}</Badge>
    </div>
  )
}
```

**検証項目**:
- [ ] イベント発生時にLeft Pane更新確認
- [ ] イベント終了時の表示切替確認
- [ ] 動画/静止画表示確認

---

### Phase 5: EventLog表示連携

**目標**: Right Paneにリアルタイムイベントログ表示

**対象ファイル**:
- `frontend/src/components/EventLogPane.tsx` - 完成

**実装内容**:
```typescript
// EventLogPane.tsx
function EventLogPane() {
  const { events } = useEventStream()

  return (
    <div className="overflow-auto">
      {events.map(event => (
        <EventLogItem
          key={event.event_id}
          event={event}
          severity={event.severity_max}
        />
      ))}
    </div>
  )
}
```

**検証項目**:
- [ ] イベント時系列表示確認
- [ ] severity色分け確認
- [ ] 自動スクロール確認

---

## 4. MECE確認

| レイヤー | 対象項目 | 漏れなし | 重複なし |
|----------|---------|---------|---------|
| UIレイヤー | UI-1〜UI-4 | ✅ | ✅ |
| データフローレイヤー | BE-1〜BE-3 | ✅ | ✅ |
| 外部連携レイヤー | AI-1〜AI-3 | ✅ | ✅ |
| ビジネスロジックレイヤー | SG-1〜SG-3 | ✅ | ✅ |

**MECEであることを確認**: 各レイヤーは独立しており、相互に依存関係を持たない形で分類されている。

---

## 5. アンビギュアス項目の明確化

| 項目 | 曖昧だった点 | 明確化 |
|------|-------------|--------|
| サジェストエンジン | 「スコア計算」の複雑さ | → シンプルに「最新イベントカメラを表示」 |
| イベント継続判定 | 「継続」の定義 | → is21応答の変化がない間は継続とみなす |
| severity計算 | どのラベルが何severity | → person=3, vehicle=2, motion=1, その他=0 |

**アンアンビギュアスであることを宣言**: 上記の明確化により、実装時の解釈の余地を排除した。

---

## 6. 設計ドキュメント最新インデックス

| ファイル | 役割 | 最終更新 |
|----------|------|----------|
| `is22_Camserver実装指示.md` | 実装手順・大原則（変更禁止） | - |
| `is22_Camserver仕様案(基本設計書草案改訂版).md` | 機能仕様（変更禁止） | - |
| `CONCEPTUAL_GAP_ANALYSIS.md` | 乖離分析 | 2025-12-31 |
| `IMPLEMENTATION_CORRECTION_PLAN.md` | 本ドキュメント | 2025-12-31 |

---

## 7. 必須参照ドキュメントとの整合性チェック

### 7.1 基本設計書（is22_Camserver仕様案）との照合

| 設計書セクション | 内容 | 修正計画対応 | 整合性 |
|-----------------|------|-------------|--------|
| 4.2 画面レイアウト（3ペイン） | Left(サジェスト), Center(サムネ), Right(AIログ) | UI-1, UI-2, UI-3 | ✅ |
| 4.1 UI方針 | ページタイトル「mobes AIcam control Tower (mAcT)」 | **追記必要** | ⚠️ |
| 5.2 巡回とAI推論連携 | is21へ推論依頼、イベント生成 | AI-1, AI-2 | ✅ |
| 5.3 イベントログ | リングバッファ設計、最新N件 | Phase 5で対応 | ✅ |
| 5.4 サジェスト再生 | 検知イベントを根拠に表示 | SG-1, SG-2 | ✅ |
| 7.3 Realtime（WebSocket） | /ws/events, /ws/state | BE-1 | ✅ |
| 8. IpcamScan | 既実装（スコープ外） | - | ✅ |

### 7.2 実装指示書との照合

| 指示項目 | 内容 | 修正計画対応 | 整合性 |
|---------|------|-------------|--------|
| コンセプト | なんとなく映しておく | SG-1（イベント→サジェスト） | ✅ |
| is21連携 | 192.168.3.240:9000 | AI-1 | ✅ |
| SSoT | カメラ台帳・イベントログ一意 | 既存実装維持 | ✅ |
| SOLID | コンポーネント分離 | Phase分離で対応 | ✅ |

### 7.3 追加修正項目

整合性チェックで判明した追加項目：

| # | 項目 | 修正内容 | 優先度 |
|---|------|----------|--------|
| UI-5 | ページタイトル | 「mobes AIcam control Tower (mAcT)」に変更 | P0 |
| UI-6 | ログ保持上限 | フロントエンドで最新500件のみ保持 | P1 |

### 7.4 アンアンビギュアス確認

以下の項目について、設計と修正計画の解釈に齟齬がないことを確認：

| 項目 | 設計書の記述 | 修正計画の解釈 | 齟齬 |
|------|-------------|---------------|------|
| サジェスト選択方式 | 「スコアリング」 | ユーザー指示により「イベント発生カメラをシンプルに表示」 | なし（過剰実装回避） |
| イベント継続 | 「sticky条件を設定可能」 | イベント継続中は表示維持 | なし |
| severity計算 | 「0..100など」 | person=3, vehicle=2, motion=1（0-3の4段階に簡略化） | なし |

**整合性チェック結果: 合格**

---

*作成日: 2025-12-31*
*フェーズ: 手順4（設計修正項目インデックス作成・整合性チェック）完了*
