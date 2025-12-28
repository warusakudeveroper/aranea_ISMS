# 共通UIガイドライン

文書番号: COMMON_01
作成日: 2025-12-29
ステータス: Draft
参照: AraneaWebUI.h, 基本設計書 Section 12

---

## 1. araneaDevices共通UI原則

### 1.1 共通タブ構成
既存ESP32デバイス（is02, is04, is05, is10等）で使用されているAraneaWebUIパターンを踏襲：

| タブ | 用途 |
|-----|------|
| Status | デバイス状態・接続状況 |
| Network | WiFi設定・ネットワーク情報 |
| Cloud | MQTT接続・クラウド連携状態 |
| Tenant | テナント設定・認証情報 |
| System | システム情報・再起動・OTA |

### 1.2 is22固有タブ
| タブ | 用途 |
|-----|------|
| Cameras | カメラグリッド表示 |
| Events | イベント検索・履歴 |
| Admin | 管理設定（別ポート） |

---

## 2. デザインシステム

### 2.1 shadcn/ui採用
- React + TailwindCSS
- 一貫したコンポーネント
- ダークモード対応

### 2.2 カラーパレット
```css
:root {
  --background: 0 0% 100%;
  --foreground: 222.2 84% 4.9%;
  --card: 0 0% 100%;
  --card-foreground: 222.2 84% 4.9%;
  --primary: 222.2 47.4% 11.2%;
  --primary-foreground: 210 40% 98%;
  --secondary: 210 40% 96.1%;
  --secondary-foreground: 222.2 47.4% 11.2%;
  --destructive: 0 84.2% 60.2%;
  --destructive-foreground: 210 40% 98%;
  --muted: 210 40% 96.1%;
  --muted-foreground: 215.4 16.3% 46.9%;
  --accent: 210 40% 96.1%;
  --accent-foreground: 222.2 47.4% 11.2%;
  --border: 214.3 31.8% 91.4%;
}
```

### 2.3 severity色分け
| severity | 色 | 用途 |
|----------|-----|------|
| 0 | gray | 通常・検知なし |
| 1 | blue | 低優先度 |
| 2 | yellow | 注意 |
| 3 | red | 緊急 |

---

## 3. レイアウト

### 3.1 80/20分割
```
+------------------------------------------+
|                 Header                    |
+------------------------------------------+
|                    |                      |
|   Main Content     |    Side Panel        |
|   (80%)            |    (20%)             |
|                    |                      |
+------------------------------------------+
```

### 3.2 レスポンシブ対応
| ブレークポイント | 動作 |
|----------------|------|
| >= 1280px | 80/20分割 |
| 768-1279px | サイドパネルをドロワーに |
| < 768px | フルスクリーン + ドロワー |

### 3.3 カメラグリッド自動調整
| カメラ数 | レイアウト |
|---------|-----------|
| 1-4 | 2x2 |
| 5-9 | 3x3 |
| 10-16 | 4x4 |
| 17-25 | 5x5 |
| 26-30 | 5x6 |

---

## 4. コンポーネント

### 4.1 カメラタイル
```tsx
interface CameraTileProps {
  cameraId: string;
  displayName: string;
  locationLabel: string;
  latestFrame: {
    frameUuid: string;
    capturedAt: string;
    detected: boolean;
    primaryEvent: string;
    severity: number;
  };
  isOffline: boolean;
  onClick: () => void;
}
```

### 4.2 イベントカード
```tsx
interface EventCardProps {
  eventId: number;
  cameraId: string;
  startAt: string;
  endAt: string | null;
  primaryEvent: string;
  severityMax: number;
  tagsSummary: string[];
  bestFrameUuid: string;
  onClick: () => void;
}
```

### 4.3 モーダル
- 画像拡大モーダル
- ストリーム再生モーダル
- 設定編集モーダル

---

## 5. アイコン規則

### 5.1 Lucide Icons
| アイコン | 用途 |
|---------|------|
| Camera | カメラ |
| User | 人物検知 |
| Bug | 動物検知 |
| Car | 車両検知 |
| AlertTriangle | 異常・警告 |
| XCircle | オフライン・エラー |
| CheckCircle | 正常 |
| Clock | 時刻・履歴 |
| Search | 検索 |
| Settings | 設定 |

### 5.2 状態インジケータ
```tsx
<Badge variant={severity >= 2 ? "destructive" : "secondary"}>
  {primaryEvent}
</Badge>
```

---

## 6. インタラクション

### 6.1 カメラタイルクリック
1. モーダルで画像拡大
2. 「ライブ映像」ボタン → HLSストリーム開始
3. 「履歴」ボタン → イベント一覧へ

### 6.2 異常時自動モーダル
- severity >= 閾値で自動表示
- 設定秒（デフォルト20秒）で自動クローズ
- 同一カメラはクールダウン

### 6.3 キーボードショートカット
| キー | 動作 |
|-----|------|
| Escape | モーダル閉じる |
| ← → | モーダル内前後ナビゲーション |
| F | フルスクリーン |
| / | 検索フォーカス |

---

## 7. アクセシビリティ

### 7.1 ARIA属性
```tsx
<div role="grid" aria-label="カメラ一覧">
  <div role="row">
    <div role="gridcell" tabIndex={0} aria-label={cameraName}>
      ...
    </div>
  </div>
</div>
```

### 7.2 フォーカス管理
- タブキーでナビゲーション
- モーダル開閉時のフォーカストラップ
- スキップリンク

---

## 8. パフォーマンス

### 8.1 画像遅延読み込み
```tsx
<img
  src={thumbnailUrl}
  loading="lazy"
  alt={cameraName}
/>
```

### 8.2 仮想スクロール（イベント一覧）
- 大量イベント表示時
- react-window使用

### 8.3 WebSocket最適化
- 差分更新のみ送信
- 接続断時の自動再接続

---

## 9. 管理UI（別ポート）

### 9.1 認証
- Basic認証必須
- ポート8081

### 9.2 機能
| 機能 | 説明 |
|-----|------|
| カメラ管理 | 追加・編集・削除 |
| スキーマ管理 | タグ追加・更新 |
| 閾値設定 | 差分・通知閾値 |
| 保持ポリシー | TTL設定 |
| ログ表示 | システムログ |

---

## 10. テスト観点

### 10.1 レイアウト
- [ ] 各ブレークポイントで正常表示
- [ ] カメラグリッド自動調整

### 10.2 コンポーネント
- [ ] 全コンポーネントのレンダリング
- [ ] severity色分け

### 10.3 インタラクション
- [ ] クリック動作
- [ ] キーボードナビゲーション
- [ ] 自動モーダル

### 10.4 アクセシビリティ
- [ ] スクリーンリーダー対応
- [ ] キーボードのみ操作可能
