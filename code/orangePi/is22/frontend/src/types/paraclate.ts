/**
 * Paraclate連携 型定義
 *
 * DD09_IS22_WebUI.md準拠
 * IS22 WebUI ↔ バックエンド /api/paraclate/* API連携用
 */

/**
 * Paraclate接続状態（/api/paraclate/status レスポンス）
 */
export interface ParaclateStatus {
  connected: boolean;
  connectionStatus: 'connected' | 'disconnected' | 'connecting' | 'error';
  endpoint: string | null;
  lastSyncAt: string | null;       // ISO8601
  lastError: string | null;
  queueStats: {
    pending: number;
    sending: number;
    failed: number;
    sentToday: number;
  };
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
 */
export interface ParaclateConnectResponse {
  success: boolean;
  connected: boolean;
  message: string;
  latencyMs?: number;
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

/**
 * 送信キューアイテム
 */
export interface ParaclateSendQueueItem {
  queueId: number;
  tid: string;
  fid: string;
  payloadType: 'summary' | 'grand_summary' | 'event' | 'emergency';
  status: 'pending' | 'sending' | 'sent' | 'failed';
  retryCount: number;
  createdAt: string;
  sentAt: string | null;
  lastError: string | null;
}

/**
 * キュー一覧レスポンス
 */
export interface ParaclateQueueListResponse {
  items: ParaclateSendQueueItem[];
  total: number;
  pendingCount: number;
  failedCount: number;
}
