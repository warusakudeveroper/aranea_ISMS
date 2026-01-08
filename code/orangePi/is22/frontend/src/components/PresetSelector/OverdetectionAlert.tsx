/**
 * OverdetectionAlert Component
 *
 * Issue #107: 過剰検出警告表示
 *
 * 過剰検出の警告を色分けして表示:
 * - Critical (80%+): 赤
 * - Warning (70-80%): 黄
 */

import React from 'react';

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

interface OverdetectionAlertProps {
  issues: OverdetectionIssue[];
  onAdjustThreshold?: () => void;
  onExcludeTag?: (tag: string) => void;
  onChangePreset?: () => void;
}

const getTypeLabel = (type: string): string => {
  switch (type) {
    case 'tag_fixation':
      return 'タグ固定化';
    case 'static_object':
      return '固定物反応';
    case 'high_frequency':
      return '高頻度検出';
    case 'unknown_flood':
      return 'Unknown乱発';
    default:
      return type;
  }
};

const getSeverityIcon = (severity: string): string => {
  switch (severity) {
    case 'critical':
      return '🔴';
    case 'warning':
      return '🟡';
    default:
      return '🔵';
  }
};

const getSeverityColor = (severity: string): string => {
  switch (severity) {
    case 'critical':
      return 'border-red-500 bg-red-500/10';
    case 'warning':
      return 'border-amber-500 bg-amber-500/10';
    default:
      return 'border-blue-500 bg-blue-500/10';
  }
};

export const OverdetectionAlert: React.FC<OverdetectionAlertProps> = ({
  issues,
  onAdjustThreshold,
  onExcludeTag,
  onChangePreset,
}) => {
  if (issues.length === 0) {
    return null;
  }

  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <h4 className="text-sm font-medium text-amber-400 mb-3 flex items-center gap-2">
        <span>⚠️</span>
        <span>過剰検出の警告</span>
      </h4>

      <div className="space-y-2">
        {issues.map((issue, index) => (
          <div
            key={index}
            className={`border rounded-lg p-3 ${getSeverityColor(issue.severity)}`}
          >
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-2">
                <span>{getSeverityIcon(issue.severity)}</span>
                <span className="text-sm font-medium text-gray-200">
                  {getTypeLabel(issue.type)}
                </span>
              </div>
              {issue.rate > 0 && (
                <span className="text-xs text-gray-400">
                  {issue.rate.toFixed(1)}%
                </span>
              )}
            </div>

            <div className="mt-1 text-xs text-gray-300">
              {issue.tag && (
                <span className="mr-2">タグ: <code className="bg-gray-700 px-1 rounded">{issue.tag}</code></span>
              )}
              {issue.label && (
                <span className="mr-2">ラベル: <code className="bg-gray-700 px-1 rounded">{issue.label}</code></span>
              )}
              {issue.zone && (
                <span className="mr-2">ゾーン: <code className="bg-gray-700 px-1 rounded">{issue.zone}</code></span>
              )}
              <span>件数: {issue.count}</span>
            </div>

            <div className="mt-2 text-xs text-gray-400">
              {issue.suggestion}
            </div>

            <div className="mt-2 flex gap-2 flex-wrap">
              {onAdjustThreshold && (
                <button
                  onClick={onAdjustThreshold}
                  className="text-xs px-2 py-1 bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors"
                >
                  閾値調整
                </button>
              )}
              {issue.tag && onExcludeTag && (
                <button
                  onClick={() => onExcludeTag(issue.tag!)}
                  className="text-xs px-2 py-1 bg-gray-600 hover:bg-gray-500 text-white rounded transition-colors"
                >
                  タグ除外
                </button>
              )}
              {onChangePreset && (
                <button
                  onClick={onChangePreset}
                  className="text-xs px-2 py-1 bg-gray-600 hover:bg-gray-500 text-white rounded transition-colors"
                >
                  プリセット変更
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default OverdetectionAlert;
