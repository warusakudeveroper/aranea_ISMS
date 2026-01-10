/**
 * MobileDrawer - モバイル用右ドロワーメニュー
 *
 * Issue #108: モバイルビューUI拡張
 *
 * 仕様:
 * - 右からスライドイン (300ms)
 * - 幅: 85%
 * - ESCキー/背景タップで閉じる
 * - スクロールロック
 * - ヘッダー付き（タイトル + 閉じるボタン）
 */

import { useEffect, useRef, type ReactNode } from 'react'
import { X } from 'lucide-react'
import { cn } from '@/lib/utils'

interface MobileDrawerProps {
  /** ドロワーが開いているかどうか */
  open: boolean
  /** 閉じる時のコールバック */
  onClose: () => void
  /** ドロワー内のコンテンツ */
  children: ReactNode
  /** ヘッダータイトル */
  title?: string
  /** ドロワーの幅（デフォルト85%） */
  width?: string
}

export function MobileDrawer({
  open,
  onClose,
  children,
  title = "AI Event Log",
  width = "85%"
}: MobileDrawerProps) {
  const drawerRef = useRef<HTMLDivElement>(null)

  // ESCキーで閉じる
  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && open) {
        onClose()
      }
    }
    document.addEventListener('keydown', handleEsc)
    return () => document.removeEventListener('keydown', handleEsc)
  }, [open, onClose])

  // スクロールロック
  useEffect(() => {
    if (open) {
      // 現在のスクロール位置を保存
      const scrollY = window.scrollY
      document.body.style.overflow = 'hidden'
      document.body.style.position = 'fixed'
      document.body.style.top = `-${scrollY}px`
      document.body.style.width = '100%'
    } else {
      // スクロール位置を復元
      const scrollY = document.body.style.top
      document.body.style.overflow = ''
      document.body.style.position = ''
      document.body.style.top = ''
      document.body.style.width = ''
      if (scrollY) {
        window.scrollTo(0, parseInt(scrollY || '0', 10) * -1)
      }
    }
    return () => {
      document.body.style.overflow = ''
      document.body.style.position = ''
      document.body.style.top = ''
      document.body.style.width = ''
    }
  }, [open])

  // 背景クリックで閉じる
  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose()
    }
  }

  return (
    <>
      {/* Backdrop */}
      <div
        className={cn(
          "fixed inset-0 z-40",
          "bg-black/50",
          "transition-opacity duration-300",
          open ? "opacity-100" : "opacity-0 pointer-events-none"
        )}
        onClick={handleBackdropClick}
        aria-hidden="true"
      />

      {/* Drawer - Issue #108: 白85%背景で視認性向上 */}
      <div
        ref={drawerRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={cn(
          "fixed top-0 right-0 z-50 h-full",
          "bg-white/[0.85] backdrop-blur-sm",
          "shadow-xl",
          "flex flex-col",
          "transition-transform duration-300 ease-out",
          open ? "translate-x-0" : "translate-x-full"
        )}
        style={{ width }}
      >
        {/* Header */}
        <div className="h-14 flex-shrink-0 flex items-center justify-between px-4 border-b bg-white/90">
          <span className="font-semibold text-foreground">{title}</span>
          <button
            onClick={onClose}
            className={cn(
              "p-2 rounded-full",
              "hover:bg-muted",
              "transition-colors",
              "touch-highlight"
            )}
            aria-label="閉じる"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-hidden">
          {children}
        </div>
      </div>
    </>
  )
}
