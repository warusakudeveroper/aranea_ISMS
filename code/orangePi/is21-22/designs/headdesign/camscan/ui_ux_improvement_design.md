# UI/UX改善 設計ドキュメント

## 1. 概要

### 1.1 目的
スキャン機能のUI/UXを改善し、ユーザーにとってわかりやすく使いやすいインターフェースを実現する。

### 1.2 対象ファイル
- フロントエンド: `frontend/src/components/CameraScanModal.tsx`
- フロントエンド: `frontend/src/components/CameraGrid.tsx`

### 1.3 現状の問題点（Camscan_designers_review.md #8, #9より）
- スキャン結果の文言がわかりづらい
- スキャン開始がテキストリンクでわかりづらい

---

## 2. 設計

### 2.1 文言改善

| 現行 | 改善後 |
|------|--------|
| カメラ確認済 | ✓ RTSPカメラ |
| 認証済 | ✓ 認証OK |
| 全クレデンシャル不一致 - 設定確認が必要 | ⚠ 認証情報の確認が必要 |
| カメラ確認済み (RTSP応答あり) | カメラからRTSP応答を確認 |
| カテゴリ A: 登録済み | 📋 登録済みカメラ |
| カテゴリ B: 登録可能 | ➕ 登録可能カメラ |
| カテゴリ C: 認証必要 | 🔐 認証待ちカメラ |
| カテゴリ D: その他カメラ | 🌐 その他ネットワークデバイス |
| カテゴリ E: 非カメラ | ⛔ 非対応デバイス |
| カメラを全選択 | 認証可能カメラを全選択 |

### 2.2 スキャン開始ボタン

#### 現行（問題あり）
```
┌─────────────────────────────────────────────────┐
│                                                 │
│              ＋ カメラを追加                     │ ← テキストリンク
│                                                 │
└─────────────────────────────────────────────────┘
```

#### 改善後
```
┌─────────────────────────────────────────────────┐
│                                                 │
│    ┌─────────────────────────────────────┐     │
│    │  🔍 ネットワークをスキャン          │     │ ← 明確なボタン
│    │     新しいカメラを探す              │     │
│    └─────────────────────────────────────┘     │
│                                                 │
└─────────────────────────────────────────────────┘
```

### 2.3 ボタンスタイル

```css
.scan-start-button {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    padding: 16px 32px;
    border-radius: 12px;
    font-size: 16px;
    font-weight: 600;
    box-shadow: 0 4px 15px rgba(102, 126, 234, 0.4);
    transition: transform 0.2s, box-shadow 0.2s;
}

.scan-start-button:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px rgba(102, 126, 234, 0.6);
}
```

### 2.4 ステータスバッジ

```tsx
const StatusBadge: React.FC<{ status: string }> = ({ status }) => {
    const config = {
        registered: { icon: '📋', color: '#4CAF50', label: '登録済み' },
        authenticated: { icon: '✓', color: '#2196F3', label: '認証OK' },
        auth_required: { icon: '🔐', color: '#FF9800', label: '認証待ち' },
        unreachable: { icon: '⚠', color: '#f44336', label: '通信断' },
    };

    return (
        <span style={{ background: config[status].color }}>
            {config[status].icon} {config[status].label}
        </span>
    );
};
```

---

## 3. テスト計画

### 3.1 UIテスト
1. 全文言の表示確認
2. ボタンスタイルの確認
3. ホバーエフェクトの確認
4. モバイル表示の確認

---

## 4. MECE確認

- [x] 全改善対象の文言がリストアップされている
- [x] ボタンスタイルが明確に定義
- [x] アクセシビリティが考慮されている

---

**作成日**: 2026-01-07
**作成者**: Claude Code
**ステータス**: 設計完了・レビュー待ち
