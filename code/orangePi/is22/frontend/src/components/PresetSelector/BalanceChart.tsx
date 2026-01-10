/**
 * BalanceChart Component
 *
 * Issue #107: プリセットバランス棒グラフ表示
 *
 * 4軸の棒グラフで表示:
 * - 検出感度
 * - 人物詳細
 * - 対象多様性
 * - 動体検知
 */

import React from 'react';

interface BalanceChartProps {
  detection_sensitivity: number;
  person_detail: number;
  object_variety: number;
  motion_sensitivity: number;
  isAnimating?: boolean;
}

interface BarProps {
  label: string;
  value: number;
  color: string;
  isAnimating?: boolean;
}

const Bar: React.FC<BarProps> = ({ label, value, color, isAnimating }) => {
  return (
    <div className="mb-3">
      <div className="flex justify-between text-xs mb-1">
        <span className="text-muted-foreground">{label}</span>
        <span className="text-muted-foreground/70">{value}%</span>
      </div>
      <div className="h-3 bg-muted rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full ${isAnimating ? 'transition-none' : 'transition-all duration-200'}`}
          style={{
            width: `${value}%`,
            backgroundColor: color,
          }}
        />
      </div>
    </div>
  );
};

export const BalanceChart: React.FC<BalanceChartProps> = ({
  detection_sensitivity,
  person_detail,
  object_variety,
  motion_sensitivity,
  isAnimating = false,
}) => {
  return (
    <div className="bg-muted/50 rounded-lg p-4">
      <h4 className="text-sm font-medium text-foreground mb-4">検出バランス</h4>

      <Bar
        label="検出感度"
        value={detection_sensitivity}
        color="#3B82F6" // blue-500
        isAnimating={isAnimating}
      />

      <Bar
        label="人物詳細"
        value={person_detail}
        color="#8B5CF6" // violet-500
        isAnimating={isAnimating}
      />

      <Bar
        label="対象多様性"
        value={object_variety}
        color="#10B981" // emerald-500
        isAnimating={isAnimating}
      />

      <Bar
        label="動体検知"
        value={motion_sensitivity}
        color="#F59E0B" // amber-500
        isAnimating={isAnimating}
      />
    </div>
  );
};

export default BalanceChart;
