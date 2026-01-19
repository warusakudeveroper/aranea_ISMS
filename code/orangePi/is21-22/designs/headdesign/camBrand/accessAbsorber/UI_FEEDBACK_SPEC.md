# Access Absorber UIフィードバック仕様

## 1. 設計思想

### 1.1 目的
ユーザーに「なんかよくわからんけど不安定」という印象を与えず、**原因があって再生できないことを明示**する。

### 1.2 原則
- **透明性**: 制限理由を明確に表示
- **即時性**: 制限検知時に即座にフィードバック
- **行動可能性**: 次のアクションを提示

---

## 2. クリックモーダル再生

### 2.1 正常時
通常どおりストリーム再生を開始。

### 2.2 制限到達時

#### 同時接続制限（Tapo等）

**表示内容:**
```
┌─────────────────────────────────────────────────────────┐
│  ⚠️ ストリーミング制限                                   │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  このカメラは並列再生数が1です。                         │
│  現在別のプロセスで再生中のためストリーミングできません。 │
│                                                         │
│  ┌───────────────────────────────────────┐              │
│  │ 現在の使用状況:                       │              │
│  │   • ポーリング処理 (残り約8秒)        │              │
│  └───────────────────────────────────────┘              │
│                                                         │
│  [ しばらく待ってから再試行 ]                            │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**React実装イメージ:**
```tsx
interface StreamLimitModalProps {
  cameraName: string;
  family: string;
  maxStreams: number;
  currentUsage: {
    purpose: string;
    remainingSeconds?: number;
  }[];
  onRetry: () => void;
  onClose: () => void;
}

const StreamLimitModal: React.FC<StreamLimitModalProps> = ({
  cameraName,
  family,
  maxStreams,
  currentUsage,
  onRetry,
  onClose,
}) => {
  return (
    <Modal isOpen onClose={onClose}>
      <ModalHeader>
        <WarningIcon />
        ストリーミング制限
      </ModalHeader>

      <ModalBody>
        <p>
          このカメラは並列再生数が<strong>{maxStreams}</strong>です。
        </p>
        <p>
          現在別のプロセスで再生中のためストリーミングできません。
        </p>

        {currentUsage.length > 0 && (
          <UsageBox>
            <h4>現在の使用状況:</h4>
            <ul>
              {currentUsage.map((usage, i) => (
                <li key={i}>
                  {usage.purpose}
                  {usage.remainingSeconds && ` (残り約${usage.remainingSeconds}秒)`}
                </li>
              ))}
            </ul>
          </UsageBox>
        )}
      </ModalBody>

      <ModalFooter>
        <Button variant="secondary" onClick={onClose}>
          閉じる
        </Button>
        <Button variant="primary" onClick={onRetry}>
          再試行
        </Button>
      </ModalFooter>
    </Modal>
  );
};
```

#### 再接続インターバル制限（JOOAN等）

**表示内容:**
```
┌─────────────────────────────────────────────────────────┐
│  ⏱️ 接続間隔制限                                        │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  NVTカメラは再接続に2秒の間隔が必要です。               │
│                                                         │
│  あと 1.5秒 後に自動再試行します...                     │
│  ████████████░░░░░░░░ 75%                               │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**動作:**
- カウントダウン表示
- 待機完了後に自動再試行
- ユーザーはキャンセル可能

---

## 3. サジェスト再生

### 3.1 正常時
サジェストカードにストリームをプレビュー表示。

### 3.2 制限到達時（Absorberが働いた場合）

**表示内容:**
```
┌─────────────────────────────────────────────────────────┐
│  🎥 Tam StorageArea                              [LIVE] │
├─────────────────────────────────────────────────────────┤
│                                                         │
│    ┌─────────────────────────────────────────────┐      │
│    │                                             │      │
│    │         📺 別デバイスで再生中               │      │
│    │                                             │      │
│    │     (Tapoは最大再生数1)                    │      │
│    │                                             │      │
│    │         残り 5秒 で終了                     │      │
│    │                                             │      │
│    └─────────────────────────────────────────────┘      │
│                                                         │
│  15:32:45 | 人物検知 2名                                │
└─────────────────────────────────────────────────────────┘
```

**動作:**
- サジェストカードは表示される
- ストリームプレビュー部分にプレースホルダーを表示
- 「別デバイスで再生中（{ブランド}は最大再生数{n}）」と表示
- **5秒でサジェスト再生終了**（通常のタイムアウトより短い）
- 検知情報（人物検知など）は表示

**React実装イメージ:**
```tsx
interface SuggestCardProps {
  camera: Camera;
  detection: Detection;
  streamStatus: StreamStatus;
}

const SuggestCard: React.FC<SuggestCardProps> = ({
  camera,
  detection,
  streamStatus,
}) => {
  const [countdown, setCountdown] = useState(5);

  useEffect(() => {
    if (streamStatus.blocked) {
      const timer = setInterval(() => {
        setCountdown(prev => {
          if (prev <= 1) {
            // 5秒で自動終了
            onSuggestEnd();
            return 0;
          }
          return prev - 1;
        });
      }, 1000);

      return () => clearInterval(timer);
    }
  }, [streamStatus.blocked]);

  return (
    <Card>
      <CardHeader>
        <CameraIcon />
        {camera.name}
        <LiveBadge>LIVE</LiveBadge>
      </CardHeader>

      <CardBody>
        {streamStatus.blocked ? (
          <StreamPlaceholder>
            <TvIcon />
            <BlockedMessage>別デバイスで再生中</BlockedMessage>
            <LimitInfo>
              ({camera.family}は最大再生数{streamStatus.maxStreams})
            </LimitInfo>
            <Countdown>残り {countdown}秒 で終了</Countdown>
          </StreamPlaceholder>
        ) : (
          <StreamPreview src={streamStatus.streamUrl} />
        )}
      </CardBody>

      <CardFooter>
        <Timestamp>{formatTime(detection.timestamp)}</Timestamp>
        <DetectionSummary>{detection.summary}</DetectionSummary>
      </CardFooter>
    </Card>
  );
};
```

---

## 4. カメラ一覧表示

### 4.1 接続状態インジケーター

カメラ一覧で各カメラの接続状態を表示。

```
┌─────────────────────────────────────────────────────────┐
│  カメラ一覧                                             │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ○ Entrance Camera          🟢 利用可能                 │
│  ○ Storage Area (Tapo)      🟡 使用中 (1/1)            │
│  ○ Parking Lot (VIGI)       🟢 利用可能 (1/3)          │
│  ○ Office (JOOAN)           🟠 クールダウン中 (1秒)     │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**ステータスアイコン:**
| アイコン | 状態 | 説明 |
|---------|------|------|
| 🟢 | 利用可能 | 接続可能 |
| 🟡 | 使用中 | 他で使用中だが上限未達 |
| 🔴 | 制限中 | 同時接続上限到達 |
| 🟠 | クールダウン | 再接続待機中 |
| ⚫ | オフライン | カメラ応答なし |

---

## 5. ツールチップ

カメラ名ホバー時に詳細情報を表示。

```
┌───────────────────────────────────────┐
│  Storage Area (Tapo C200)             │
├───────────────────────────────────────┤
│  ブランド: TP-link Tapo               │
│  最大同時接続: 1                       │
│  再接続間隔: 制限なし                  │
│                                       │
│  現在の状態:                          │
│    • ポーリング処理中                 │
│    • 次回スナップショット: 15秒後      │
├───────────────────────────────────────┤
│  💡 Tapoカメラは同時に1つの           │
│     ストリームのみ対応しています       │
└───────────────────────────────────────┘
```

---

## 6. 通知トースト

バックグラウンドでAbsorberが動作した場合の通知。

### 6.1 ポーリングが制限で遅延した場合

```
┌────────────────────────────────────────────────┐
│ ℹ️ Storage Area のポーリングを待機中           │
│    (Tapoは並列接続1のため順番待ち)             │
│                                    [閉じる]    │
└────────────────────────────────────────────────┘
```

### 6.2 プリエンプション発生時

```
┌────────────────────────────────────────────────┐
│ ⚠️ Storage Area のサジェスト再生を中断         │
│    (優先度の高いリクエストが入りました)        │
│                                    [閉じる]    │
└────────────────────────────────────────────────┘
```

---

## 7. API連携

### 7.1 ストリームリクエストAPI

```
POST /api/cameras/{camera_id}/stream/request
```

**リクエスト:**
```json
{
  "purpose": "click_modal",
  "stream_type": "main",
  "client_id": "browser-session-abc"
}
```

**成功レスポンス:**
```json
{
  "success": true,
  "stream_token": "token-xxx",
  "stream_url": "http://go2rtc:1984/api/stream.mp4?src=camera_xxx"
}
```

**制限レスポンス:**
```json
{
  "success": false,
  "error": {
    "code": "CONCURRENT_LIMIT_REACHED",
    "family": "tapo",
    "max_streams": 1,
    "current_streams": [
      {
        "purpose": "polling",
        "started_at": "2026-01-17T12:00:00Z",
        "estimated_remaining_sec": 8
      }
    ],
    "user_message": {
      "title": "ストリーミング制限",
      "message": "このカメラは並列再生数が1です。現在別のプロセスで再生中のためストリーミングできません。",
      "severity": "warning",
      "action_hint": "しばらく待ってから再度お試しください"
    }
  }
}
```

---

## 8. 多言語対応

### 8.1 メッセージキー

| キー | 日本語 | 英語 |
|------|--------|------|
| `absorber.concurrent_limit.title` | ストリーミング制限 | Streaming Limit |
| `absorber.concurrent_limit.message` | このカメラは並列再生数が{n}です。現在{purpose}で再生中のためストリーミングできません。 | This camera supports up to {n} concurrent streams. Currently in use by {purpose}. |
| `absorber.interval_limit.title` | 接続間隔制限 | Connection Interval Limit |
| `absorber.interval_limit.message` | {brand}カメラは再接続に{n}秒の間隔が必要です。 | {brand} cameras require {n} seconds between reconnections. |
| `absorber.suggest.blocked` | 別デバイスで再生中 | Playing on another device |
| `absorber.suggest.limit_info` | ({brand}は最大再生数{n}) | ({brand} max streams: {n}) |

---

## 9. 実装チェックリスト

- [ ] クリックモーダル制限表示コンポーネント
- [ ] サジェストカードプレースホルダー
- [ ] カメラ一覧ステータスインジケーター
- [ ] ツールチップ詳細表示
- [ ] 通知トースト
- [ ] API連携
- [ ] 多言語対応
