# DD09: IS22 WebUI 詳細設計

作成日: 2026-01-11
対象: is22 フロントエンド（React/TypeScript）
ステータス: 詳細設計

## 1. 目的と概要

### 1.1 目的
IS22フロントエンドのAIアシスタントタブ・Paraclate連携UIを実装し、バックエンドAPI（DD03_ParaclateClient）と連携させる。

### 1.2 現状分析

#### 実装済み
| 項目 | 状態 | 場所 |
|------|------|------|
| AIアシスタントタブUI構造 | ✅ | SettingsModal.tsx:1736-2006 |
| サジェスト頻度スライダー | ✅ | SettingsModal.tsx:1752-1763 |
| LocalStorage保存/読込 | ✅ | aiSettingsStore.ts |
| EventLogPaneサジェスト制御 | ✅ | EventLogPane.tsx:657-680 |
| Paraclate設定UIプレースホルダー | ✅ | SettingsModal.tsx:1790-2004 |
| バックエンドAPI | ✅ | paraclate_routes.rs |

#### 未実装（本設計で対応）
| 項目 | 優先度 | 概要 |
|------|--------|------|
| Paraclate API連携 | P0 | フロントエンド→バックエンドAPI呼び出し |
| 接続状態リアルタイム表示 | P0 | /api/paraclate/status の反映 |
| 接続テストボタン | P1 | /api/paraclate/connect 呼び出し |
| 設定同期 | P1 | LocalStorage ↔ バックエンドDB同期 |
| プレースホルダー有効化 | P2 | is22登録完了後にUI有効化 |

### 1.3 スコープ

#### 対象
- SettingsModal.tsx AIアシスタントタブ
- Paraclate API連携フック
- 接続状態表示コンポーネント

#### スコープ外
- バックエンドAPI修正（DD03で対応済み）
- mobes2.0側実装
- EventLogPaneチャット機能拡張（別Issue）

## 2. 依存関係

### 2.1 バックエンドAPI（DD03_ParaclateClient準拠）

| エンドポイント | メソッド | 用途 |
|---------------|---------|------|
| /api/paraclate/status | GET | 接続状態取得 |
| /api/paraclate/connect | POST | 接続開始 |
| /api/paraclate/disconnect | POST | 切断 |
| /api/paraclate/config | GET | 設定取得 |
| /api/paraclate/config | PUT | 設定更新 |

### 2.2 内部依存

| モジュール | 用途 |
|-----------|------|
| aiSettingsStore.ts | AI設定LocalStorage管理 |
| SettingsModal.tsx | 設定モーダル本体 |
| api.ts | APIクライアント基盤 |

## 3. データ設計

### 3.1 型定義

```typescript
// src/types/paraclate.ts

/**
 * Paraclate接続状態（/api/paraclate/status レスポンス）
 */
export interface ParaclateStatus {
  connected: boolean;
  endpoint: string | null;
  lastCheckAt: string | null;      // ISO8601
  lastSuccessAt: string | null;    // ISO8601
  lastError: string | null;
  pendingQueueCount: number;
}

/**
 * Paraclate設定（/api/paraclate/config レスポンス）
 */
export interface ParaclateConfig {
  tid: string;
  fid: string;
  endpoint: string;
  reportIntervalMinutes: number;
  grandSummaryTimes: string[];     // ["09:00", "17:00", "21:00"]
  retentionDays: number;
  attunement: ParaclateAttunement;
  syncSourceTimestamp: string | null;
}

/**
 * AIアチューンメント設定
 */
export interface ParaclateAttunement {
  autoTuningEnabled: boolean;
  tuningFrequency: "daily" | "weekly" | "monthly";
  tuningAggressiveness: number;    // 0-100
}

/**
 * 接続リクエスト
 */
export interface ParaclateConnectRequest {
  endpoint: string;
  fid: string;
}

/**
 * 接続レスポンス
 *
 * バックエンド ConnectResponse (types.rs:372) 準拠
 */
export interface ParaclateConnectResponse {
  connected: boolean;
  endpoint: string;
  configId: number;
  error?: string;
}

/**
 * 設定更新リクエスト
 */
export interface ParaclateConfigUpdateRequest {
  fid: string;
  reportIntervalMinutes?: number;
  grandSummaryTimes?: string[];
  retentionDays?: number;
  attunement?: Partial<ParaclateAttunement>;
}
```

### 3.2 LocalStorage拡張

```typescript
// src/lib/aiSettingsStore.ts 拡張

export interface AIAssistantSettings {
  suggestionFrequency: number;      // 0-3 (OFF, 低, 中, 高)
  paraclate: {
    // UI状態（ローカルのみ）
    reportIntervalMinutes: number;
    grandSummaryTimes: string[];
    reportDetail: "concise" | "standard" | "detailed";
    instantAlertOnAnomaly: boolean;
    autoTuningEnabled: boolean;
    tuningFrequency: "daily" | "weekly" | "monthly";
    tuningAggressiveness: number;
    contextNote: string;
    // バックエンド同期状態
    lastSyncedAt: string | null;    // 追加: バックエンドとの最終同期時刻
    endpointConfigured: boolean;    // 追加: エンドポイント設定済みフラグ
  };
}
```

## 4. API連携設計

### 4.1 カスタムフック

```typescript
// src/hooks/useParaclateStatus.ts

import { useState, useEffect, useCallback } from 'react';
import { ParaclateStatus, ParaclateConfig } from '@/types/paraclate';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

interface UseParaclateStatusResult {
  status: ParaclateStatus | null;
  config: ParaclateConfig | null;
  isLoading: boolean;
  error: string | null;
  refetch: () => Promise<void>;
  connect: (endpoint: string, fid: string) => Promise<boolean>;
  disconnect: () => Promise<boolean>;
  updateConfig: (update: Partial<ParaclateConfig>) => Promise<boolean>;
}

export function useParaclateStatus(
  tid: string,
  fid: string
): UseParaclateStatusResult {
  const [status, setStatus] = useState<ParaclateStatus | null>(null);
  const [config, setConfig] = useState<ParaclateConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/status?tid=${tid}&fid=${fid}`
      );
      if (!response.ok) throw new Error('Failed to fetch status');
      const data = await response.json();
      setStatus(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  }, [tid, fid]);

  const fetchConfig = useCallback(async () => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/config?tid=${tid}&fid=${fid}`
      );
      if (!response.ok) return; // Config may not exist
      const data = await response.json();
      if (!data.error) {
        setConfig(data);
      }
    } catch {
      // Config fetch failure is not critical
    }
  }, [tid, fid]);

  const refetch = useCallback(async () => {
    setIsLoading(true);
    await Promise.all([fetchStatus(), fetchConfig()]);
    setIsLoading(false);
  }, [fetchStatus, fetchConfig]);

  // Initial fetch
  useEffect(() => {
    refetch();
  }, [refetch]);

  // Polling every 30 seconds
  useEffect(() => {
    const interval = setInterval(fetchStatus, 30000);
    return () => clearInterval(interval);
  }, [fetchStatus]);

  const connect = useCallback(async (endpoint: string, fid: string): Promise<boolean> => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/connect?tid=${tid}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ endpoint, fid }),
        }
      );
      const data = await response.json();
      if (data.success) {
        await refetch();
        return true;
      }
      setError(data.error || 'Connection failed');
      return false;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Connection error');
      return false;
    }
  }, [tid, refetch]);

  const disconnect = useCallback(async (): Promise<boolean> => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/disconnect?tid=${tid}&fid=${fid}`,
        { method: 'POST' }
      );
      const data = await response.json();
      if (data.success) {
        await refetch();
        return true;
      }
      return false;
    } catch {
      return false;
    }
  }, [tid, fid, refetch]);

  const updateConfig = useCallback(async (
    update: Partial<ParaclateConfig>
  ): Promise<boolean> => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/config?tid=${tid}`,
        {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ fid, ...update }),
        }
      );
      const data = await response.json();
      if (!data.error) {
        setConfig(data);
        return true;
      }
      return false;
    } catch {
      return false;
    }
  }, [tid, fid]);

  return {
    status,
    config,
    isLoading,
    error,
    refetch,
    connect,
    disconnect,
    updateConfig,
  };
}
```

### 4.2 AraneaRegistration連携

```typescript
// src/hooks/useAraneaRegistration.ts より取得

// 登録状態からtid/fidを取得
const { araneaRegistrationStatus } = useAraneaRegistration();
const tid = araneaRegistrationStatus?.tid || '';
const fid = araneaRegistrationStatus?.fid || '0000';
const isRegistered = araneaRegistrationStatus?.registered || false;
```

## 5. UI設計

### 5.1 AIアシスタントタブ構成

```
AIアシスタントタブ
├── Section 1: サジェスト設定（既存、変更なし）
│   └── サジェスト頻度スライダー
│
├── Section 2: Paraclate連携設定
│   ├── 接続状態バナー（NEW）
│   │   ├── 接続中: 緑バッジ + エンドポイント + 最終同期時刻
│   │   ├── 未接続: 灰色バッジ + 「接続テスト」ボタン
│   │   └── エラー: 赤バッジ + エラーメッセージ
│   │
│   ├── エンドポイント設定（NEW - 接続前のみ表示）
│   │   ├── Input: Paraclate APIエンドポイント
│   │   └── Button: 「接続テスト」
│   │
│   ├── 定時報告設定（isRegistered && connected で有効化）
│   │   ├── Input: 通常報告間隔（分）
│   │   └── MultiSelect: GrandSummary時刻
│   │
│   ├── 報告コンテキスト設定（同上）
│   │   ├── Textarea: 重視するポイント
│   │   ├── Select: 報告の詳細度
│   │   └── Checkbox: 異常検出時の即時通知
│   │
│   └── AIアチューンメント設定（同上）
│       ├── Checkbox: 自動チューニング提案
│       ├── Select: 提案頻度
│       └── Slider: チューニング積極性
│
└── Section 3: 接続ログ（NEW - 折りたたみ可能）
    └── 直近10件の接続/切断/エラーログ
```

### 5.2 接続状態バナー仕様

```tsx
// ParaclateConnectionBanner.tsx

interface Props {
  status: ParaclateStatus | null;
  isLoading: boolean;
  onConnect: () => void;
  onDisconnect: () => void;
}

// 状態別表示
// connected=true:
//   🟢 Paraclate API: 接続中
//   エンドポイント: https://...
//   最終同期: 2026-01-11 12:00:00
//   [切断] ボタン

// connected=false, endpoint設定済み:
//   🟡 Paraclate API: 切断中
//   [再接続] ボタン

// connected=false, endpoint未設定:
//   ⚪ Paraclate API: 未設定
//   [接続設定] ボタン → エンドポイント入力ダイアログ

// error存在:
//   🔴 Paraclate API: エラー
//   エラー: {lastError}
//   [再試行] ボタン
```

### 5.3 有効化条件

```typescript
// Paraclate設定UI有効化条件
const isParaclateEnabled = useMemo(() => {
  // 条件1: is22がAraneaDeviceGateに登録済み
  if (!araneaRegistrationStatus?.registered) return false;

  // 条件2: Paraclate APPに接続済み
  if (!paraclateStatus?.connected) return false;

  return true;
}, [araneaRegistrationStatus, paraclateStatus]);

// UIでの使用
<Input
  disabled={!isParaclateEnabled}
  className={!isParaclateEnabled ? 'opacity-60' : ''}
  ...
/>
```

## 6. 処理フロー

### 6.1 初期読み込み

```
SettingsModal Open
       │
       ▼
useAraneaRegistration() ─────► tid, fid取得
       │
       ▼
useParaclateStatus(tid, fid) ─► GET /api/paraclate/status
       │                         GET /api/paraclate/config
       ▼
接続状態に応じてUI表示
       │
       ├─[未登録]──► 「is22登録が必要です」メッセージ
       │
       ├─[登録済/未接続]──► エンドポイント入力UI表示
       │
       └─[登録済/接続済]──► 設定UI有効化
```

### 6.2 接続フロー

```
「接続テスト」ボタンクリック
       │
       ▼
エンドポイント入力バリデーション
       │
       ▼
POST /api/paraclate/connect ─────► バックエンド
       │                            │
       │                            ▼
       │                     Paraclate APP接続テスト
       │                            │
       ▼◄───────────────────────────┘
レスポンス受信
       │
       ├─[成功]──► 接続状態更新 → UI有効化 → Toast「接続成功」
       │
       └─[失敗]──► エラー表示 → Toast「接続失敗: {error}」
```

### 6.3 設定変更フロー

```
設定値変更（Input/Select/Slider）
       │
       ▼
LocalStorage保存（即座）
       │
       ▼
デバウンス（500ms）
       │
       ▼
PUT /api/paraclate/config ─────► バックエンド
       │                          │
       │                          ▼
       │                   DB更新 → mobes2.0同期
       │                          │
       ▼◄────────────────────────┘
レスポンス受信
       │
       ├─[成功]──► lastSyncedAt更新 → 同期完了表示
       │
       └─[失敗]──► エラー表示（LocalStorageは保持）
```

## 7. 実装タスク

### 7.1 Phase 1: 型定義・フック作成

| ID | タスク | ファイル |
|----|--------|---------|
| T9-1 | Paraclate型定義追加 | src/types/paraclate.ts（新規）|
| T9-2 | useParaclateStatus フック作成 | src/hooks/useParaclateStatus.ts（新規）|
| T9-3 | aiSettingsStore拡張 | src/lib/aiSettingsStore.ts |

### 7.2 Phase 2: UI実装

| ID | タスク | ファイル |
|----|--------|---------|
| T9-4 | ParaclateConnectionBanner作成 | src/components/ParaclateConnectionBanner.tsx（新規）|
| T9-5 | SettingsModal AIタブ修正 | src/components/SettingsModal.tsx |
| T9-6 | エンドポイント入力ダイアログ | src/components/ParaclateConnectDialog.tsx（新規）|

### 7.3 Phase 3: 統合・テスト

| ID | タスク | 内容 |
|----|--------|------|
| T9-7 | フック統合 | SettingsModalでuseParaclateStatus使用 |
| T9-8 | 有効化条件実装 | isParaclateEnabled制御 |
| T9-9 | ビルド確認 | npm run build |
| T9-10 | Chrome実機テスト | 192.168.125.246:3000 |

## 8. テスト計画

### 8.1 ユニットテスト

| ケース | 内容 | 期待結果 |
|--------|------|---------|
| useParaclateStatus初期化 | フック作成 | isLoading=true |
| fetchStatus成功 | API呼び出し成功 | status更新、error=null |
| fetchStatus失敗 | API呼び出し失敗 | error設定 |
| connect成功 | 接続成功 | connected=true |
| connect失敗 | 接続失敗 | error設定 |

### 8.2 E2Eテスト（Chrome実機）

| # | テストケース | 前提 | 手順 | 確認項目 |
|---|-------------|------|------|---------|
| E1 | 未登録状態 | is22未登録 | AIタブ開く | 「登録が必要」メッセージ |
| E2 | 登録済/未接続 | is22登録済 | AIタブ開く | エンドポイント入力UI表示 |
| E3 | 接続テスト | E2状態 | エンドポイント入力→接続 | 接続成功/失敗表示 |
| E4 | 設定変更 | 接続済 | 報告間隔変更 | LocalStorage更新、API呼び出し |
| E5 | 接続状態更新 | 接続済 | 30秒待機 | 自動ポーリング確認 |

### 8.3 確認コマンド

```bash
# ビルド
cd /Users/hideakikurata/Library/CloudStorage/Dropbox/Mac\ \(3\)/Documents/aranea_ISMS/code/orangePi/is22/frontend
npm run build

# デプロイ（is22サーバー）
scp -r dist/* mijeosadmin@192.168.125.246:/opt/is22/frontend/dist/

# Chrome確認
open http://192.168.125.246:3000
```

## 9. MECE/SOLID確認

### 9.1 MECE確認

- **網羅性**: 接続状態取得/接続/切断/設定取得/設定更新を全カバー
- **重複排除**: バックエンドAPI（DD03）とUI（DD09）を明確に分離
- **未カバー領域**: Pub/Sub通知のUI反映（別Issue）

### 9.2 SOLID確認

- **SRP**: useParaclateStatus（状態管理）、ParaclateConnectionBanner（表示）分離
- **OCP**: 新しい設定項目はParaclateConfig型拡張で対応
- **LSP**: ParaclateStatus/Configはバックエンドレスポンス型と一致
- **ISP**: 接続/設定/状態取得の各機能を独立メソッド化
- **DIP**: APIクライアントはfetch抽象に依存

## 10. The_golden_rules.md準拠確認

| # | ルール | 確認 |
|---|-------|------|
| 1 | SSoT遵守 | ✅ 設定SSoTはバックエンド、UIはキャッシュ |
| 2 | SOLID原則 | ✅ 上記9.2で確認 |
| 3 | MECE | ✅ 上記9.1で確認 |
| 4 | アンアンビギュアス | ✅ 型定義・API仕様を明確化 |
| 6 | 現場猫禁止 | ✅ 各処理でエラーハンドリング |
| 12 | 車輪の再発明禁止 | ✅ 既存aiSettingsStore/APIクライアント活用 |
| 15 | 設計ドキュメント必須 | ✅ 本詳細設計 |

---

**作成完了日**: 2026-01-11
**次ステップ**: INDEX.md更新 → 実装開始 → ビルド → Chrome実機テスト
