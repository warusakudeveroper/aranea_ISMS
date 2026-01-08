/**
 * TagDistribution Component
 *
 * Issue #107: タグ分布棒グラフ表示
 *
 * 上位10件のタグを横棒グラフで表示
 * - 過剰検出タグは赤でハイライト
 * - 通常タグは青
 */

import React from 'react';

interface TagTrend {
  tag: string;
  count: number;
  percentage: number;
  is_over_detected: boolean;
  trend: 'increasing' | 'decreasing' | 'stable';
}

interface TagDistributionProps {
  tags: TagTrend[];
  title?: string;
}

const getTrendIcon = (trend: string): string => {
  switch (trend) {
    case 'increasing':
      return '↑';
    case 'decreasing':
      return '↓';
    default:
      return '→';
  }
};

const getTrendColor = (trend: string): string => {
  switch (trend) {
    case 'increasing':
      return 'text-red-400';
    case 'decreasing':
      return 'text-green-400';
    default:
      return 'text-gray-400';
  }
};

export const TagDistribution: React.FC<TagDistributionProps> = ({
  tags,
  title = 'タグ分布（上位10件）',
}) => {
  if (tags.length === 0) {
    return (
      <div className="bg-gray-800 rounded-lg p-4">
        <h4 className="text-sm font-medium text-gray-200 mb-3">{title}</h4>
        <p className="text-xs text-gray-500">データがありません</p>
      </div>
    );
  }

  // Find max count for scaling
  const maxCount = Math.max(...tags.map(t => t.count));

  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <h4 className="text-sm font-medium text-gray-200 mb-3">{title}</h4>

      <div className="space-y-2">
        {tags.slice(0, 10).map((tag, index) => {
          const barWidth = (tag.count / maxCount) * 100;
          const barColor = tag.is_over_detected ? '#EF4444' : '#3B82F6';

          return (
            <div key={index} className="group">
              <div className="flex items-center justify-between text-xs mb-1">
                <div className="flex items-center gap-2 min-w-0">
                  <span
                    className={`truncate ${tag.is_over_detected ? 'text-red-400 font-medium' : 'text-gray-300'}`}
                    title={tag.tag}
                  >
                    {tag.tag}
                  </span>
                  <span className={`${getTrendColor(tag.trend)}`}>
                    {getTrendIcon(tag.trend)}
                  </span>
                </div>
                <span className="text-gray-400 ml-2 whitespace-nowrap">
                  {tag.count.toLocaleString()}
                </span>
              </div>

              <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
                <div
                  className="h-full rounded-full transition-all duration-300"
                  style={{
                    width: `${barWidth}%`,
                    backgroundColor: barColor,
                  }}
                />
              </div>

              {/* Tooltip on hover */}
              <div className="hidden group-hover:block absolute z-10 bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs mt-1">
                <div>件数: {tag.count.toLocaleString()}</div>
                <div>割合: {tag.percentage.toFixed(1)}%</div>
                <div>傾向: {tag.trend === 'increasing' ? '増加' : tag.trend === 'decreasing' ? '減少' : '安定'}</div>
                {tag.is_over_detected && (
                  <div className="text-red-400">⚠️ 過剰検出の可能性</div>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {tags.length > 10 && (
        <p className="text-xs text-gray-500 mt-2">
          他 {tags.length - 10} 件のタグ
        </p>
      )}
    </div>
  );
};

export default TagDistribution;
