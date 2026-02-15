/**
 * Chat Route - Chat page with active agent session display
 *
 * Displays a chat with:
 * - Header showing active session status
 * - Chat panel for messages
 * - Real-time session status updates
 *
 * ## Usage
 *
 * This route is accessed via /workspaces/:workspaceId/chats/:chatId
 */

import { createFileRoute } from '@tanstack/react-router'
import { useAgentSessions, AgentStatusIndicator, Chat, useChat, type ChatMessageItem, type AgentSession } from '@buildscale/sdk'
import { Clock } from 'lucide-react'

export const Route = createFileRoute('/workspaces/$workspaceId/chats/$chatId')({
  component: ChatPage,
})

function ChatPage() {
  const { chatId, workspaceId } = Route.useParams()
  const { getSessionsByChatId } = useAgentSessions()

  // Find active session for this chat
  const sessions = getSessionsByChatId(chatId)
  const activeSession = sessions.find((s) => s.status !== 'completed' && s.status !== 'error')

  return (
    <div className="flex-1 w-full relative">
      <Chat.Provider workspaceId={workspaceId} chatId={chatId}>
        <ChatContent activeSession={activeSession} />
      </Chat.Provider>
    </div>
  )
}

function ChatContent({ activeSession }: { activeSession?: AgentSession | undefined }) {
  const { messages, isStreaming } = useChat()

  const formatTimeAgo = (timestamp: string) => {
    const now = new Date()
    const time = new Date(timestamp)
    const diff = now.getTime() - time.getTime()

    const seconds = Math.floor(diff / 1000)
    const minutes = Math.floor(seconds / 60)
    const hours = Math.floor(minutes / 60)

    if (seconds < 60) return 'just now'
    if (minutes < 60) return `${minutes}m ago`
    if (hours < 24) return `${hours}h ago`
    return `${Math.floor(hours / 24)}d ago`
  }

  return (
    <>
      {/* Chat header with session status */}
      {activeSession && (
        <div className="border-b bg-muted/30 px-4 py-3 flex items-center gap-3">
          <AgentStatusIndicator status={activeSession.status} size="md" showLabel />
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium capitalize">
                {activeSession.agent_type}
              </span>
              <span className="text-xs text-muted-foreground">
                {activeSession.model}
              </span>
            </div>
            {activeSession.current_task && (
              <p className="text-xs text-muted-foreground truncate mt-0.5">
                {activeSession.current_task}
              </p>
            )}
          </div>
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <Clock className="h-3 w-3" />
            <span>{formatTimeAgo(activeSession.last_heartbeat)}</span>
          </div>
        </div>
      )}

      {/* Chat messages and input */}
      <Chat>
        <Chat.MessageList className="max-h-[calc(100vh-200px)] pb-32">
          {messages.length === 0 && (
            <div className="flex flex-col items-center justify-center text-center space-y-4 py-20">
              <div className="size-12 rounded-2xl bg-primary/10 flex items-center justify-center">
                <span className="text-2xl">✨</span>
              </div>
              <div className="space-y-2">
                <h2 className="text-xl font-semibold tracking-tight">Agent Chat</h2>
                <p className="text-muted-foreground max-w-sm">
                  Start a conversation with your agent.
                </p>
              </div>
            </div>
          )}

          {messages.map((message: ChatMessageItem) => (
            <Chat.Message key={message.id} role={message.role} message={message}>
              <Chat.Bubble />
            </Chat.Message>
          ))}

          {isStreaming && messages[messages.length - 1]?.status === "sending" && (
            <div className="flex items-center gap-2 text-muted-foreground animate-pulse px-2">
              <div className="size-2 bg-primary rounded-full" />
              <span className="text-xs font-medium">Agent is connecting...</span>
            </div>
          )}
        </Chat.MessageList>

        <div className="absolute bottom-0 left-0 right-0 bg-background border-t border-border">
          <div className="max-w-4xl mx-auto px-4 pt-2 pb-4">
            <Chat.Input />
          </div>
        </div>
      </Chat>
    </>
  )
}
