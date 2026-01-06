/**
 * ChatInput - Auto-expanding text input with send button
 *
 * Features:
 * - Auto-resize textarea based on content (max 120px)
 * - Enter to send (Shift+Enter for newline)
 * - Send button
 */

import { useState, useRef, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Send } from "lucide-react"

interface ChatInputProps {
  onSend: (message: string) => void
  disabled?: boolean
  placeholder?: string
}

export function ChatInput({
  onSend,
  disabled = false,
  placeholder = "メッセージを入力...",
}: ChatInputProps) {
  const [message, setMessage] = useState("")
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  // Auto-resize textarea based on content
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto"
      textareaRef.current.style.height =
        Math.min(textareaRef.current.scrollHeight, 120) + "px"
    }
  }, [message])

  const handleSend = () => {
    if (message.trim() && !disabled) {
      onSend(message.trim())
      setMessage("")
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  return (
    <div className="flex items-end gap-2 p-2 border-t bg-background">
      <textarea
        ref={textareaRef}
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        disabled={disabled}
        rows={1}
        className="flex-1 resize-none rounded-md border px-3 py-2 text-sm
                   focus:outline-none focus:ring-2 focus:ring-primary
                   disabled:opacity-50 min-h-[36px] max-h-[120px]
                   bg-background"
      />
      <Button
        size="icon"
        onClick={handleSend}
        disabled={disabled || !message.trim()}
        className="flex-shrink-0"
      >
        <Send className="h-4 w-4" />
      </Button>
    </div>
  )
}
