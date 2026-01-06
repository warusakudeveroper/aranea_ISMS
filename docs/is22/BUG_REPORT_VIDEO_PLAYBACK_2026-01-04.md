# is22 動画再生障害 バグレポート

**報告日**: 2026-01-04 23:25
**報告者**: Claude Code
**ステータス**: 調査中

---

## 1. 観察された現象

### 1.1 SuggestPane（左側パネル）の動作

| 状況 | 動作 |
|------|------|
| 初期読み込み時 | **再生される** |
| 初期読み込み以降 | **再生されない** |
| イベント発生時の新規追加 | 未確認（イベント発生待ち） |

**観察詳細**:
- ページ読み込み直後、SuggestPaneに表示されたカメラ（Tapo C100 / 192.168.125.79）はライブ映像が再生されていた
- 「LIVE」バッジ、「human」検知バッジが正常に表示
- しばらくすると「イベントなし」表示に変わり、映像が消えた

### 1.2 LiveViewModal（カメラクリック時のモーダル）の動作

| 状況 | 動作 |
|------|------|
| カメラタイルクリック | モーダルは開く |
| 接続試行 | 「接続中...」表示 + カウントダウン（6秒から開始） |
| 10秒後 | **即座に「接続タイムアウト」で失敗** |
| サブストリームへのフォールバック | **発生しない** |

**観察詳細**:
- 「Toj 1F East」(192.168.126.24)をクリック
- カウントダウンタイマーは表示された（6秒→0秒）
- 10秒経過後、サブストリームへの切り替えなしに直接「接続タイムアウト」
- エラーメッセージ: 「カメラのストリームを再生できませんでした」「ネットワーク問題の可能性があります」
- フッターに「接続失敗」バッジ表示

### 1.3 カメラタイルの状態

多数のカメラタイルで以下のエラー表示:
- 「Internal e」
- 「最終: 更新なし」
- 「応答なし 最終: 23:21:44 (1分以上)」

**注目点**: SuggestPaneで再生されていたカメラ（Tapo C100 / 192.168.125.79）も、タイルではエラー表示だった。

### 1.4 コンソールエラー

```
[WS] Error: Event (385件以上)
```

大量のWebSocketエラーが記録されている。is22バックエンド(port 8080)へのWebSocket接続が失敗している模様。

---

## 2. 期待される動作 vs 実際の動作

### 2.1 Go2rtcPlayerのフォールバック設計

**設計仕様** (Go2rtcPlayer.tsx):
1. メインストリーム接続試行（10秒タイムアウト）
2. タイムアウト時、rtspUrlSubが存在すればサブストリームへフォールバック（20秒タイムアウト）
3. サブもタイムアウトまたはrtspUrlSubがなければ「接続タイムアウト」表示

**実際の動作**:
- カウントダウンは表示される（設計通り）
- しかし10秒後に即座に失敗状態へ移行
- サブストリームへのフォールバックは発生しない

### 2.2 rtsp_sub未設定の問題

go2rtc APIとis22 APIの確認結果:
```
カメラデータ:
- rtsp_main=SET (全17台)
- rtsp_sub=EMPTY (全17台)
```

LiveViewModal.tsx の実装:
```typescript
const rtspMain = camera.rtsp_main || ""
const rtspSub = camera.rtsp_sub || ""
// ...
rtspUrlSub={rtspMain ? rtspSub : undefined}
```

`rtsp_sub`が空文字列の場合、`rtspUrlSub`も空文字列となり、Go2rtcPlayerの`if (currentPhase === "main" && rtspUrlSub)`が偽となるため、サブストリームへのフォールバックは設計上発生しない。

**しかし**: これは「サブへフォールバックしない」の説明であり、「メインストリームがそもそも接続できない」理由の説明ではない。

---

## 3. 矛盾する観察事実

### 3.1 SuggestPaneは初期読み込み時に再生できた

- 同じGo2rtcPlayerコンポーネントを使用
- 同じgo2rtcサーバー(port 1984)に接続
- 同じカメラデータを使用

**疑問**: なぜSuggestPaneは再生できて、LiveViewModalは再生できないのか？

### 3.2 go2rtcにはストリームが登録されている

```
go2rtc streams: 17件登録済み
各ストリームにはRTSP URLが正しく設定されている
```

### 3.3 サーバー側WebSocketテストは成功

過去のテスト結果:
- サーバー内部(localhost)からgo2rtc WebSocket: **成功** (101 Switching Protocols)
- 外部(Mac)からgo2rtc WebSocket: **成功** (101 Switching Protocols)

**疑問**: サーバー側テストは成功するのに、ブラウザからは失敗する理由は？

---

## 4. 未解明の問題

1. **初期読み込み時のみSuggestPaneが再生される理由**
   - 何か初期化順序やタイミングの問題？
   - go2rtcのストリーム状態と関係がある？

2. **大量のWebSocketエラー `[WS] Error: Event` の原因**
   - is22バックエンド(port 8080)のWebSocket接続が失敗
   - これがGo2rtcPlayer(port 1984)にも影響している可能性？

3. **メインストリームがそもそも接続できない根本原因**
   - go2rtcにストリームは登録されている
   - サーバー側テストは成功する
   - ブラウザからのみ失敗する

4. **「以前は動いていた」との証言**
   - 環境は変わっていないとのこと
   - コードの何かが変わった？

---

## 5. 関連ファイル

| ファイル | 役割 |
|----------|------|
| `frontend/src/components/Go2rtcPlayer.tsx` | MSE動画プレイヤー、フォールバックロジック |
| `frontend/src/components/SuggestPane.tsx` | AIイベント表示パネル、Go2rtcPlayer使用 |
| `frontend/src/components/LiveViewModal.tsx` | カメラライブビューモーダル、Go2rtcPlayer使用 |
| `frontend/src/hooks/useWebSocket.ts` | is22バックエンドWebSocket接続 |
| `src/rtsp_manager/mod.rs` | RTSP多重接続制御（新規実装） |
| `src/snapshot_service/mod.rs` | スナップショット取得、RtspManager使用 |

---

## 6. 次のステップ（提案）

1. **SuggestPaneとLiveViewModalのGo2rtcPlayer呼び出し差分を詳細比較**
   - props
   - ライフサイクル
   - 呼び出しタイミング

2. **ブラウザのネットワークタブでWebSocket接続を詳細確認**
   - go2rtc (port 1984) への接続状態
   - is22 (port 8080) への接続状態
   - 接続確立の有無、エラーコード

3. **「初期読み込み時のみ動く」条件の特定**
   - 何が初期読み込み時と以降で異なるのか
   - タイミング？状態？リソース競合？

4. **git履歴で「動いていた時」のコードとの差分確認**

---

## 7. スクリーンショット記録

### 7.1 初期状態（SuggestPane再生中）
- 左側: Tapo C100 (192.168.125.79) ライブ再生中
- タイル: 複数のカメラで「Internal e」エラー
- 時刻: 23:21頃

### 7.2 モーダル接続中
- カメラ: Toj 1F East (192.168.126.24)
- 状態: 「接続中...」+ カウントダウン「6秒」
- 時刻: 23:22頃

### 7.3 モーダル接続失敗
- カメラ: Toj 1F East (192.168.126.24)
- 状態: 「接続タイムアウト」
- SuggestPane: 「イベントなし」に変化
- 時刻: 23:22頃

---

**重要な教訓**: 実装が正しいと思われても、観察された現象を否定してはならない。現象が存在する以上、どこかに問題がある。

---

## 8. 追加調査結果 (23:30追記)

### 8.1 is22バックエンドWebSocketの状態

サーバー側ログで確認:

```
WebSocket client connected connection_id=bc419cad-ce59-4a80-bca0-c3b8c9205137
WebSocket error connection_id=bc419cad-ce59-4a80-bca0-c3b8c9205137
  error=WebSocket protocol error: Connection reset without closing handshake
```

**発見**:
- WebSocket接続自体は**成功する**(101 Switching Protocols)
- しかし接続直後に**即座にリセット**される
- これが3秒間隔で繰り返され、大量のエラーに

### 8.2 ffmpeg RTSPエラー

```
Error opening input: Operation not permitted
Error opening input file rtsp://halecam:hale0083%40@192.168.125.79:554/stream1
```

**発見**:
- ffmpegがRTSP接続で`Operation not permitted`
- go2rtcとのRTSP接続競合の可能性

### 8.3 根本原因の仮説

1. **useWebSocket.ts問題**:
   - 接続→即リセット→3秒後再接続→ループ
   - exponential backoffなし
   - 無限リトライでリソース消費

2. **Go2rtcPlayer多重起動問題**:
   - SuggestPane、LiveViewModal、CameraTileなど複数箇所から呼び出し
   - 同一カメラへの多重接続でgo2rtcがビジー状態に

3. **RtspManager vs フロントエンド**:
   - RtspManagerはバックエンド(Rust)側でRTSP多重接続を防止
   - しかしフロントエンド側のGo2rtcPlayer多重起動は未制御

---

## 9. 修正提案

### 9.1 useWebSocket.ts

```typescript
// 現状: 固定3秒間隔で無限リトライ
reconnectTimeoutRef.current = setTimeout(connect, reconnectInterval)

// 修正案: exponential backoff + 最大リトライ回数
const backoffMs = Math.min(reconnectInterval * Math.pow(2, retryCount), 30000)
if (retryCount < MAX_RETRIES) {
  reconnectTimeoutRef.current = setTimeout(connect, backoffMs)
}
```

### 9.2 Go2rtcPlayer呼び出し制御

- 同一cameraIdへの多重Player起動を防止
- グローバルなストリーム管理コンテキスト導入検討
- または呼び出し元でのマウント状態管理強化

### 9.3 WebSocketリセット問題の調査

- なぜ接続直後にリセットされるのか？
- サーバー側のkeep-alive設定？
- ブラウザ側のタイムアウト設定？
