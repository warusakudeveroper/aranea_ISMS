/**
 * PresetSelector Component
 *
 * Issue #107: プリセットグラフィカルUI
 *
 * プリセット選択UIの統合コンポーネント:
 * - プリセット選択ドロップダウン
 * - バランスチャート（4軸棒グラフ）
 * - 過剰検出警告
 * - タグ分布
 */

import React, { useEffect, useState, useCallback } from 'react';
import { BalanceChart } from './BalanceChart';
import { OverdetectionAlert } from './OverdetectionAlert';
import { TagDistribution } from './TagDistribution';
import { usePresetAnimation, PresetBalance } from '../../hooks/usePresetAnimation';

// API Response Types
interface PresetBalanceInfo {
  preset_id: string;
  name: string;
  balance: PresetBalance;
}

interface OverdetectionIssue {
  type: 'tag_fixation' | 'static_object' | 'high_frequency' | 'unknown_flood';
  tag?: string;
  label?: string;
  zone?: string;
  rate: number;
  count: number;
  severity: 'info' | 'warning' | 'critical';
  suggestion: string;
}

interface CameraOverdetection {
  camera_id: string;
  camera_name?: string;
  issues: OverdetectionIssue[];
}

interface TagTrend {
  tag: string;
  count: number;
  percentage: number;
  is_over_detected: boolean;
  trend: 'increasing' | 'decreasing' | 'stable';
}

interface PresetSelectorProps {
  cameraId: string;
  currentPresetId: string;
  onPresetChange: (presetId: string) => void;
  onOpenThresholdSettings?: () => void;
  apiBaseUrl?: string;
}

export const PresetSelector: React.FC<PresetSelectorProps> = ({
  cameraId,
  currentPresetId,
  onPresetChange,
  onOpenThresholdSettings,
  apiBaseUrl = '',
}) => {
  const [presets, setPresets] = useState<PresetBalanceInfo[]>([]);
  const [overdetection, setOverdetection] = useState<CameraOverdetection | null>(null);
  const [tagTrends, setTagTrends] = useState<TagTrend[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const { currentBalance, isAnimating, animateTo, setInstant } = usePresetAnimation();

  // Fetch preset balances
  const fetchPresets = useCallback(async () => {
    try {
      const response = await fetch(`${apiBaseUrl}/api/presets/balance`);
      const data = await response.json();
      if (data.ok) {
        setPresets(data.presets);

        // Set initial balance for current preset
        const currentPreset = data.presets.find((p: PresetBalanceInfo) => p.preset_id === currentPresetId);
        if (currentPreset) {
          setInstant(currentPreset.balance);
        }
      }
    } catch (err) {
      console.error('Failed to fetch presets:', err);
      setError('プリセット取得に失敗');
    }
  }, [apiBaseUrl, currentPresetId, setInstant]);

  // Fetch overdetection analysis
  const fetchOverdetection = useCallback(async () => {
    try {
      const response = await fetch(`${apiBaseUrl}/api/stats/overdetection?period=24h`);
      const data = await response.json();
      if (data.ok) {
        const cameraData = data.data.cameras.find((c: CameraOverdetection) => c.camera_id === cameraId);
        setOverdetection(cameraData || null);
      }
    } catch (err) {
      console.error('Failed to fetch overdetection:', err);
    }
  }, [apiBaseUrl, cameraId]);

  // Fetch tag trends
  const fetchTagTrends = useCallback(async () => {
    try {
      const response = await fetch(`${apiBaseUrl}/api/stats/tags/${cameraId}?period=24h`);
      const data = await response.json();
      if (data.ok) {
        setTagTrends(data.tags || []);
      }
    } catch (err) {
      console.error('Failed to fetch tag trends:', err);
    }
  }, [apiBaseUrl, cameraId]);

  // Initial fetch
  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      await Promise.all([
        fetchPresets(),
        fetchOverdetection(),
        fetchTagTrends(),
      ]);
      setLoading(false);
    };
    loadData();
  }, [fetchPresets, fetchOverdetection, fetchTagTrends]);

  // Handle preset change
  const handlePresetChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newPresetId = e.target.value;
    const newPreset = presets.find(p => p.preset_id === newPresetId);

    if (newPreset) {
      // Animate to new balance
      animateTo(newPreset.balance);

      // Notify parent
      onPresetChange(newPresetId);
    }
  };

  // Handle tag exclusion (future feature)
  const handleExcludeTag = (tag: string) => {
    console.log('Exclude tag:', tag);
    // TODO: Implement tag filter modal
  };

  if (loading) {
    return (
      <div className="space-y-4">
        <div className="animate-pulse bg-gray-700 h-10 rounded"></div>
        <div className="animate-pulse bg-gray-700 h-40 rounded"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="text-red-400 text-sm p-4 bg-red-500/10 rounded">
        {error}
      </div>
    );
  }

  const currentPreset = presets.find(p => p.preset_id === currentPresetId);

  return (
    <div className="space-y-4">
      {/* Preset Selector */}
      <div>
        <label className="block text-sm text-gray-300 mb-2">
          現在のプリセット
        </label>
        <select
          value={currentPresetId}
          onChange={handlePresetChange}
          className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-sm text-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          {presets.map(preset => (
            <option key={preset.preset_id} value={preset.preset_id}>
              {preset.name}
            </option>
          ))}
        </select>
      </div>

      {/* Balance Chart */}
      <BalanceChart
        detection_sensitivity={currentBalance.detection_sensitivity}
        person_detail={currentBalance.person_detail}
        object_variety={currentBalance.object_variety}
        motion_sensitivity={currentBalance.motion_sensitivity}
        isAnimating={isAnimating}
      />

      {/* Overdetection Alert */}
      {overdetection && overdetection.issues.length > 0 && (
        <OverdetectionAlert
          issues={overdetection.issues}
          onAdjustThreshold={onOpenThresholdSettings}
          onExcludeTag={handleExcludeTag}
          onChangePreset={() => {
            // Focus on preset selector
            const selectEl = document.querySelector('select');
            if (selectEl) selectEl.focus();
          }}
        />
      )}

      {/* Tag Distribution */}
      {tagTrends.length > 0 && (
        <TagDistribution tags={tagTrends} />
      )}
    </div>
  );
};

export default PresetSelector;
