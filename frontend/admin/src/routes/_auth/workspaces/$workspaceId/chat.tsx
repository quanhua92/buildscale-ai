import { createFileRoute } from '@tanstack/react-router'
import { Chat, useChat, type ChatMessageItem } from '@buildscale/sdk'
import { z } from 'zod'

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
    <div className="h-[calc(100vh-var(--header-height))] w-full overflow-hidden">
      <Chat.Provider 
        workspaceId={workspaceId} 
        chatId={chatId}
        onChatCreated={handleChatCreated}
      >
        <ChatContent />
      </Chat.Provider>
    </div>
  )
}

function ChatContent() {
  const { messages, isStreaming } = useChat()

  return (
    <Chat containerClassName="max-w-4xl pt-4">
      <Chat.MessageList>
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full text-center space-y-4 py-20">
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
      
      <div className="pb-4 pt-2 shrink-0">
        <Chat.Input />
        <p className="text-[10px] text-center text-muted-foreground mt-3 uppercase tracking-widest font-bold opacity-50">
          Agentic Engine v0.1.0 • BuildScale.ai
        </p>
      </div>
    </Chat>
  )
}
