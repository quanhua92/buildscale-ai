import * as React from "react"
import { useAuth } from "../../../context/AuthContext"
import {
  type CreateChatRequest,
  type CreateChatResponse,
  type PostChatMessageRequest,
  type PostChatMessageResponse,
} from "../../../api/types"

export type MessageRole = "user" | "assistant" | "system"

export type MessagePart =
  | { type: "text"; content: string }
  | { type: "thought"; content: string }
  | { type: "call"; tool: string; args: any; id: string }
  | { type: "observation"; output: string; success: boolean; callId: string }

export interface ChatMessageItem {
  id: string
  role: MessageRole
  parts: MessagePart[]
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
  const connectionIdRef = React.useRef<number>(0)
  
  const onChatCreatedRef = React.useRef(onChatCreated)
  React.useEffect(() => {
    onChatCreatedRef.current = onChatCreated
  }, [onChatCreated])

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
    if (connectingRef.current === targetChatId) return

    const currentConnectionId = ++connectionIdRef.current

    if (abortControllerRef.current) abortControllerRef.current.abort()
    
    const abortController = new AbortController()
    abortControllerRef.current = abortController
    connectingRef.current = targetChatId
    setIsStreaming(true)

    try {
      const response = await apiClientRef.current.requestRaw(
        `/workspaces/${workspaceId}/chats/${targetChatId}/events`,
        {
          headers: { 'Accept': 'text/event-stream' },
          signal: abortController.signal,
        }
      )

      if (!response.ok) throw new Error(`SSE Connection failed: ${response.statusText}`)
      
      const reader = response.body?.getReader()
      if (!reader) throw new Error('No reader available')

      const decoder = new TextDecoder()
      let buffer = ''

      try {
        while (true) {
          const { done, value } = await reader.read()
          if (done) break
          if (currentConnectionId !== connectionIdRef.current) break

          buffer += decoder.decode(value, { stream: true })
          const lines = buffer.split('\n\n')
          buffer = lines.pop() || ''

          for (const line of lines) {
            const parts = line.split('\n')
            let eventType = 'chunk'
            let dataStr = ''

            for (const p of parts) {
              if (p.startsWith('event: ')) eventType = p.slice(7).trim()
              else if (p.startsWith('data: ')) dataStr = p.slice(6).trim()
            }

            if (!dataStr) continue
            
            try {
              const payload = JSON.parse(dataStr)
              const type = payload.type || eventType
              const data = payload.data || payload

              if (type === "ping") continue
              if (currentConnectionId !== connectionIdRef.current) break

              setMessages((prev) => {
                if (currentConnectionId !== connectionIdRef.current) return prev

                const newMessages = [...prev]
                let lastMessage = newMessages[newMessages.length - 1]
                
                if (!lastMessage || lastMessage.role !== "assistant" || lastMessage.status === "completed") {
                  if (type === "session_init" || type === "file_updated") return prev

                  lastMessage = {
                    id: generateId(),
                    role: "assistant",
                    parts: [],
                    status: "streaming",
                    created_at: new Date().toISOString(),
                  }
                  newMessages.push(lastMessage)
                }

                const updatedMessage = { ...lastMessage, parts: [...lastMessage.parts] }
                const lastPart = updatedMessage.parts[updatedMessage.parts.length - 1]

                switch (type) {
                  case "session_init":
                    if (data.chat_id && data.chat_id !== targetChatId) {
                      setChatId(data.chat_id)
                      onChatCreatedRef.current?.(data.chat_id)
                    }
                    return prev
                  case "thought":
                    if (lastPart?.type === "thought") {
                      lastPart.content += (data.text || "")
                    } else {
                      updatedMessage.parts.push({ type: "thought", content: (data.text || "") })
                    }
                    updatedMessage.status = "streaming"
                    break
                  case "chunk":
                    if (lastPart?.type === "text") {
                      lastPart.content += (data.text || "")
                    } else {
                      updatedMessage.parts.push({ type: "text", content: (data.text || "") })
                    }
                    updatedMessage.status = "streaming"
                    break
                  case "call": {
                    const callId = generateId()
                    updatedMessage.parts.push({ type: "call", tool: data.tool, args: data.args, id: callId })
                    updatedMessage.status = "streaming"
                    break
                  }
                  case "observation":
                    // Look for the last call part to link it, or just push it
                    updatedMessage.parts.push({ 
                      type: "observation", 
                      output: data.output, 
                      success: data.success ?? true,
                      callId: "" // We'll link visually by order for now
                    })
                    updatedMessage.status = "streaming"
                    break
                  case "done":
                    updatedMessage.status = "completed"
                    if (currentConnectionId === connectionIdRef.current) setIsStreaming(false)
                    break
                  case "error":
                    updatedMessage.status = "error"
                    updatedMessage.parts.push({ type: "text", content: `\nError: ${data.message}` })
                    if (currentConnectionId === connectionIdRef.current) setIsStreaming(false)
                    break
                  case "file_updated":
                    return prev
                }

                newMessages[newMessages.length - 1] = updatedMessage
                return newMessages
              })
            } catch (e) {
              console.error(`[Chat] [Conn:${currentConnectionId}] SSE Parse error`, e)
            }
          }
        }
      } finally {
        reader.releaseLock()
      }
    } catch (error) {
      if ((error as Error).name === 'AbortError') return
      console.error(`[Chat] [Conn:${currentConnectionId}] SSE Error:`, error)
      if (currentConnectionId === connectionIdRef.current) {
        setIsStreaming(false)
        connectingRef.current = null
      }
    }
  }, [workspaceId])

  React.useEffect(() => {
    if (chatId) connectToSse(chatId)
    else stopGeneration()
    return () => {
      if (abortControllerRef.current) abortControllerRef.current.abort()
      connectingRef.current = null
    }
  }, [chatId, connectToSse, stopGeneration])

  const sendMessage = React.useCallback(
    async (content: string, _attachments?: string[]) => {
      const userMessage: ChatMessageItem = {
        id: generateId(),
        role: "user",
        parts: [{ type: "text", content }],
        status: "completed",
        created_at: new Date().toISOString(),
      }

      setMessages((prev) => [...prev, userMessage])
      
      try {
        if (!chatId) {
          const response = await apiClientRef.current.post<CreateChatResponse>(
            `/workspaces/${workspaceId}/chats`,
            { goal: content } as CreateChatRequest
          )
          if (!response?.chat_id) throw new Error('Invalid server response')
          setChatId(response.chat_id)
          onChatCreatedRef.current?.(response.chat_id)
        } else {
          const response = await apiClientRef.current.post<PostChatMessageResponse>(
            `/workspaces/${workspaceId}/chats/${chatId}`,
            { content } as PostChatMessageRequest
          )
          if (response?.status !== "accepted") throw new Error('Message not accepted')
          connectToSse(chatId)
        }
      } catch (error) {
        console.error('[Chat] Send error', error)
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
      messages, isStreaming, sendMessage, stopGeneration, clearMessages, chatId
    }),
    [messages, isStreaming, sendMessage, stopGeneration, clearMessages, chatId]
  )

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>
}
