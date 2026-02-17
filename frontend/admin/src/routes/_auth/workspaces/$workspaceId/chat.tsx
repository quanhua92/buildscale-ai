import { createFileRoute } from '@tanstack/react-router'
import { Chat, useChat, type ChatMessageItem, AgentSessionsProvider, useAgentSessions, type SessionStatus, type ChatFile, MultiChatSSEManagerProvider } from '@buildscale/sdk'
import { z } from 'zod'
import { useState, useEffect, useMemo } from 'react'
import type { ChatTab } from '@buildscale/sdk'

const chatSearchSchema = z.object({
  chatId: z.string().optional(),
})

export const Route = createFileRoute('/_auth/workspaces/$workspaceId/chat')({
  component: ChatRoute,
  validateSearch: (search) => chatSearchSchema.parse(search),
})

function ChatRoute() {
  const { workspaceId } = Route.useParams()
  const { chatId } = Route.useSearch()
  const navigate = Route.useNavigate()

  const handleChatCreated = (newChatId: string) => {
    navigate({
      search: { chatId: newChatId },
      replace: true, // Replace to avoid cluttering history with the "new chat" state
    })
  }

  return (
    <div className="flex-1 w-full relative">
      <MultiChatSSEManagerProvider>
        <AgentSessionsProvider workspaceId={workspaceId}>
          <Chat.Provider
            workspaceId={workspaceId}
            initialChatId={chatId}
            onChatCreated={handleChatCreated}
          >
            <ChatContent workspaceId={workspaceId} />
          </Chat.Provider>
        </AgentSessionsProvider>
      </MultiChatSSEManagerProvider>
    </div>
  )
}

function ChatContent({ workspaceId }: { workspaceId: string }) {
  const {
    messages,
    isStreaming,
    clearMessages,
    model,
    setModel,
    mode,
    currentQuestion,
    submitAnswer,
    dismissQuestion,
    setMode,
    // Multi-chat state
    activeChatId,
    chatSessions,
    recentChats,
    switchToChat,
    refreshRecentChats,
  } = useChat()

  const { sessions } = useAgentSessions()
  const navigate = Route.useNavigate()
  const [isChangingMode, setIsChangingMode] = useState(false)
  const [initialChatIdLoaded, setInitialChatIdLoaded] = useState(false)

  // One-time URL sync for deep linking
  const urlChatId = Route.useSearch().chatId
  useEffect(() => {
    if (urlChatId && !initialChatIdLoaded) {
      // Set initial chat from URL (deep linking only)
      switchToChat(urlChatId)
      setInitialChatIdLoaded(true)
    }
  }, [urlChatId, initialChatIdLoaded, switchToChat])

  // Refresh recent chats periodically
  useEffect(() => {
    const interval = setInterval(() => {
      refreshRecentChats()
    }, 30000) // Every 30 seconds

    return () => clearInterval(interval)
  }, [refreshRecentChats])

  // Create a map of chat_id to session status
  const chatStatusMap = useMemo(() => {
    const map = new Map<string, SessionStatus>()
    sessions.forEach((session) => {
      // If a chat has multiple sessions, use the most recent one
      const existing = map.get(session.chat_id)
      if (!existing || new Date(session.updated_at) > new Date(existing)) {
        map.set(session.chat_id, session.status)
      }
    })
    return map
  }, [sessions])

  // Build tabs from chat sessions and recent chats
  const tabs = useMemo((): ChatTab[] => {
    const tabMap = new Map<string, ChatTab>()

    console.log('[Chat] Building tabs...', { chatSessions: chatSessions.size, recentChats: recentChats.length, activeChatId })

    // First, add all active chat sessions
    for (const session of chatSessions.values()) {
      const sessionStatus = chatStatusMap.get(session.chatId)
      tabMap.set(session.chatId, {
        chatId: session.chatId,
        name: getChatName(session.chatId, recentChats),
        status: session.isStreaming ? 'streaming' : 'idle',
        sessionStatus: sessionStatus || 'idle',
      })
    }

    // Then add recent chats that aren't already in tabs
    for (const chat of recentChats) {
      if (!tabMap.has(chat.chat_id)) {
        const sessionStatus = chatStatusMap.get(chat.chat_id)
        tabMap.set(chat.chat_id, {
          chatId: chat.chat_id,
          name: chat.name || 'Untitled Chat',
          status: 'idle',
          sessionStatus: sessionStatus || 'idle',
        })
      }
    }

    // Convert to array and sort by recent chat updated_at (most recent first)
    // Use stable sorting based on backend updated_at, not click/access time
    const result = Array.from(tabMap.values()).sort((a, b) => {
      // Sort by backend updated_at (most recent first)
      const recentA = recentChats.find((c: ChatFile) => c.chat_id === a.chatId)
      const recentB = recentChats.find((c: ChatFile) => c.chat_id === b.chatId)
      if (recentA && recentB) {
        return new Date(recentB.updated_at).getTime() - new Date(recentA.updated_at).getTime()
      }

      // Fallback: chat without recentChats data goes last
      if (recentA) return -1
      if (recentB) return 1

      return 0
    })

    return result
  }, [chatSessions, recentChats, chatStatusMap])

  const handleNewChat = () => {
    clearMessages()
    navigate({
      to: '.',
      search: {},
    })
  }

  const handleModeChange = async (newMode: 'plan' | 'build') => {
    if (newMode === mode) return // Already in this mode
    setIsChangingMode(true)
    try {
      await setMode(newMode)
    } finally {
      setIsChangingMode(false)
    }
  }

  // Get chat name from recent chats or default
  function getChatName(chatId: string, chats: typeof recentChats): string {
    const chat = chats.find((c: ChatFile) => c.chat_id === chatId)
    return chat?.name || 'Untitled Chat'
  }

  return (
    <>
      <Chat.Tabs
        tabs={tabs}
        activeTabId={activeChatId}
        onTabClick={switchToChat}
      />
      <Chat containerClassName="max-w-4xl flex flex-col h-full">
        <Chat.Header
          onNewChat={handleNewChat}
          model={model}
          onModelChange={setModel}
        >
          {/* Mode Toggle in header */}
          <Chat.ModeToggle
            currentMode={mode}
            onModeChange={handleModeChange}
            disabled={isChangingMode}
          />
          {/* Context Dialog */}
          {activeChatId && (
            <Chat.ContextDialog workspaceId={workspaceId} chatId={activeChatId} />
          )}
        </Chat.Header>

        {/* Question Bar (appears when AI asks a question) */}
        {currentQuestion && (
          <Chat.QuestionBar
            question={currentQuestion}
            onSubmit={submitAnswer}
            onDismiss={dismissQuestion}
          />
        )}

        <Chat.MessageList className="max-h-[calc(100vh-200px)] pb-32">
          {messages.length === 0 && (
            <div className="flex flex-col items-center justify-center text-center space-y-4 py-20">
              <div className="size-12 rounded-2xl bg-primary/10 flex items-center justify-center">
                <span className="text-2xl">✨</span>
              </div>
              <div className="space-y-2">
                <h2 className="text-xl font-semibold tracking-tight">Agentic Engine</h2>
                <p className="text-muted-foreground max-w-sm">
                  Ask anything about your workspace. I can read code, write files, and execute tools to help you build.
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
            <p className="text-[10px] text-center text-muted-foreground mt-3 uppercase tracking-widest font-bold opacity-50">
              Agentic Engine v0.1.0 • BuildScale.ai
            </p>
          </div>
        </div>
      </Chat>
    </>
  )
}
