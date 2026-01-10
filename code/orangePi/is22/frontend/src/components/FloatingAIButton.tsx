/**
 * FloatingAIButton - モバイル用フローティングAIボタン
 *
 * Issue #108: モバイルビューUI拡張
 *
 * 仕様:
 * - 位置: 右下固定 (right-4 bottom-4)
 * - サイズ: 56x56px (タップ領域確保)
 * - スクロール追従
 * - 新着イベント時パルスアニメーション+バッジ
 * - ドロワー開時は×アイコン表示
 */

import { Bot, X } from 'lucide-react'
import { cn } from '@/lib/utils'

interface FloatingAIButtonProps {
  /** ボタンクリック時のコールバック */
  onClick: () => void
  /** 未読イベントがあるかどうか */
  hasNewEvents?: boolean
  /** ドロワーが開いているかどうか */
  isDrawerOpen?: boolean
  /** 追加のクラス名 */
  className?: string
}

export function FloatingAIButton({
  onClick,
  hasNewEvents = false,
  isDrawerOpen = false,
  className
}: FloatingAIButtonProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        // 位置・サイズ
        "fixed right-4 bottom-4 z-50",
        "w-14 h-14 rounded-full",
        // 色 - Issue #108: 白85%背景で視認性向上
        "bg-white/[0.85] text-primary",
        // ボーダー
        "border-2 border-primary/30",
        // シャドウ - 強化
        "shadow-xl",
        // レイアウト
        "flex items-center justify-center",
        // トランジション
        "transition-transform duration-200",
        // タップフィードバック
        "active:scale-95",
        // タッチハイライト無効化
        "touch-highlight",
        // パルスアニメーション（新着時、ドロワー閉時のみ）
        hasNewEvents && !isDrawerOpen && "animate-pulse-strong",
        className
      )}
      aria-label={isDrawerOpen ? "ドロワーを閉じる" : "AIイベントログを開く"}
    >
      {isDrawerOpen ? (
        <X className="h-6 w-6" />
      ) : (
        <div className="relative">
          <Bot className="h-6 w-6" />
          {/* 新着通知バッジ */}
          {hasNewEvents && (
            <span
              className={cn(
                "absolute -top-1 -right-1",
                "w-3 h-3 rounded-full",
                "bg-red-500",
                "ring-2 ring-primary"
              )}
              aria-hidden="true"
            />
          )}
        </div>
      )}
    </button>
  )
}
