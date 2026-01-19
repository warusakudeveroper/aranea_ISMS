/**
 * Paraclate接続状態・設定管理フック
 *
 * DD09_IS22_WebUI.md準拠
 * バックエンド /api/paraclate/* APIとの連携を担当
 */

import { useState, useEffect, useCallback } from 'react';
import type {
  ParaclateStatus,
  ParaclateConfig,
  ParaclateConnectResponse,
  ParaclateConfigUpdateRequest,
} from '@/types/paraclate';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

export interface UseParaclateStatusResult {
  /** 接続状態 */
  status: ParaclateStatus | null;
  /** 設定 */
  config: ParaclateConfig | null;
  /** 読み込み中フラグ */
  isLoading: boolean;
  /** エラーメッセージ */
  error: string | null;
  /** 状態再取得 */
  refetch: () => Promise<void>;
  /** Paraclate APPに接続 */
  connect: (endpoint: string, fid: string) => Promise<ParaclateConnectResponse>;
  /** Paraclate APPから切断 */
  disconnect: () => Promise<boolean>;
  /** 設定更新 */
  updateConfig: (update: Partial<ParaclateConfigUpdateRequest>) => Promise<boolean>;
  /** 接続テスト実行 */
  testConnection: () => Promise<ParaclateConnectResponse>;
}

/**
 * Paraclate接続状態・設定管理フック
 *
 * @param tid テナントID
 * @param fid 施設ID
 * @returns UseParaclateStatusResult
 */
export function useParaclateStatus(
  tid: string,
  fid: string
): UseParaclateStatusResult {
  const [status, setStatus] = useState<ParaclateStatus | null>(null);
  const [config, setConfig] = useState<ParaclateConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  /**
   * 接続状態取得
   */
  const fetchStatus = useCallback(async () => {
    if (!tid || !fid) return;

    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/status?tid=${encodeURIComponent(tid)}&fid=${encodeURIComponent(fid)}`
      );
      if (!response.ok) {
        throw new Error(`Failed to fetch status: ${response.status}`);
      }
      const data = await response.json();
      setStatus(data);
      setError(null);
    } catch (err) {
      console.error('Failed to fetch Paraclate status:', err);
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  }, [tid, fid]);

  /**
   * 設定取得
   */
  const fetchConfig = useCallback(async () => {
    if (!tid || !fid) return;

    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/config?tid=${encodeURIComponent(tid)}&fid=${encodeURIComponent(fid)}`
      );
      if (!response.ok) return; // Config may not exist yet

      const data = await response.json();
      if (!data.error) {
        setConfig(data);
      }
    } catch {
      // Config fetch failure is not critical
    }
  }, [tid, fid]);

  /**
   * 状態・設定を再取得
   */
  const refetch = useCallback(async () => {
    setIsLoading(true);
    await Promise.all([fetchStatus(), fetchConfig()]);
    setIsLoading(false);
  }, [fetchStatus, fetchConfig]);

  // 初期取得
  useEffect(() => {
    if (tid && fid) {
      refetch();
    }
  }, [tid, fid, refetch]);

  // 30秒ごとにポーリング
  useEffect(() => {
    if (!tid || !fid) return;

    const interval = setInterval(fetchStatus, 30000);
    return () => clearInterval(interval);
  }, [tid, fid, fetchStatus]);

  /**
   * Paraclate APPに接続
   */
  const connect = useCallback(async (
    endpoint: string,
    targetFid: string
  ): Promise<ParaclateConnectResponse> => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/connect?tid=${encodeURIComponent(tid)}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ endpoint, fid: targetFid }),
        }
      );
      const data = await response.json();

      if (data.success) {
        await refetch();
      } else {
        setError(data.error || data.message || 'Connection failed');
      }

      return data;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Connection error';
      setError(errorMessage);
      return {
        success: false,
        connected: false,
        message: errorMessage,
      };
    }
  }, [tid, refetch]);

  /**
   * Paraclate APPから切断
   */
  const disconnect = useCallback(async (): Promise<boolean> => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/disconnect?tid=${encodeURIComponent(tid)}&fid=${encodeURIComponent(fid)}`,
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

  /**
   * 設定更新
   */
  const updateConfig = useCallback(async (
    update: Partial<ParaclateConfigUpdateRequest>
  ): Promise<boolean> => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/paraclate/config?tid=${encodeURIComponent(tid)}`,
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

  /**
   * 接続テスト（接続せずにエンドポイント疎通確認のみ）
   */
  const testConnection = useCallback(async (): Promise<ParaclateConnectResponse> => {
    if (!status?.endpoint) {
      return {
        success: false,
        connected: false,
        message: 'Endpoint not configured',
      };
    }

    // 接続テストは connect と同じAPIを使用
    return connect(status.endpoint, fid);
  }, [status?.endpoint, fid, connect]);

  return {
    status,
    config,
    isLoading,
    error,
    refetch,
    connect,
    disconnect,
    updateConfig,
    testConnection,
  };
}
