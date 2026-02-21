import { formatTime } from '@buildscale/sdk'
import type { ChatMessage } from '@buildscale/sdk'

interface MessagePreviewProps {
  message: ChatMessage
  isStreaming?: boolean
}

/**
 * Renders a preview of a chat message with role icon, timestamp, and content.
 * Content is truncated to 2 lines with ellipsis.
 */
export function MessagePreview({ message, isStreaming }: MessagePreviewProps) {
  const roleIcon = message.role === 'assistant' ? '🤖' : message.role === 'user' ? '👤' : '⚙️'
  const time = formatTime(message.created_at)

  // Filter out tool messages and system messages for cleaner preview
  if (message.role === 'tool' || message.role === 'system') {
    return null
  }

  // Skip empty content
  const content = message.content?.trim()
  if (!content) {
    return null
  }

  return (
    <div className="flex gap-2 py-1.5 px-2 bg-muted/30 rounded text-xs items-start">
      <span className="shrink-0 text-sm" role="img" aria-label={message.role}>
        {roleIcon}
      </span>
      <span className="text-muted-foreground shrink-0 tabular-nums">{time}</span>
      <p className="flex-1 line-clamp-2 break-all text-foreground">
        {content}
        {isStreaming && (
          <span className="inline-block w-1.5 h-3 bg-primary animate-pulse ml-0.5 align-middle" />
        )}
      </p>
    </div>
  )
}

interface MessagePreviewListProps {
  messages: ChatMessage[]
  isSessionRunning?: boolean
}

/**
 * Renders a list of message previews with the most recent first.
 */
export function MessagePreviewList({ messages, isSessionRunning }: MessagePreviewListProps) {
  // Filter valid messages (non-tool, non-system, non-empty)
  const validMessages = messages.filter(
    (msg) =>
      msg.role !== 'tool' &&
      msg.role !== 'system' &&
      msg.content?.trim()
  )

  if (validMessages.length === 0) {
    return (
      <div className="text-xs text-muted-foreground px-2 py-1.5 italic">
        No messages yet
      </div>
    )
  }

  // Show most recent first
  const reversedMessages = [...validMessages].reverse()

  return (
    <div className="space-y-1.5">
      {reversedMessages.map((msg, index) => (
        <MessagePreview
          key={msg.id}
          message={msg}
          isStreaming={
            isSessionRunning &&
            msg.role === 'assistant' &&
            index === 0 // Only show streaming cursor on the most recent assistant message
          }
        />
      ))}
    </div>
  )
}
