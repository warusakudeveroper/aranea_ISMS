# SuggestPane go2rtc動画再生 詳細実装仕様書

## 1. 概要

AI検知イベント発生時に該当カメラの動画をgo2rtc経由でリアルタイム再生するサジェストビューの完全仕様。

### 1.1 技術スタック
- **動画配信**: go2rtc MSE over WebSocket
- **API**: `ws://{host}:1984/api/ws?src={cameraId}` + `{"type":"mse"}`
- **ストリーム登録**: `PUT /api/streams?name={cameraId}&src={rtspUrl}`

---

## 2. レイアウト仕様

### 2.1 ペイン構成
| 項目 | 値 |
|------|-----|
| 幅 | 画面横幅の30%（パーセンテージ、ピクセル上限なし） |
| 高さ | 画面高さからヘッダーを除いた全高 |
| 最大カメラ数 | **3台同時** |

### 2.2 各カメラスロットのサイズ
| 項目 | 値 |
|------|-----|
| 高さ | ペイン高さの **1/3**（縦3分割） |
| 幅 | 100%（ペイン幅いっぱい） |
| 角丸 | **なし**（角丸カード禁止） |
| スロット間隔 | **最小限**（0〜2px程度） |
| 上下配置 | **上寄せ**（新検知で押し下げ） |

### 2.3 動画表示
| 項目 | 値 |
|------|-----|
| object-fit | **cover**（見切れてOK） |
| 動画領域 | スロット全体（full-bleed） |

---

## 3. オーバーレイ要素

### 3.1 カメラ情報（下部グラデーション上）
```
┌────────────────────────────┐
│                            │
│       [ 動画映像 ]          │
│                            │
├────────────────────────────┤
│ グラデーション背景          │
│ カメラ名                   │
│ IP: xxx.xxx.xxx.xxx        │
└────────────────────────────┘
```

### 3.2 検出バッジ（左上）
| 項目 | 値 |
|------|-----|
| 位置 | 動画上の左上 |
| 内容 | primary_event（例: "human", "vehicle"） |
| スタイル | severity別カラーバッジ |

### 3.3 LIVEインジケータ（任意）
| 項目 | 値 |
|------|-----|
| 位置 | 右上 |
| スタイル | 赤背景 + 白文字 + パルスアニメーション |

---

## 4. onairtime管理

### 4.1 設定
| 項目 | 値 |
|------|-----|
| 設定場所 | SettingsModal |
| デフォルト値 | **180秒** |
| 保存先 | localStorage |
| キー | `onairtime_seconds` |

### 4.2 カウントダウンロジック
```typescript
interface OnAirCamera {
  cameraId: string
  lacisId: string
  rtspUrl: string
  startedAt: number      // Unixタイムスタンプ(ms)
  lastEventAt: number    // 最後のイベント検知時刻
  primaryEvent: string
  severity: number
}

// 終了判定
const shouldEnd = (cam: OnAirCamera, onairtime: number) => {
  const now = Date.now()
  const elapsed = (now - cam.lastEventAt) / 1000
  return elapsed >= onairtime
}
```

### 4.3 延長ロジック
- 再生中に同一カメラのイベント検知 → `lastEventAt` を更新
- カウントダウンリセット（延長）
- 順序変更：最新イベント順に並び替え

---

## 5. カメラ追加・削除フロー

### 5.1 新規カメラ挿入（イベント検知時）
```
1. イベント検知（WebSocket: event_log）
2. severity > 0 かつ rtsp_main が存在するか確認
3. 既に再生中の場合 → lastEventAt を更新して延長
4. 新規の場合:
   a. 現在3台再生中なら最古を削除（スライドアウト左）
   b. 新カメラを最上部にスライドイン（上から）
   c. 既存カメラは下にスライド移動
5. go2rtcストリーム登録 & MSE再生開始
```

### 5.2 onairtime終了時
```
1. setInterval(1000ms) で全OnAirCameraをチェック
2. 終了条件を満たしたカメラ:
   a. スライドアウト（左方向）アニメーション
   b. go2rtc WebSocket切断
   c. OnAirCamera配列から削除
3. 残りカメラは上方向にスライド移動
```

---

## 6. アニメーション仕様

### 6.1 挿入アニメーション
| 項目 | 値 |
|------|-----|
| 方向 | **上から下へスライドイン** |
| 時間 | 300ms |
| イージング | ease-out |
| 動画再生 | アニメーション中も継続 |

```css
@keyframes slideInFromTop {
  from {
    transform: translateY(-100%);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}
```

### 6.2 削除アニメーション（終了時）
| 項目 | 値 |
|------|-----|
| 方向 | **左へスライドアウト** |
| 時間 | 300ms |
| イージング | ease-in |

```css
@keyframes slideOutToLeft {
  from {
    transform: translateX(0);
    opacity: 1;
  }
  to {
    transform: translateX(-100%);
    opacity: 0;
  }
}
```

### 6.3 位置移動アニメーション
| 項目 | 値 |
|------|-----|
| トリガー | 新規挿入時の既存カメラ移動、延長による順序変更 |
| 時間 | 300ms |
| CSS | `transition: transform 300ms ease-out` |

---

## 7. カメラタイルとの連携（オンエアハイライト）

### 7.1 ハイライト対象
- SuggestPaneで再生中のカメラのタイル

### 7.2 ハイライトスタイル
| 項目 | 値 |
|------|-----|
| 枠線 | **黄色** (ring-2 ring-yellow-500) |
| オンエアアイコン | タイル上にアイコン表示（オプション） |

### 7.3 実装
```typescript
// App.tsx
const onAirCameraIds = onAirCameras.map(c => c.cameraId)

// CameraGrid.tsx
<CameraTile
  isOnAir={onAirCameraIds.includes(camera.camera_id)}
  ...
/>

// CameraTile.tsx
className={cn(
  isOnAir && "ring-2 ring-yellow-500"
)}
```

---

## 8. 独立動作要件

### 8.1 サジェスト再生の独立性
| シナリオ | 動作 |
|----------|------|
| タイルクリックモーダル表示中 | サジェスト再生は**継続** |
| 設定モーダル表示中 | サジェスト再生は**継続** |
| イベントログクリック中 | サジェスト再生は**継続** |

### 8.2 自動再生のみ
- **再生ボタンなし**（自動開始）
- **停止ボタンなし**（自動終了のみ）
- **一時停止なし**

---

## 9. 状態管理

### 9.1 必要な状態
```typescript
// stores/onAirStore.ts または App.tsx useState
interface OnAirState {
  cameras: OnAirCamera[]   // 最大3件
  onAirTimeSeconds: number // 設定値（デフォルト180）
}

// WebSocket event_log ハンドラ
const handleEventLog = (event: EventLogMessage) => {
  if (event.severity > 0) {
    addOrExtendOnAirCamera(event)
  }
}

// 1秒ごとのチェック
useEffect(() => {
  const interval = setInterval(() => {
    removeExpiredCameras()
  }, 1000)
  return () => clearInterval(interval)
}, [onAirTimeSeconds])
```

---

## 10. Go2rtcPlayer修正済み仕様

### 10.1 無限ループ修正
- `state` を依存配列から除去
- `stateRef` で状態をクロージャ内参照
- `isMountedRef` でアンマウント後の更新防止

### 10.2 インターフェース
```typescript
interface Go2rtcPlayerProps {
  cameraId: string       // ストリーム名
  rtspUrl: string        // RTSP URL
  autoPlay?: boolean     // デフォルト: true
  muted?: boolean        // デフォルト: true
  className?: string
  onError?: (error: string) => void
  onPlaying?: () => void
  onStopped?: () => void
}
```

---

## 11. SettingsModal追加項目

### 11.1 onairtime設定UI
```typescript
<div className="space-y-2">
  <Label>オンエア時間（秒）</Label>
  <Input
    type="number"
    min={30}
    max={600}
    value={onAirTimeSeconds}
    onChange={(e) => setOnAirTimeSeconds(Number(e.target.value))}
  />
  <p className="text-xs text-muted-foreground">
    最後のイベント検知から動画再生を終了するまでの時間
  </p>
</div>
```

---

## 12. 実装チェックリスト

### 12.1 Go2rtcPlayer
- [x] MSE over WebSocket実装
- [x] 無限ループバグ修正
- [x] エラー時のフォールバック表示

### 12.2 SuggestPane
- [x] 3カメラスロット表示
- [x] 縦1/3サイズ、上寄せ
- [x] 角丸なし
- [x] スライドインアニメーション（新規挿入）
- [x] スライドアウトアニメーション（終了時）
- [x] 位置交換アニメーション（延長時）
- [x] カメラ名・IP表示
- [x] 検出バッジ（動画上左上）
- [x] LIVEインジケータ

### 12.3 onairtime管理
- [x] SettingsModalに設定追加
- [x] localStorage保存・読み込み
- [x] カウントダウンロジック
- [x] 延長ロジック

### 12.4 オンエアハイライト
- [x] CameraTileに黄色枠追加
- [x] onAirCameraIds連携

### 12.5 状態管理
- [x] onAirCameras状態
- [x] WebSocketイベントハンドラ統合
- [x] 1秒ごとの終了チェック

---

## 13. 更新履歴

| 日付 | 内容 |
|------|------|
| 2025/01/04 | 詳細実装仕様書作成 |
