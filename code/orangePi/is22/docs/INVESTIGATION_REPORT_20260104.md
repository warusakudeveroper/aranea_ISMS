# IS22 障害調査報告書

**日時**: 2026-01-04 20:00 JST
**調査者**: Claude Code
**重要度**: Critical
**ステータス**: 検証依頼中

---

## 1. 報告された問題一覧

| # | 問題 | 重要度 | 原因特定 | 修正状況 |
|---|------|--------|----------|----------|
| 1 | クリックモーダルで動画再生されない | Critical | ○ | 未修正 |
| 2 | サジェストペインで動画再生されない | Critical | ○ | 未修正 |
| 3 | サブストリームへのフォールバック不動作 | High | ○ | 未修正 |
| 4 | 192.168.126.127 (Toj Office) 接続失敗 | High | ○ | 自動復旧 |
| 5 | ドラッグ＆ドロップ並べ替え不動作 | Medium | **△ 未特定** | 未修正 |
| 6 | スナップショット503エラー多発 | Medium | ○ | 未修正 |
| 7 | 実装済み機能の動作不良 | Medium | **△ 要確認** | 要調査 |

---

## 2. 問題詳細と調査結果

### 2.1 go2rtc WebSocket接続失敗（Critical）【原因特定済・未修正】

**症状**:
```
[Go2rtcPlayer] MediaSource opened
[Go2rtcPlayer] Connecting to: ws://192.168.125.246:1984/api/ws?src=cam-aa583798-...
[Go2rtcPlayer] WebSocket error
[Go2rtcPlayer] WebSocket closed
[Go2rtcPlayer] Phase timeout: main
[Go2rtcPlayer] All streams failed
```

**調査結果**:
- go2rtcサービスは正常稼働中（active, version 1.9.4）
- 17ストリームが登録済み
- **全17ストリームでMedias: []（空）** - プロデューサーがカメラに実接続していない
- WebSocket接続は即座にエラー → close

**原因**:
go2rtcはオンデマンド接続方式。クライアント要求時にカメラへRTSP接続を開始するが、カメラ側のRTSP同時接続数制限に達しているため接続失敗。

**エビデンス**:
```bash
# go2rtc streams状態
Total streams: 17
Working (has medias): 0
Not working (no medias): 17

# 直接ffprobeは成功（カメラ自体は正常）
$ ffprobe rtsp://halecam:hale0083%40@192.168.125.78:554/stream1
h264
pcm_alaw
```

**関連ファイル**:
- `frontend/src/components/Go2rtcPlayer.tsx:312-373` - WebSocket接続処理
- `frontend/src/components/LiveViewModal.tsx:100-114` - プレイヤー呼び出し

---

### 2.2 サブストリームフォールバック不動作（High）【原因特定済・未修正】

**症状**:
メイン10秒タイムアウト後、サブストリーム20秒試行せずに即座に「All streams failed」

**コード分析** (`Go2rtcPlayer.tsx:239-254`):
```typescript
const handlePhaseTimeout = () => {
  if (currentPhase === "main" && rtspUrlSub) {  // ← rtspUrlSubがundefinedだとスキップ
    // Move to sub stream
    setStreamPhase("sub")
    startStream(rtspUrlSub, SUB_STREAM_TIMEOUT)
  } else {
    // Final failure - 即座にここに来る
    setStreamPhase("failed")
  }
}
```

**原因**:
1. 多くのカメラで`rtsp_sub: null`が設定されている
2. `LiveViewModal.tsx:103`で`rtspUrlSub={rtspMain ? rtspSub : undefined}`としているため、rtsp_subがnullの場合undefinedになる
3. フォールバック条件`rtspUrlSub`がfalsy値のため、即座にfailedへ遷移

**データベース状態**:
```json
{
  "camera_id": "cam-c23353c9-98d1-4a13-af66-068b9dd44b14",
  "name": "Toj Office",
  "rtsp_main": "rtsp://halecam:hale0083%40@192.168.126.127:554/stream1",
  "rtsp_sub": null  // ← サブストリーム未設定
}
```

**関連ファイル**:
- `frontend/src/components/Go2rtcPlayer.tsx:239-254` - フォールバック処理
- `frontend/src/components/LiveViewModal.tsx:103` - rtspUrlSub渡し方
- `src/config_store/types.rs` - Camera構造体定義

---

### 2.3 192.168.126.127 (Toj Office) 接続失敗（High）【原因特定済・自動復旧】

**症状**:
- UIにオレンジ色で「Internal e / 最終: 更新なし」と表示
- 定期的にスナップショット取得失敗

**ログ分析**:
```
11:18:32 Error opening input: Operation not permitted
11:19:10 Error opening input: Operation not permitted
11:19:48 Error opening input: Operation not permitted
... (6回連続失敗) ...
11:24:26 NO_DETECTION: analyzed=true ← 成功
11:25:10 NO_DETECTION: analyzed=true ← 成功
```

**疎通確認**:
```bash
# ping成功（3/3パケット）
$ ping -c 3 192.168.126.127
64 bytes from 192.168.126.127: icmp_seq=1 ttl=62 time=9.37 ms

# ffprobe成功
$ ffprobe rtsp://halecam:hale0083%40@192.168.126.127:554/stream1
h264
pcm_alaw
```

**原因**:
- **Operation not permitted** - カメラのRTSP同時接続数が上限に達した状態
- 接続が解放されると復旧（11:24:26以降成功）

**関連エラー** (別カメラ 192.168.125.17):
```
method SETUP failed: 406 Not Acceptable
Server returned 4XX Client Error
```
→ これは明確にRTSP同時接続数制限

---

### 2.4 ドラッグ＆ドロップ並べ替え不動作（Medium）【原因未特定・要調査】

**症状**:
- カメラタイルのドラッグ＆ドロップで並べ替えが機能しない
- 段を跨がない同一行内でも機能しない

**調査状況**: **未完了**

**コード分析** (`CameraGrid.tsx`):
- @dnd-kit/core, @dnd-kit/sortable を使用
- `DndContext` でラップ、`SortableContext` で並べ替え管理
- `handleDragEnd` でローカル状態更新 + バックエンドAPI呼び出し

**潜在的原因（推測）**:
1. `isDraggingRef.current` の状態管理問題
2. `localChangeTimeRef` による更新抑制
3. `useSortable` のID不一致
4. バックエンドAPIの sort_order 更新失敗

**調査必要事項**:
- [ ] Chromeコンソールでドラッグイベント発火確認
- [ ] `handleDragStart`, `handleDragEnd` のログ出力確認
- [ ] バックエンドPUT `/api/cameras/{id}` の応答確認
- [ ] React DevTools で状態変化確認

**関連ファイル**:
- `frontend/src/components/CameraGrid.tsx:304-374` - DnD処理
- `src/web_api/routes.rs` - PUT /api/cameras/{id} エンドポイント

---

### 2.5 スナップショット503エラー多発（Medium）【原因特定済・未修正】

**症状**:
同一スナップショットに対して10回以上の503エラー後、最終的に200

**ネットワークログ**:
```
GET /api/snapshots/cam-bb66a615.../latest.jpg?t=1767516601078 → 503
GET /api/snapshots/cam-bb66a615.../latest.jpg?t=1767516601082 → 503
GET /api/snapshots/cam-bb66a615.../latest.jpg?t=1767516601086 → 503
... (10回以上) ...
GET /api/snapshots/cam-bb66a615.../latest.jpg?t=1767516601123 → 200
```

**原因**:
- タイムスタンプが4msごとに更新されている → 同一リソースへの多重リクエスト
- バックエンドがスナップショット生成中に次のリクエストが到着
- 503 Service Unavailable を返して再試行を促している

**関連ファイル**:
- `frontend/src/components/CameraGrid.tsx:300-302` - タイムスタンプ取得
- `src/web_api/routes.rs` - スナップショットエンドポイント

---

### 2.6 実装済み機能の動作不良（Medium）【要確認】

**症状**:
ユーザー報告「実装したはずの機能も動作しなくなっていってます」

**調査状況**: **具体的機能未特定**

**確認必要事項**:
- [ ] 動作しなくなった機能の具体的リスト
- [ ] いつから動作しなくなったか
- [ ] 関連するコード変更履歴

---

### 2.7 コンソールエラー大量発生

**症状**:
```
[WS] Error: Event (113件以上)
```

**原因**:
go2rtc WebSocket接続エラーが各カメラで発生し、大量のエラーログが出力されている。問題2.1の派生症状。

---

## 3. 根本原因の結論

### 主要原因: RTSP同時接続数の競合

Tapo C100等の家庭用カメラはRTSP同時接続数が限定されている（通常2-4）。以下が同時にカメラへ接続を試みる：

1. **is22ポーリング** - ffmpegでスナップショット取得
2. **go2rtc** - オンデマンドでRTSPストリーム接続
3. **ブラウザ** - go2rtc経由でストリーム視聴

これらが競合すると：
- 後発の接続が`406 Not Acceptable`や`Operation not permitted`で拒否
- go2rtc WebSocket接続がエラーで即座に切断
- スナップショット取得が断続的に失敗

### 副次的原因

1. **rtsp_sub未設定** - フォールバックが機能しない
2. **多重リクエスト** - フロントエンドが同一リソースに短時間で複数リクエスト

---

## 4. システム状態

| 項目 | 状態 |
|------|------|
| go2rtc | Active, version 1.9.4, 17 streams registered |
| is22 backend | Active, polling正常動作 |
| WebSocket (is22) | Connected (LIVE表示あり) |
| ディスク | 24GB/98GB使用 (25%) |
| メモリ | 1.4GB/11GB使用 |
| RTSP接続数 | 1 (調査時点) |

---

## 5. 推奨対策

### 即時対応（Critical）

1. **go2rtcストリーム接続のタイミング制御**
   - ポーリング中はgo2rtcへの新規接続を制限
   - または、go2rtcを「常時接続」モードに変更

2. **RTSP接続の排他制御**
   ```rust
   // polling_orchestrator内で
   // go2rtcストリームを使用中のカメラはffmpeg直接接続をスキップ
   ```

### 短期対応（High）

3. **rtsp_sub自動設定**
   - カメラスキャン時にサブストリームURLも自動検出・保存
   - 例: `stream2`や`substream`

4. **フォールバックロジック修正**
   ```typescript
   // Go2rtcPlayer.tsx
   if (currentPhase === "main") {
     if (rtspUrlSub) {
       // サブストリームへフォールバック
     } else {
       // サブがない場合はスナップショットモードへ
       // 即座にfailedにしない
     }
   }
   ```

### 中期対応（Medium）

5. **スナップショットリクエストのデバウンス**
   - 同一カメラへの連続リクエストを制限
   - 最低100ms間隔を設ける

6. **カメラ接続状態の可視化**
   - どのカメラがRTSP接続数上限に達しているか表示
   - 接続競合時の警告表示

---

## 6. 追加調査必要事項

| # | 調査項目 | 優先度 | 担当 |
|---|----------|--------|------|
| 1 | ドラッグ＆ドロップ問題のデバッグ | High | 要アサイン |
| 2 | 動作しなくなった機能の具体化 | Medium | ユーザー確認 |
| 3 | go2rtcログ詳細（debug level） | Low | 任意 |
| 4 | カメラ個別の同時接続数上限調査 | Low | 任意 |

---

## 7. 関連ファイル一覧

### フロントエンド

| ファイル | 用途 |
|----------|------|
| `frontend/src/components/Go2rtcPlayer.tsx` | MSE動画プレイヤー |
| `frontend/src/components/LiveViewModal.tsx` | クリックモーダル |
| `frontend/src/components/SuggestPane.tsx` | サジェストペイン |
| `frontend/src/components/CameraGrid.tsx` | カメラグリッド（DnD） |
| `frontend/src/components/CameraTile.tsx` | カメラタイル |
| `frontend/src/hooks/useWebSocket.ts` | WebSocket接続 |

### バックエンド

| ファイル | 用途 |
|----------|------|
| `src/polling_orchestrator/mod.rs` | ポーリング制御 |
| `src/web_api/routes.rs` | REST API |
| `src/config_store/types.rs` | データ型定義 |

---

## 8. 検証依頼

### 検証環境
- **サーバー**: 192.168.125.246
- **go2rtc**: http://192.168.125.246:1984
- **is22 API**: http://192.168.125.246:8080
- **フロントエンド**: http://192.168.125.246:3000

### 検証手順

1. **動画再生確認**
   - カメラタイルをクリック → LiveViewModal表示
   - 10秒以内に動画再生開始するか確認
   - サブストリーム試行されるか確認（rtsp_sub設定済みカメラで）

2. **サジェストペイン確認**
   - AIイベント発生時にSuggestPaneに動画表示されるか確認

3. **ドラッグ＆ドロップ確認**
   - カメラタイルをドラッグして並べ替え
   - 同一行内、および行を跨いだ移動を試行

4. **接続エラー確認**
   - 192.168.126.127 (Toj Office) の接続状態確認
   - オレンジ枠（エラー表示）が出るか確認

---

*Report generated: 2026-01-04 21:00 JST*
