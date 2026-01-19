/**
 * Paraclate連携 型定義
 *
 * DD09_IS22_WebUI.md準拠
 * IS22 WebUI ↔ バックエンド /api/paraclate/* API連携用
 */

/**
 * Paraclate接続状態（/api/paraclate/status レスポンス）
 *
 * バックエンド StatusResponse (types.rs:383) 準拠
 */
export interface ParaclateStatus {
  connected: boolean;
  connectionStatus: 'connected' | 'disconnected' | 'error';  // バックエンドConnectionStatus準拠（'connecting'は存在しない）
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

/**
 * 送信キューアイテム
 *
 * バックエンド QueueItemResponse (types.rs:465) 準拠
 */
export interface ParaclateSendQueueItem {
  queueId: number;
  payloadType: 'summary' | 'grand_summary' | 'event' | 'emergency';
  referenceId: number | null;
  status: 'pending' | 'sending' | 'sent' | 'failed' | 'skipped';  // バックエンドQueueStatus準拠（'skipped'追加）
  retryCount: number;
  nextRetryAt: string | null;
  lastError: string | null;
  createdAt: string;
  sentAt: string | null;
}

/**
 * キュー統計
 *
 * バックエンド QueueStats (types.rs:398) 準拠
 */
export interface ParaclateQueueStats {
  pending: number;
  sending: number;
  failed: number;
  sentToday: number;
}

/**
 * キュー一覧レスポンス
 *
 * バックエンド QueueListResponse (types.rs:456) 準拠
 */
export interface ParaclateQueueListResponse {
  items: ParaclateSendQueueItem[];
  total: number;
  stats: ParaclateQueueStats;
}
