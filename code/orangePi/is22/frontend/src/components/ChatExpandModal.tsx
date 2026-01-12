/**
 * ChatExpandModal - 拡大チャットモーダル
 *
 * AIアシスタントチャットを大きなモーダルで表示
 * メッセージ閲覧性を向上
 *
 * Features:
 * - ESCキーで閉じる
 * - モーダル外クリックで閉じる
 * - 開いた時に最新メッセージへ自動スクロール
 * - タイピングアニメーション (overflow: hidden + steps())
 * - メインページとスタイル統一
 */

import { useEffect, useRef, useState, useCallback } from "react"
import { X, MessageCircle, Send } from "lucide-react"
import { cn } from "@/lib/utils"
import aichatIcon from "@/assets/aichat_icon.svg"

interface ChatMessage {
  id: string
  role: "user" | "assistant" | "system"
  content: string
  timestamp: Date
  handled?: boolean
  actionMeta?: {
    type: "preset_change"
    cameraId: string
    presetId: string
  }
  // Auto-dismiss tracking
  dismissAt?: number  // timestamp when message should be hidden
  isHiding?: boolean  // true when slide-out animation is active
  // Typing animation tracking
  isTyping?: boolean  // true while typing animation is active
}

interface ChatExpandModalProps {
  isOpen: boolean
  onClose: () => void
  messages: ChatMessage[]
  onSend: (message: string) => void
  onPresetChange: (cameraId: string, presetId: string, messageId: string) => void
  onDismiss: (messageId: string) => void
}

// Typing animation component - uses useRef to avoid re-render loops
function TypedText({ text, onComplete }: { text: string; onComplete?: () => void }) {
  const [displayedText, setDisplayedText] = useState("")
  const [isComplete, setIsComplete] = useState(false)
  const textRef = useRef(text)
  const onCompleteRef = useRef(onComplete)

  // Update refs when props change (but don't trigger re-render)
  useEffect(() => {
    textRef.current = text
    onCompleteRef.current = onComplete
  })

  // Run typing animation only once on mount
  useEffect(() => {
    let currentIndex = 0
    const targetText = textRef.current

    const interval = setInterval(() => {
      if (currentIndex < targetText.length) {
        currentIndex++
        setDisplayedText(targetText.slice(0, currentIndex))
      } else {
        clearInterval(interval)
        setIsComplete(true)
        onCompleteRef.current?.()
      }
    }, 20) // 20ms per character for smooth typing effect

    return () => clearInterval(interval)
  }, []) // Empty dependency array - run only on mount

  return (
    <span>
      {displayedText}
      {!isComplete && (
        <span className="inline-block w-0.5 h-4 bg-current ml-0.5 animate-blink" />
      )}
    </span>
  )
}

export function ChatExpandModal({
  isOpen,
  onClose,
  messages,
  onSend,
  onPresetChange,
  onDismiss,
}: ChatExpandModalProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const modalRef = useRef<HTMLDivElement>(null)
  const [typingMessageId, setTypingMessageId] = useState<string | null>(null)

  // Scroll to bottom - helper function
  const scrollToBottom = useCallback(() => {
    if (scrollRef.current) {
      // Use requestAnimationFrame for smooth scroll after render
      requestAnimationFrame(() => {
        if (scrollRef.current) {
          scrollRef.current.scrollTop = scrollRef.current.scrollHeight
        }
      })
    }
  }, [])

  // Scroll to bottom when modal opens
  useEffect(() => {
    if (isOpen) {
      // Small delay to ensure modal is rendered
      setTimeout(() => {
        scrollToBottom()
      }, 50)
    }
  }, [isOpen, scrollToBottom])

  // Scroll to bottom when new messages arrive
  useEffect(() => {
    if (isOpen) {
      scrollToBottom()
    }
  }, [messages, isOpen, scrollToBottom])

  // Focus input when modal opens
  useEffect(() => {
    if (isOpen && inputRef.current) {
      setTimeout(() => inputRef.current?.focus(), 100)
    }
  }, [isOpen])

  // ESC key handler - global document listener
  useEffect(() => {
    if (!isOpen) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault()
        e.stopPropagation()
        onClose()
      }
    }

    // Use capture phase to ensure we catch the event first
    document.addEventListener("keydown", handleKeyDown, true)
    return () => document.removeEventListener("keydown", handleKeyDown, true)
  }, [isOpen, onClose])

  // Prevent body scroll when modal is open
  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = "hidden"
    } else {
      document.body.style.overflow = ""
    }
    return () => {
      document.body.style.overflow = ""
    }
  }, [isOpen])

  // Handle key press in input
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
    // Don't handle ESC here - let the global handler do it
  }

  // Handle send button click
  const handleSend = () => {
    const value = inputRef.current?.value.trim()
    if (value) {
      onSend(value)
      if (inputRef.current) inputRef.current.value = ""
      // Mark next assistant message for typing animation
      setTimeout(() => {
        const lastMsg = messages[messages.length - 1]
        if (lastMsg?.role === "assistant") {
          setTypingMessageId(lastMsg.id)
        }
      }, 100)
    }
  }

  // Handle backdrop click
  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    // Only close if clicking the backdrop itself, not the modal content
    if (e.target === e.currentTarget) {
      onClose()
    }
  }

  // Filter out messages that are hiding
  const visibleMessages = messages.filter(msg => !msg.isHiding)

  if (!isOpen) return null

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={handleBackdropClick}
      role="dialog"
      aria-modal="true"
      aria-labelledby="chat-modal-title"
    >
      {/* Modal Container */}
      <div
        ref={modalRef}
        className="w-[90vw] max-w-2xl h-[85vh] bg-white rounded-xl shadow-2xl flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b bg-gradient-to-r from-blue-500 to-blue-600 flex-shrink-0">
          <div className="flex items-center gap-3">
            <img
              src={aichatIcon}
              alt="AI"
              className="w-8 h-8 rounded-full bg-white p-0.5"
            />
            <div>
              <h2 id="chat-modal-title" className="text-white font-semibold">AIアシスタント</h2>
              <p className="text-white/70 text-xs">is22 mobes2.0連携</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-full hover:bg-white/20 transition-colors"
            aria-label="閉じる"
          >
            <X className="h-5 w-5 text-white" />
          </button>
        </div>

        {/* Messages Area */}
        <div
          className="flex-1 overflow-y-auto p-4 scroll-smooth"
          ref={scrollRef}
        >
          {visibleMessages.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-gray-400">
              <MessageCircle className="h-12 w-12 mb-3 opacity-50" />
              <p className="text-sm">質問を入力してください</p>
              <p className="text-xs mt-1">カメラ設定やシステムについてお答えします</p>
            </div>
          ) : (
            <div className="space-y-3">
              {visibleMessages.map((msg) => {
                // Only animate if explicitly marked with typingMessageId
                const shouldAnimate = msg.role === "assistant" &&
                  msg.id === typingMessageId &&
                  !msg.handled

                return (
                  <div
                    key={msg.id}
                    className={cn(
                      "flex animate-in slide-in-from-bottom-2 duration-300",
                      msg.role === "user" ? "justify-end" : "justify-start items-start gap-2"
                    )}
                  >
                    {/* AI Avatar - matching main page style */}
                    {msg.role !== "user" && (
                      <img
                        src={aichatIcon}
                        alt="AI"
                        className="w-8 h-8 flex-shrink-0 rounded-full bg-white p-0.5"
                      />
                    )}

                    {/* Message Bubble - iOS style matching main page exactly */}
                    <div
                      className={cn(
                        "max-w-[80%] text-xs px-3 py-2 rounded-2xl shadow-sm",
                        // User message: iOS blue background, white text, right tail
                        msg.role === "user" && "bg-[#007AFF] text-white rounded-tr-sm",
                        // System/Assistant message: light gray background, dark text, left tail
                        (msg.role === "system" || msg.role === "assistant") && "bg-[#F0F0F0] text-[#1A1A1A] rounded-tl-sm"
                      )}
                    >
                      {/* Message Content with optional typing animation */}
                      <div className="leading-relaxed whitespace-pre-wrap">
                        {shouldAnimate ? (
                          <TypedText
                            key={msg.id}
                            text={msg.content}
                            onComplete={() => {
                              if (msg.id === typingMessageId) {
                                setTypingMessageId(null)
                              }
                              scrollToBottom()
                            }}
                          />
                        ) : (
                          msg.content
                        )}
                      </div>

                      {/* Timestamp - matching main page style */}
                      <div className={cn(
                        "text-[9px] mt-1",
                        msg.role === "user" ? "text-right text-white/70" : "text-right text-gray-500"
                      )}>
                        {msg.timestamp.toLocaleTimeString("ja-JP", {
                          hour: "2-digit",
                          minute: "2-digit"
                        })}
                      </div>

                      {/* Action Buttons for System Messages - matching main page style */}
                      {msg.role === "system" && msg.actionMeta && !msg.handled && (
                        <div className="flex gap-2 mt-2 pt-2 border-t border-gray-300">
                          <button
                            onClick={() => onPresetChange(
                              msg.actionMeta!.cameraId,
                              msg.actionMeta!.presetId,
                              msg.id
                            )}
                            className="flex-1 px-3 py-1.5 rounded-lg text-[11px] font-medium transition-colors bg-[#007AFF] hover:bg-[#0066CC] text-white"
                          >
                            はい
                          </button>
                          <button
                            onClick={() => onDismiss(msg.id)}
                            className="flex-1 px-3 py-1.5 rounded-lg text-[11px] font-medium transition-colors bg-gray-300 hover:bg-gray-400 text-gray-700"
                          >
                            いいえ
                          </button>
                        </div>
                      )}

                      {/* Handled Indicator */}
                      {msg.role === "system" && msg.handled && (
                        <div className="mt-1.5 text-[10px] text-gray-500 flex items-center gap-1">
                          <span>✓</span>
                          <span>対応済み</span>
                        </div>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </div>

        {/* Input Area - matching main page feel but larger */}
        <div className="border-t p-3 bg-gray-50 flex-shrink-0">
          <div className="flex items-center gap-2">
            <input
              ref={inputRef}
              type="text"
              placeholder="メッセージを入力..."
              className="flex-1 px-4 py-2.5 rounded-full border border-gray-300 bg-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              onKeyDown={handleKeyDown}
            />
            <button
              onClick={handleSend}
              className="p-2.5 rounded-full bg-[#007AFF] hover:bg-[#0066CC] text-white transition-colors shadow-md flex-shrink-0"
              aria-label="送信"
            >
              <Send className="h-5 w-5" />
            </button>
          </div>
          <p className="text-[10px] text-gray-400 mt-1.5 text-center">
            ESCキーまたはモーダル外クリックで閉じる
          </p>
        </div>
      </div>

      {/* CSS for typing cursor animation */}
      <style>{`
        @keyframes blink {
          0%, 50% { opacity: 1; }
          51%, 100% { opacity: 0; }
        }
        .animate-blink {
          animation: blink 0.8s infinite;
        }
      `}</style>
    </div>
  )
}
