/**
 * ChatExpandModal - 拡大チャットモーダル
 *
 * AIアシスタントチャットを大きなモーダルで表示
 * メッセージ閲覧性を向上
 */

import { useEffect, useRef } from "react"
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
}

interface ChatExpandModalProps {
  isOpen: boolean
  onClose: () => void
  messages: ChatMessage[]
  onSend: (message: string) => void
  onPresetChange: (cameraId: string, presetId: string, messageId: string) => void
  onDismiss: (messageId: string) => void
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

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [messages])

  // Focus input when modal opens
  useEffect(() => {
    if (isOpen && inputRef.current) {
      setTimeout(() => inputRef.current?.focus(), 100)
    }
  }, [isOpen])

  // Handle key press
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      const value = inputRef.current?.value.trim()
      if (value) {
        onSend(value)
        if (inputRef.current) inputRef.current.value = ""
      }
    }
    if (e.key === "Escape") {
      onClose()
    }
  }

  // Handle send button click
  const handleSend = () => {
    const value = inputRef.current?.value.trim()
    if (value) {
      onSend(value)
      if (inputRef.current) inputRef.current.value = ""
    }
  }

  // Filter out messages that are hiding
  const visibleMessages = messages.filter(msg => !msg.isHiding)

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      {/* Modal Container */}
      <div className="w-[90vw] max-w-2xl h-[80vh] bg-white dark:bg-gray-900 rounded-xl shadow-2xl flex flex-col overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b bg-gradient-to-r from-blue-500 to-blue-600">
          <div className="flex items-center gap-3">
            <img
              src={aichatIcon}
              alt="AI"
              className="w-8 h-8 rounded-full bg-white p-0.5"
            />
            <div>
              <h2 className="text-white font-semibold">AIアシスタント</h2>
              <p className="text-white/70 text-xs">is22 mobes2.0連携</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-full hover:bg-white/20 transition-colors"
          >
            <X className="h-5 w-5 text-white" />
          </button>
        </div>

        {/* Messages Area */}
        <div className="flex-1 overflow-auto p-4 space-y-4" ref={scrollRef}>
          {visibleMessages.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-gray-400">
              <MessageCircle className="h-12 w-12 mb-3 opacity-50" />
              <p className="text-sm">質問を入力してください</p>
              <p className="text-xs mt-1">カメラ設定やシステムについてお答えします</p>
            </div>
          ) : (
            visibleMessages.map((msg) => (
              <div
                key={msg.id}
                className={cn(
                  "flex",
                  msg.role === "user" ? "justify-end" : "justify-start items-start gap-3"
                )}
              >
                {/* AI Avatar */}
                {msg.role !== "user" && (
                  <img
                    src={aichatIcon}
                    alt="AI"
                    className="w-10 h-10 flex-shrink-0 rounded-full bg-white p-0.5 shadow-md"
                  />
                )}

                {/* Message Bubble */}
                <div
                  className={cn(
                    "max-w-[70%] px-4 py-3 rounded-2xl shadow-md",
                    msg.role === "user"
                      ? "bg-blue-500 text-white rounded-tr-sm"
                      : "bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-100 rounded-tl-sm"
                  )}
                >
                  {/* Message Content */}
                  <div className="text-sm leading-relaxed whitespace-pre-wrap">
                    {msg.content}
                  </div>

                  {/* Timestamp */}
                  <div className={cn(
                    "text-[10px] mt-2",
                    msg.role === "user" ? "text-right text-white/60" : "text-right text-gray-400"
                  )}>
                    {msg.timestamp.toLocaleTimeString("ja-JP", {
                      hour: "2-digit",
                      minute: "2-digit"
                    })}
                  </div>

                  {/* Action Buttons for System Messages */}
                  {msg.role === "system" && msg.actionMeta && !msg.handled && (
                    <div className="flex gap-3 mt-3 pt-3 border-t border-gray-300 dark:border-gray-600">
                      <button
                        onClick={() => onPresetChange(
                          msg.actionMeta!.cameraId,
                          msg.actionMeta!.presetId,
                          msg.id
                        )}
                        className="flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-all bg-blue-500 hover:bg-blue-600 text-white shadow-sm hover:shadow"
                      >
                        はい、変更する
                      </button>
                      <button
                        onClick={() => onDismiss(msg.id)}
                        className="flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-all bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300"
                      >
                        いいえ
                      </button>
                    </div>
                  )}

                  {/* Handled Indicator */}
                  {msg.role === "system" && msg.handled && (
                    <div className="mt-2 text-xs text-gray-400 flex items-center gap-1">
                      <span>✓</span>
                      <span>対応済み</span>
                    </div>
                  )}
                </div>
              </div>
            ))
          )}
        </div>

        {/* Input Area */}
        <div className="border-t p-4 bg-gray-50 dark:bg-gray-800">
          <div className="flex items-center gap-3">
            <input
              ref={inputRef}
              type="text"
              placeholder="メッセージを入力..."
              className="flex-1 px-4 py-3 rounded-full border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-900 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              onKeyDown={handleKeyDown}
            />
            <button
              onClick={handleSend}
              className="p-3 rounded-full bg-blue-500 hover:bg-blue-600 text-white transition-colors shadow-md"
            >
              <Send className="h-5 w-5" />
            </button>
          </div>
          <p className="text-xs text-gray-400 mt-2 text-center">
            ESCキーで閉じる
          </p>
        </div>
      </div>
    </div>
  )
}
