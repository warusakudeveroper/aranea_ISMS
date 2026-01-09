/**
 * useMediaQuery - メディアクエリ判定フック
 *
 * Issue #108: モバイルビューUI拡張
 *
 * 仕様:
 * - SSR安全（初期値false）
 * - リサイズ時リアルタイム更新
 * - ブレークポイント: 768px（モバイル/デスクトップ境界）
 */

import { useState, useEffect } from 'react'

/**
 * 任意のメディアクエリを判定するフック
 * @param query メディアクエリ文字列 (例: "(max-width: 768px)")
 * @returns マッチしているかどうか
 */
export function useMediaQuery(query: string): boolean {
  // SSR安全: 初期値false
  const [matches, setMatches] = useState(false)

  useEffect(() => {
    // クライアントサイドのみ
    const mediaQuery = window.matchMedia(query)
    setMatches(mediaQuery.matches)

    const handler = (event: MediaQueryListEvent) => {
      setMatches(event.matches)
    }

    // リサイズ時の変更を監視
    mediaQuery.addEventListener('change', handler)
    return () => mediaQuery.removeEventListener('change', handler)
  }, [query])

  return matches
}

/**
 * モバイル判定フック
 * ブレークポイント: 768px以下
 * @returns モバイルデバイスかどうか
 */
export function useIsMobile(): boolean {
  return useMediaQuery('(max-width: 768px)')
}

/**
 * タブレット判定フック
 * ブレークポイント: 769px - 1024px
 * @returns タブレットデバイスかどうか
 */
export function useIsTablet(): boolean {
  return useMediaQuery('(min-width: 769px) and (max-width: 1024px)')
}

/**
 * デスクトップ判定フック
 * ブレークポイント: 1025px以上
 * @returns デスクトップかどうか
 */
export function useIsDesktop(): boolean {
  return useMediaQuery('(min-width: 1025px)')
}
