import * as React from "react"
import { useAuth } from "../../../context/AuthContext"
import {
  type CreateChatRequest,
  type CreateChatResponse,
  type PostChatMessageRequest,
  type PostChatMessageResponse,
} from "../../../api/types"

export type MessageRole = "user" | "assistant" | "system"

export type MessageStep =
  | { type: "thought"; text: string; agent_id?: string }
  | { type: "call"; tool: string; path: string; args?: any }
  | { type: "observation"; output: string }

export interface ChatMessageItem {
  id: string
  role: MessageRole
  content: string
  thinking?: string
  steps?: MessageStep[]
  status: "sending" | "streaming" | "completed" | "error"
  created_at: string
}

interface ChatContextValue {
  messages: ChatMessageItem[]
  isStreaming: boolean
  sendMessage: (content: string, attachments?: string[]) => Promise<void>
  stopGeneration: () => void
  clearMessages: () => void
  chatId?: string
}

const ChatContext = React.createContext<ChatContextValue | null>(null)

export function useChat() {
  const context = React.useContext(ChatContext)
  if (!context) {
    throw new Error("useChat must be used within a ChatProvider")
  }
  return context
}

interface ChatProviderProps {
  children: React.ReactNode
  workspaceId: string
  chatId?: string
  onChatCreated?: (chatId: string) => void
}

const generateId = () => {
  try {
    if (typeof crypto !== "undefined" && crypto.randomUUID) {
      return crypto.randomUUID()
    }
  } catch (e) {
    // Fallback for non-secure contexts
  }
  return `temp-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

export function ChatProvider({
  children,
  workspaceId,
  chatId: initialChatId,
  onChatCreated,
}: ChatProviderProps) {
  const { apiClient } = useAuth()
  const apiClientRef = React.useRef(apiClient)
  React.useEffect(() => {
    apiClientRef.current = apiClient
  }, [apiClient])

  const [messages, setMessages] = React.useState<ChatMessageItem[]>([])
  const [isStreaming, setIsStreaming] = React.useState(false)
  const [chatId, setChatId] = React.useState<string | undefined>(initialChatId)
  const abortControllerRef = React.useRef<AbortController | null>(null)
  const connectingRef = React.useRef<string | null>(null)
  
  // Use Refs for callbacks and stable values to keep connectToSse reference stable
  const onChatCreatedRef = React.useRef(onChatCreated)
  React.useEffect(() => {
    onChatCreatedRef.current = onChatCreated
  }, [onChatCreated])

  // Sync chatId with prop changes (e.g. from URL)
  React.useEffect(() => {
    setChatId(initialChatId)
  }, [initialChatId])

  const stopGeneration = React.useCallback(() => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort()
      abortControllerRef.current = null
    }
    connectingRef.current = null
    setIsStreaming(false)
  }, [])

  const connectToSse = React.useCallback(async (targetChatId: string) => {
    // Prevent double connection for the same chatId
    if (connectingRef.current === targetChatId) {
      return
    }

    // Abort previous connection if any
    if (abortControllerRef.current) {
      abortControllerRef.current.abort()
    }
    
    abortControllerRef.current = new AbortController()
    connectingRef.current = targetChatId
    setIsStreaming(true)

    try {
      console.info(`[Chat] Connecting to SSE for chat ${targetChatId}...`)
      
      const response = await apiClientRef.current.requestRaw(
        `/workspaces/${workspaceId}/chats/${targetChatId}/events`,
        {
          headers: {
            'Accept': 'text/event-stream',
          },
          signal: abortControllerRef.current.signal,
        }
      )

      if (!response.ok) {
        if (response.status === 401) {
          throw new Error('Unauthorized: Please login again')
        }
        throw new Error(`Failed to connect to event stream: ${response.statusText}`)
      }
      
      const reader = response.body?.getReader()
      if (!reader) throw new Error('No reader available')

      const decoder = new TextDecoder()
      let buffer = ''

      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        buffer += decoder.decode(value, { stream: true })
        const chunks = buffer.split('\n\n')
        buffer = chunks.pop() || ''

        for (const chunk of chunks) {
          const lines = chunk.split('\n')
          let eventType = 'chunk'
          let dataStr = ''

          for (const line of lines) {
            if (line.startsWith('event: ')) {
              eventType = line.slice(7).trim()
            } else if (line.startsWith('data: ')) {
              dataStr = line.slice(6).trim()
            }
          }

          if (!dataStr) continue
          
          try {
            const payload = JSON.parse(dataStr)
            const type = payload.type || eventType
            const eventData = payload.data || {}

            console.debug(`[Chat] SSE Event: ${type}`, eventData)

            if (type === "ping") continue

            setMessages((prev) => {
              const newMessages = [...prev]
              let lastMessage = newMessages[newMessages.length - 1]
              
              // If last message isn't assistant or we are starting a new assistant response turn
              if (!lastMessage || lastMessage.role !== "assistant" || lastMessage.status === "completed") {
                // Don't create a blank assistant message for purely structural events like session_init
                if (type === "session_init" || type === "file_updated") return prev

                lastMessage = {
                  id: generateId(),
                  role: "assistant",
                  content: "",
                  status: "streaming",
                  created_at: new Date().toISOString(),
                }
                newMessages.push(lastMessage)
              }

              const updatedMessage = { ...lastMessage }

              switch (type) {
                case "session_init":
                  if (eventData.chat_id && eventData.chat_id !== targetChatId) {
                    console.info(`[Chat] Session initialized with different ID: ${eventData.chat_id}`)
                    setChatId(eventData.chat_id)
                    onChatCreatedRef.current?.(eventData.chat_id)
                  }
                  return prev
                case "thought":
                  updatedMessage.thinking = (updatedMessage.thinking || "") + (eventData.text || "")
                  updatedMessage.status = "streaming"
                  break
                case "chunk":
                  updatedMessage.content = (updatedMessage.content || "") + (eventData.text || "")
                  updatedMessage.status = "streaming"
                  break
                case "call":
                  updatedMessage.steps = [
                    ...(updatedMessage.steps || []),
                    { type: "call", tool: eventData.tool, path: eventData.path, args: eventData.args },
                  ]
                  updatedMessage.status = "streaming"
                  break
                case "observation":
                  updatedMessage.steps = [
                    ...(updatedMessage.steps || []),
                    { type: "observation", output: eventData.output },
                  ]
                  updatedMessage.status = "streaming"
                  break
                case "done":
                  updatedMessage.status = "completed"
                  setIsStreaming(false)
                  // connectingRef.current = null // DO NOT clear this here, we want to stay "connected" to this ID
                  break
                case "error":
                  updatedMessage.status = "error"
                  updatedMessage.content += `\nError: ${eventData.message}`
                  setIsStreaming(false)
                  connectingRef.current = null
                  break
                case "file_updated":
                  console.info(`[Chat] File updated: ${eventData.path} v${eventData.version}`)
                  return prev
              }

              newMessages[newMessages.length - 1] = updatedMessage
              return newMessages
            })
          } catch (e) {
            console.error('[Chat] Failed to parse SSE event payload', e, dataStr)
          }
        }
      }
    } catch (error) {
      if ((error as Error).name === 'AbortError') {
        console.info('[Chat] SSE connection aborted')
        return
      }
      console.error('[Chat] SSE Error:', error)
      connectingRef.current = null
    }
  }, [workspaceId])

  // Automatically connect to SSE when chatId is available
  React.useEffect(() => {
    if (chatId) {
      connectToSse(chatId)
    } else {
      stopGeneration()
    }
  }, [chatId, connectToSse, stopGeneration])

  const sendMessage = React.useCallback(
    async (content: string, _attachments?: string[]) => {
      // 1. Add user message optimistically
      const userMessage: ChatMessageItem = {
        id: generateId(),
        role: "user",
        content,
        status: "completed",
        created_at: new Date().toISOString(),
      }

      setMessages((prev) => [...prev, userMessage])
      
      try {
        if (!chatId) {
          console.info('[Chat] Seeding new chat...')
          const response = await apiClientRef.current.post<CreateChatResponse>(
            `/workspaces/${workspaceId}/chats`,
            { goal: content } as CreateChatRequest
          )
          
          if (!response || !response.chat_id) {
            throw new Error('Invalid response from server during chat creation')
          }

          const newChatId = response.chat_id
          console.info(`[Chat] Chat seeded: ${newChatId}`)
          setChatId(newChatId)
          onChatCreatedRef.current?.(newChatId)
          // SSE will be connected by the useEffect
        } else {
          console.info(`[Chat] Sending message to ${chatId}...`)
          const response = await apiClientRef.current.post<PostChatMessageResponse>(
            `/workspaces/${workspaceId}/chats/${chatId}`,
            { content } as PostChatMessageRequest
          )

          if (!response || response.status !== "accepted") {
            throw new Error('Message not accepted by server')
          }

          // Ensure SSE is connected (should be already, but safety first)
          connectToSse(chatId)
        }
      } catch (error) {
        console.error('[Chat] Failed to send message', error)
        setMessages((prev) => {
          const newMessages = [...prev]
          const last = newMessages[newMessages.length - 1]
          if (last) last.status = "error"
          return newMessages
        })
        setIsStreaming(false)
      }
    },
    [workspaceId, chatId, connectToSse]
  )

  const clearMessages = React.useCallback(() => {
    setMessages([])
    stopGeneration()
  }, [stopGeneration])

  const value = React.useMemo(
    () => ({
      messages,
      isStreaming,
      sendMessage,
      stopGeneration,
      clearMessages,
      chatId,
    }),
    [messages, isStreaming, sendMessage, stopGeneration, clearMessages, chatId]
  )

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>
}
