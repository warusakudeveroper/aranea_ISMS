# FIX-002 詳細設計: フロントエンド/バックエンド境界修正

**作成日**: 2026-01-05 12:00 JST
**親ドキュメント**: [FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md](./FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md)
**優先度**: P1（重要）
**難易度**: 低

---

# 絶対ルール（大原則の宣言）

**以下を必ず復唱してから作業を開始すること**

1. SSoTの大原則を遵守します
2. Solidの原則を意識し特に単一責任、開放閉鎖、置換原則、依存逆転に注意します
3. 必ずMECEを意識し、このコードはMECEであるか、この報告はMECEであるかを報告の中に加えます。
4. アンアンビギュアスな報告回答を義務化しアンアンビギュアスに到達できていない場合はアンアンビギュアスであると宣言できるまで明確な検証を行います。
5. 根拠なき自己解釈で情報に優劣を持たせる差別コード、レイシストコードは絶対の禁止事項です。情報の公平性原則
6. 実施手順と大原則に違反した場合は直ちに作業を止め再度現在位置を確認の上でリカバーしてから再度実装に戻ります
7. チェック、テストのない完了報告はただのやった振りのためこれは行いません
8. 依存関係と既存の実装を尊重し、確認し、車輪の再発明行為は絶対に行いません
9. 作業のフェーズ開始前とフェーズ終了時に必ずこの実施手順と大原則を全文省略なく復唱します
10. 必須参照の改変はゴールを動かすことにつながるため禁止です。
11. このドキュメントを全てのフェーズ前に鑑として再読し基本的な思想や概念的齟齬が存在しないかを確認します。
12. ルール無視はルールを無視した時点からやり直すこと。

---

# 1. 修正概要

## 1.1 問題の要約

SuggestPaneのビデオ再生がページリロード時に過去のイベントで起動される。これは設計意図に反する。

**設計意図**:
- SuggestPaneの再生トリガー = バックエンドのAI推論結果（WebSocketイベント）
- ページリロード = 過去ログの表示のみ、再生トリガーではない

**現状の動作**:
1. ページリロード時、`App.tsx`が`fetchLogs()`を呼び出し（lines 117-119）
2. 取得した`detectionLogs`から`currentEvent`を導出（lines 203-207）
3. SuggestPaneが`currentEvent`変更を検知して再生開始（SuggestPane lines 156-165）
4. 結果: **過去のイベントでビデオ再生が起動**

## 1.2 修正の意図

フロントエンド/バックエンド境界を明確化し、SuggestPaneの再生トリガーをWebSocketイベント（リアルタイム）に限定する。

---

# 2. 影響範囲

## 2.1 変更対象ファイル

| ファイル | 変更内容 |
|----------|----------|
| `frontend/src/App.tsx` | `currentEvent`導出ロジックの修正 |
| `frontend/src/components/SuggestPane.tsx` | インターフェース変更なし（Props変更なし） |

## 2.2 依存関係

| 依存元 | 依存先 | 影響 |
|--------|--------|------|
| `SuggestPane` | `currentEvent` prop | インターフェース変更なし |
| `EventLogPane` | `detectionLogs` | 影響なし（別目的） |

**結論**: 本修正はインターフェース変更を伴わないため、コンポーネント間の契約は維持される。

---

# 3. 現状コード分析

## 3.1 問題箇所1: App.tsx（初期fetch）

**ファイル**: `frontend/src/App.tsx` (Lines 117-119)

```typescript
// Fetch detection logs on mount (for SuggestPane currentEvent)
useEffect(() => {
  fetchLogs({ detected_only: true, severity_min: 1, limit: 100 })
}, [fetchLogs])
```

**問題**: マウント時にログを取得し、それが`currentEvent`の源泉となる。

## 3.2 問題箇所2: App.tsx（currentEvent導出）

**ファイル**: `frontend/src/App.tsx` (Lines 203-207)

```typescript
// Get the most recent detection for suggest pane (from store)
const currentEvent = useMemo(() => {
  if (detectionLogs.length === 0) return null
  // Find the most recent detection with severity > 0
  return detectionLogs.find(e => e.severity > 0) || null
}, [detectionLogs])
```

**問題**: `detectionLogs`全体から最新を取得するため、初期fetchのログも対象になる。

## 3.3 問題箇所3: SuggestPane.tsx（再生トリガー）

**ファイル**: `frontend/src/components/SuggestPane.tsx` (Lines 156-165)

```typescript
// Handle new detection events
useEffect(() => {
  if (!currentEvent) return
  if (currentEvent.severity === 0) return

  // Check if this is a new event
  if (lastEventIdRef.current === currentEvent.log_id) return
  lastEventIdRef.current = currentEvent.log_id

  addOrExtendCamera(currentEvent)
}, [currentEvent, addOrExtendCamera])
```

**問題**: `lastEventIdRef`は初期値`null`のため、初回の`currentEvent`は必ず「新しいイベント」と判定される。

## 3.4 問題のMECE分析

| カテゴリ | 問題 | 原因 |
|----------|------|------|
| **データ取得** | 初期fetchが再生トリガーを生む | `fetchLogs()`の結果が`currentEvent`に流れる |
| **データ変換** | 過去ログが再生対象になる | `useMemo`が過去ログも対象にしている |
| **再生判定** | 初回は必ず再生される | `lastEventIdRef`が初期値`null` |
| **イベント識別** | リアルタイムと過去ログの区別なし | 両者が同じ`currentEvent`経路を通る |

---

# 4. 修正設計

## 4.1 修正方針

**選択肢A**: currentEventをWebSocketイベント専用にする（推奨）
- `currentEvent`の源泉を**WebSocketイベントのみ**に限定
- `fetchLogs()`の結果は`EventLogPane`表示専用

**選択肢B**: SuggestPaneで初期イベントをスキップ
- `lastEventIdRef`の初期化タイミングを調整
- 複雑度が上がり、問題の根本解決にならない

**結論**: 選択肢Aを採用。境界が明確でMECEである。

## 4.2 修正後コード

### 4.2.1 App.tsx（State追加）

```typescript
// State for real-time event (WebSocket only)
const [realtimeEvent, setRealtimeEvent] = useState<DetectionLog | null>(null)
```

### 4.2.2 App.tsx（handleEventLog修正）

**現行コード（Lines 129-132）**:

```typescript
const handleEventLog = useCallback((_msg: EventLogMessage) => {
  // New detection event received - refetch to update currentEvent for SuggestPane
  fetchLogs({ detected_only: true, severity_min: 1, limit: 100 })
}, [fetchLogs])
```

**修正後コード**:

```typescript
const handleEventLog = useCallback((msg: EventLogMessage) => {
  // WebSocket event received - update realtime event for SuggestPane
  if (msg.severity > 0) {
    // Convert EventLogMessage to DetectionLog format for SuggestPane
    const realtimeLog: DetectionLog = {
      log_id: msg.log_id,
      lacis_id: msg.lacis_id,
      camera_id: msg.camera_id,
      primary_event: msg.primary_event,
      severity: msg.severity,
      detected_at: msg.detected_at,
      // Optional fields
      camera_name: msg.camera_name,
      snapshot_url: msg.snapshot_url,
    }
    setRealtimeEvent(realtimeLog)
  }

  // Also refetch logs for EventLogPane display
  fetchLogs({ detected_only: true, severity_min: 1, limit: 100 })
}, [fetchLogs])
```

### 4.2.3 App.tsx（currentEvent導出修正）

**現行コード（Lines 200-207）**:

```typescript
// Get the most recent detection for suggest pane (from store)
// Only returns events with severity > 0 (AI detections)
const currentEvent = useMemo(() => {
  if (detectionLogs.length === 0) return null
  // Find the most recent detection with severity > 0
  return detectionLogs.find(e => e.severity > 0) || null
}, [detectionLogs])
```

**修正後コード**:

```typescript
// currentEvent for SuggestPane is from realtime WebSocket events only
// NOT from fetched logs (which include historical data)
// This ensures page reload doesn't trigger video playback
const currentEvent = realtimeEvent
```

### 4.2.4 型定義確認

`EventLogMessage`に必要なフィールドが含まれているか確認が必要。
不足している場合はバックエンドのWebSocketメッセージを拡張する。

---

# 5. 型整合性確認

## 5.1 EventLogMessage（現行）

確認が必要なフィールド:
- `log_id`: 必須
- `lacis_id`: 必須
- `camera_id`: 必須
- `primary_event`: 必須
- `severity`: 必須
- `detected_at`: 必須
- `camera_name`: オプション（SuggestPane表示用）
- `snapshot_url`: オプション

## 5.2 DetectionLog（現行）

SuggestPaneが必要とするフィールド:
- `log_id`: ユニーク識別用
- `lacis_id`: カメラ検索用
- `primary_event`: バッジ表示用
- `severity`: バッジ色判定用

---

# 6. テスト計画

## 6.1 ユニットテスト

### テストケース1: ページリロード時に再生が開始されないこと

| 項目 | 内容 |
|------|------|
| **前提条件** | 過去に検知イベントが存在する |
| **操作** | ページをリロード（F5） |
| **期待結果** | SuggestPaneは「イベントなし」表示 |
| **確認項目** | ビデオプレイヤーが起動しないこと |

### テストケース2: WebSocketイベントで再生が開始されること

| 項目 | 内容 |
|------|------|
| **前提条件** | ページがロード済み |
| **操作** | バックエンドで検知イベント発生（severity > 0） |
| **期待結果** | SuggestPaneでビデオ再生開始 |
| **確認項目** | - カメラ名表示 - LIVEバッジ表示 - 検知バッジ表示 |

### テストケース3: EventLogPaneには過去ログが表示されること

| 項目 | 内容 |
|------|------|
| **前提条件** | 過去に検知イベントが存在する |
| **操作** | ページをリロード（F5） |
| **期待結果** | EventLogPaneに過去のイベントが表示される |
| **確認項目** | - ログ一覧表示 - severity > 0のイベントのみ |

### テストケース4: severity=0のイベントは再生されないこと

| 項目 | 内容 |
|------|------|
| **前提条件** | ページがロード済み |
| **操作** | バックエンドでseverity=0イベント発生（camera_lost等） |
| **期待結果** | SuggestPaneは変化しない |
| **確認項目** | ビデオプレイヤーが起動しないこと |

## 6.2 Chrome統合テスト

### テストケース5: 連続イベントでの動作

| 項目 | 内容 |
|------|------|
| **前提条件** | SuggestPaneで1台再生中 |
| **操作** | 別のカメラで検知イベント発生 |
| **期待結果** | 2台目のカメラが追加表示される |
| **確認項目** | - 最大3台まで表示 - 古いカメラがonairtime後に退出 |

---

# 7. 受け入れ基準

## 7.1 必須基準

| ID | 基準 | 検証方法 |
|----|------|----------|
| AC-002-1 | ページリロード時にSuggestPaneでビデオ再生が開始されないこと | テストケース1 |
| AC-002-2 | WebSocketイベントでSuggestPaneのビデオ再生が開始されること | テストケース2 |
| AC-002-3 | EventLogPaneは過去ログを正常に表示すること | テストケース3 |
| AC-002-4 | severity=0イベントは再生トリガーにならないこと | テストケース4 |

## 7.2 確認手順

1. Chromeでis22ダッシュボードを開く
2. 過去の検知イベントがEventLogPaneに表示されることを確認
3. SuggestPaneが「イベントなし」であることを確認
4. ページをリロード（F5）
5. SuggestPaneが「イベントなし」のままであることを確認（**AC-002-1**）
6. バックエンドで検知イベントを発生させる（実カメラまたはテストAPI）
7. SuggestPaneでビデオ再生が開始されることを確認（**AC-002-2**）
8. EventLogPaneに新しいイベントが追加されることを確認（**AC-002-3**）

---

# 8. ロールバック計画

## 8.1 ロールバック条件

- 修正後にWebSocketイベントでSuggestPaneが反応しない場合
- EventLogPaneの表示が壊れる場合

## 8.2 ロールバック手順

1. Gitで変更をリバート: `git revert <commit-hash>`
2. フロントエンド再ビルド: `cd frontend && npm run build`
3. 配信サーバー再起動

---

# 9. 実装チェックリスト

- [ ] 大原則を復唱した
- [ ] `EventLogMessage`の型定義を確認した
- [ ] `App.tsx`に`realtimeEvent`stateを追加した
- [ ] `handleEventLog`を修正した
- [ ] `currentEvent`導出ロジックを修正した
- [ ] TypeScriptコンパイルが通ることを確認した
- [ ] テストケース1〜4を実行した
- [ ] Chrome統合テストを実行した
- [ ] 大原則を再度復唱した

---

# 10. 備考

## 10.1 EventLogMessageの拡張について

現行の`EventLogMessage`に`camera_name`や`snapshot_url`が含まれていない場合、バックエンド側のWebSocketメッセージを拡張する必要がある。

ただし、SuggestPaneは`lacis_id`から`cameras`配列を検索してカメラ情報を取得しているため（`findCamera`関数）、最低限`log_id`, `lacis_id`, `primary_event`, `severity`があれば動作する。

## 10.2 設計意図の明確化

この修正により以下が明確になる:
- **SuggestPane** = リアルタイム再生専用（WebSocket駆動）
- **EventLogPane** = 履歴表示専用（API fetch駆動）
- **境界** = WebSocketイベントがトリガー、API fetchは表示用データ

---

# 更新履歴

| 日時 (JST) | 内容 |
|------------|------|
| 2026-01-05 12:00 | 初版作成 |

---

**作成者**: Claude Code
**ステータス**: 実装待ち
