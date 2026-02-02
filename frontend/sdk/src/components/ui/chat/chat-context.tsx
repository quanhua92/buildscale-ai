import * as React from "react"
import { toast } from "sonner"
import { useAuth } from "../../../context/AuthContext"
import {
  type CreateChatRequest,
  type CreateChatResponse,
  type PostChatMessageRequest,
  type PostChatMessageResponse,
  type ChatMode,
  type Question,
  type QuestionPendingData,
  type ModeChangedData,
} from "../../../api/types"

export type MessageRole = "user" | "assistant" | "system"

export type MessagePart =
  | { type: "text"; content: string }
  | { type: "thought"; content: string }
  | { type: "call"; tool: string; args: any; id: string }
  | { type: "observation"; output: string; success: boolean; callId: string }

// ============================================================================
// Multi-Provider Types
// ============================================================================

export type AiProvider = "openai" | "openrouter"

export interface ChatModel {
  id: string              // "openai:gpt-4o" or "openrouter:anthropic/claude-3.5-sonnet"
  provider: AiProvider
  name: string            // "GPT-4o" or "Claude 3.5 Sonnet"
  model: string           // "gpt-4o" or "anthropic/claude-3.5-sonnet"
  legacyId?: string       // For backward compatibility with old model strings
  description?: string    // Optional model description
  contextWindow?: number  // Optional context window size in tokens
  is_default?: boolean    // Whether this is the default model from the API
  is_free?: boolean       // Whether this model is free to use
}

// Models fetched from backend (filtered by configured providers)
let AVAILABLE_MODELS: ChatModel[] = []

// Parse model identifier from "provider:model" format
export function parseModelIdentifier(modelId: string, defaultProvider: AiProvider = "openai"): ChatModel | null {
  // Check if it's already in the new format
  if (modelId.includes(':')) {
    const [provider, model] = modelId.split(':', 2)
    const availableModel = AVAILABLE_MODELS.find(m => m.id === modelId)
    if (availableModel) return availableModel

    // Create a model object if not in available list
    return {
      id: modelId,
      provider: provider as AiProvider,
      name: model,
      model
    }
  }

  // Not found - create with default provider
  return {
    id: `${defaultProvider}:${modelId}`,
    provider: defaultProvider,
    name: modelId,
    model: modelId
  }
}

// Update available models from backend API
export function updateAvailableModels(models: ChatModel[]) {
  AVAILABLE_MODELS = models
}

// Get current available models
export function getAvailableModels(): ChatModel[] {
  return AVAILABLE_MODELS
}

// Group models by provider
export function groupModelsByProvider(): Record<AiProvider, ChatModel[]> {
  const grouped: Record<string, ChatModel[]> = { openai: [], openrouter: [] }
  for (const model of AVAILABLE_MODELS) {
    if (!grouped[model.provider]) {
      grouped[model.provider] = []
    }
    grouped[model.provider].push(model)
  }
  return grouped as Record<AiProvider, ChatModel[]>
}

export interface ChatMessageItem {
  id: string
  role: MessageRole
  parts: MessagePart[]
  status: "sending" | "streaming" | "completed" | "error"
  created_at: string
}

// Multi-question session state
export interface QuestionSession {
  questionId: string
  allQuestions: Question[]
  currentIndex: number
  answers: Record<string, any>  // question name -> answer
  createdAt: Date
}

interface ChatContextValue {
  messages: ChatMessageItem[]
  isStreaming: boolean
  isLoading: boolean
  sendMessage: (content: string, attachments?: string[], metadata?: Record<string, any>) => Promise<void>
  stopGeneration: () => void
  clearMessages: () => void
  chatId?: string
  model: ChatModel
  setModel: (model: ChatModel) => void
  availableModels: ChatModel[]  // Available models from backend
  // Plan Mode State
  mode: ChatMode
  planFile: string | null
  pendingQuestionSession: QuestionSession | null
  currentQuestion: Question | null  // Convenience getter for current question
  // Plan Mode Actions
  submitAnswer: (answer: any) => Promise<void>
  dismissQuestion: () => void
  setMode: (mode: ChatMode, planFile?: string) => Promise<void>
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
  const [isLoading, setIsLoading] = React.useState(false)
  const [chatId, setChatId] = React.useState<string | undefined>(initialChatId)
  // Placeholder model state, will be replaced by API response
  const [model, setModel] = React.useState<ChatModel>({
    id: 'openrouter:placeholder',
    provider: 'openrouter',
    name: 'Loading...',
    model: 'placeholder'
  })
  const [availableModels, setAvailableModelsState] = React.useState<ChatModel[]>([])

  // Plan Mode State
  const [mode, setModeState] = React.useState<ChatMode>('plan')
  const [planFile, setPlanFileState] = React.useState<string | null>(null)
  const [pendingQuestionSession, setPendingQuestionSession] = React.useState<QuestionSession | null>(null)

  // Convenience getter for current question
  const currentQuestion = React.useMemo(() => {
    if (!pendingQuestionSession) return null
    return pendingQuestionSession.allQuestions[pendingQuestionSession.currentIndex] || null
  }, [pendingQuestionSession])

  const abortControllerRef = React.useRef<AbortController | null>(null)
  const connectingRef = React.useRef<string | null>(null)
  const connectionIdRef = React.useRef<number>(0)
  const streamingTimeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null)
  const hasReceivedStreamingEventRef = React.useRef<boolean>(false)
  
  const onChatCreatedRef = React.useRef(onChatCreated)
  React.useEffect(() => {
    onChatCreatedRef.current = onChatCreated
  }, [onChatCreated])

  React.useEffect(() => {
    setChatId(initialChatId)
  }, [initialChatId])

  // Fetch available models from backend providers API
  React.useEffect(() => {
    let mounted = true

    const fetchProviders = async () => {
      try {
        console.log('[Chat] Fetching providers from workspace:', workspaceId)
        const response = await apiClientRef.current.get<{ providers: any[], default_provider: string }>(
          `/workspaces/${workspaceId}/providers`
        )

        console.log('[Chat] Providers API response:', response)

        if (mounted && response?.providers) {
          // Convert backend provider response to ChatModel array
          const models: ChatModel[] = []

          for (const provider of response.providers) {
            console.log('[Chat] Processing provider:', provider.provider, 'configured:', provider.configured, 'models:', provider.models?.length || 0)
            if (!provider.configured || !provider.models) continue

            for (const model of provider.models) {
              models.push({
                id: model.id,
                provider: provider.provider,
                name: model.display_name,
                model: model.model,
                description: model.description,
                contextWindow: model.context_window,
                is_default: model.is_default,
                is_free: model.is_free
              })
            }
          }

          console.log('[Chat] Total models fetched:', models.length, models)
          // Update available models if we got any
          if (models.length > 0) {
            if (mounted) {
              setAvailableModelsState(models)

              // Set default model based on is_default flag
              const defaultModel = models.find(m => m.is_default)
              if (defaultModel) {
                console.log('[Chat] Setting default model from API:', defaultModel)
                setModel(defaultModel)
              }
            }
            console.log('[Chat] Updated available models')
          } else {
            console.warn('[Chat] No models found in provider response')
          }
        }
      } catch (error) {
        // If providers API fails, leave models empty
        console.error('[Chat] Failed to fetch providers:', error)
        if (mounted) {
          setAvailableModelsState([])
        }
      }
    }

    fetchProviders()

    return () => {
      mounted = false
    }
  }, [workspaceId])

  const stopGeneration = React.useCallback(async () => {
    if (!chatId) return

    // Clear streaming timeout if exists
    if (streamingTimeoutRef.current) {
      clearTimeout(streamingTimeoutRef.current)
      streamingTimeoutRef.current = null
    }

    // Increment connection ID to prevent processing any buffered SSE events
    ++connectionIdRef.current

    // Abort SSE connection immediately
    if (abortControllerRef.current) {
      abortControllerRef.current.abort()
      abortControllerRef.current = null
    }
    connectingRef.current = null

    // Set streaming state to false synchronously before async backend call
    setIsStreaming(false)
    hasReceivedStreamingEventRef.current = false

    // Mark streaming message as completed
    setMessages((prev) => {
      const newMessages = [...prev]
      const lastMessage = newMessages[newMessages.length - 1]
      if (lastMessage?.status === 'streaming') {
        lastMessage.status = 'completed'
      }
      return newMessages
    })

    try {
      // Call backend stop endpoint (fire and forget)
      await apiClientRef.current.post(
        `/workspaces/${workspaceId}/chats/${chatId}/stop`,
        {}
      )
    } catch (error) {
      console.error('[Chat] Stop error', error)
    }
  }, [workspaceId, chatId])

  const connectToSse = React.useCallback(async (targetChatId: string) => {
    if (connectingRef.current === targetChatId) return

    // Abort any existing connection before starting a new one
    if (abortControllerRef.current) {
      abortControllerRef.current.abort()
      abortControllerRef.current = null
    }
    connectingRef.current = null

    // Clear any existing streaming timeout
    if (streamingTimeoutRef.current) {
      clearTimeout(streamingTimeoutRef.current)
      streamingTimeoutRef.current = null
    }

    const currentConnectionId = ++connectionIdRef.current

    const abortController = new AbortController()
    abortControllerRef.current = abortController
    connectingRef.current = targetChatId

    // Don't set isStreaming immediately - wait for actual streaming events
    hasReceivedStreamingEventRef.current = false

    // Set a timeout: if no streaming events in 1 second, turn off streaming state
    streamingTimeoutRef.current = setTimeout(() => {
      if (currentConnectionId === connectionIdRef.current && !hasReceivedStreamingEventRef.current) {
        setIsStreaming(false)
      }
      streamingTimeoutRef.current = null
    }, 1000)

    try {
      const response = await apiClientRef.current.requestRaw(
        `/workspaces/${workspaceId}/chats/${targetChatId}/events`,
        {
          headers: { 'Accept': 'text/event-stream' },
          signal: abortController.signal,
          timeout: false, // Disable timeout for SSE connections
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

              // Detect streaming events (thought, chunk, call, observation)
              // These indicate AI is actively responding
              const isStreamingEvent = ["thought", "chunk", "call", "observation"].includes(type)

              if (isStreamingEvent && currentConnectionId === connectionIdRef.current) {
                if (!hasReceivedStreamingEventRef.current) {
                  hasReceivedStreamingEventRef.current = true
                  setIsStreaming(true)

                  // Clear the timeout since we received streaming event
                  if (streamingTimeoutRef.current) {
                    clearTimeout(streamingTimeoutRef.current)
                    streamingTimeoutRef.current = null
                  }
                }
              }

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
                    if (currentConnectionId === connectionIdRef.current) {
                      setIsStreaming(false)
                      // Clear streaming timeout
                      if (streamingTimeoutRef.current) {
                        clearTimeout(streamingTimeoutRef.current)
                        streamingTimeoutRef.current = null
                      }
                    }
                    break
                  case "error":
                    updatedMessage.status = "error"
                    updatedMessage.parts.push({ type: "text", content: `\nError: ${data.message}` })
                    if (currentConnectionId === connectionIdRef.current) {
                      setIsStreaming(false)
                      // Clear streaming timeout
                      if (streamingTimeoutRef.current) {
                        clearTimeout(streamingTimeoutRef.current)
                        streamingTimeoutRef.current = null
                      }
                    }
                    break
                  case "stopped":
                    updatedMessage.status = "completed"
                    if (currentConnectionId === connectionIdRef.current) {
                      setIsStreaming(false)
                      // Clear streaming timeout
                      if (streamingTimeoutRef.current) {
                        clearTimeout(streamingTimeoutRef.current)
                        streamingTimeoutRef.current = null
                      }
                    }
                    break
                  case "file_updated":
                    return prev
                  case "question_pending":
                    // Handle question_pending event
                    if (currentConnectionId === connectionIdRef.current) {
                      const questionData: QuestionPendingData = data
                      // Create a question session with all questions
                      if (questionData.questions && questionData.questions.length > 0) {
                        setPendingQuestionSession({
                          questionId: questionData.question_id,
                          allQuestions: questionData.questions.map((q) => ({
                            ...q,
                            id: questionData.question_id,
                            createdAt: new Date(questionData.created_at)
                          })),
                          currentIndex: 0,  // Start with first question
                          answers: {},  // No answers yet
                          createdAt: new Date(questionData.created_at)
                        })
                      }
                    }
                    return prev
                  case "mode_changed":
                    // Handle mode_changed event
                    if (currentConnectionId === connectionIdRef.current) {
                      const modeData: ModeChangedData = data
                      setModeState(modeData.mode)
                      setPlanFileState(modeData.plan_file)
                    }
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
        // Clear streaming timeout on connection end
        if (currentConnectionId === connectionIdRef.current) {
          if (streamingTimeoutRef.current) {
            clearTimeout(streamingTimeoutRef.current)
            streamingTimeoutRef.current = null
          }
        }
      }
    } catch (error) {
      if ((error as Error).name === 'AbortError') {
        // Clear streaming timeout on abort
        if (currentConnectionId === connectionIdRef.current) {
          if (streamingTimeoutRef.current) {
            clearTimeout(streamingTimeoutRef.current)
            streamingTimeoutRef.current = null
          }
        }
        return
      }
      console.error(`[Chat] [Conn:${currentConnectionId}] SSE Error:`, error)
      if (currentConnectionId === connectionIdRef.current) {
        setIsStreaming(false)
        connectingRef.current = null
        // Clear streaming timeout on error
        if (streamingTimeoutRef.current) {
          clearTimeout(streamingTimeoutRef.current)
          streamingTimeoutRef.current = null
        }
      }
    }
  }, [workspaceId])

  React.useEffect(() => {
    let mounted = true
    const initChat = async () => {
      if (!chatId) {
        stopGeneration()
        return
      }

      setIsLoading(true)
      try {
        const session = await apiClientRef.current.getChat(workspaceId, chatId)
        if (mounted) {
          const historyMessages: ChatMessageItem[] = session.messages.map(msg => ({
            id: msg.id,
            role: msg.role as MessageRole,
            parts: [{ type: "text", content: msg.content }],
            status: "completed",
            created_at: msg.created_at
          }))
          setMessages(historyMessages)

          // Load model from existing chat session
          // Priority: 1) chat's saved model (if available), 2) API's default model
          const modelId = session.agent_config.model // string
          const parsedModel = parseModelIdentifier(modelId)

          // Check if the parsed model exists in our available models list
          const modelInAvailableModels = availableModels.find(m => m.id === parsedModel?.id)
          if (modelInAvailableModels) {
            console.log('[Chat] Using saved model from chat session:', modelInAvailableModels)
            setModel(modelInAvailableModels)
          } else {
            // Fallback to the API's default model
            const apiDefaultModel = availableModels.find(m => m.is_default)
            if (apiDefaultModel) {
              console.log('[Chat] Saved model not found, using API default model:', apiDefaultModel)
              setModel(apiDefaultModel)
            } else {
              console.warn('[Chat] No saved model and no API default found, using first available model')
              setModel(availableModels[0])
            }
          }

          // Initialize Plan Mode state from chat metadata (agent_config)
          // Default to 'plan' mode if not set
          setModeState(session.agent_config.mode || 'plan')
          setPlanFileState(session.agent_config.plan_file || null)

          // Connect to SSE only after history is loaded
          connectToSse(chatId)
        }
      } catch (error) {
        console.error('[Chat] Failed to load history:', error)
        if (mounted) {
          // Even if history fails, try to connect to SSE
          connectToSse(chatId)
        }
      } finally {
        if (mounted) setIsLoading(false)
      }
    }

    initChat()

    return () => {
      mounted = false
      if (abortControllerRef.current) abortControllerRef.current.abort()
      connectingRef.current = null
      // Clear streaming timeout on cleanup
      if (streamingTimeoutRef.current) {
        clearTimeout(streamingTimeoutRef.current)
        streamingTimeoutRef.current = null
      }
    }
  }, [chatId, workspaceId, connectToSse, stopGeneration, availableModels])

  const sendMessage = React.useCallback(
    async (content: string, _attachments?: string[], metadata?: Record<string, any>) => {
      const userMessage: ChatMessageItem = {
        id: generateId(),
        role: "user",
        parts: [{ type: "text", content }],
        status: "sending",
        created_at: new Date().toISOString(),
      }

      setMessages((prev) => [...prev, userMessage])

      // Clear any existing streaming timeout
      if (streamingTimeoutRef.current) {
        clearTimeout(streamingTimeoutRef.current)
        streamingTimeoutRef.current = null
      }

      // Reset streaming event flag
      hasReceivedStreamingEventRef.current = false

      // Set streaming state when sending message
      setIsStreaming(true)

      const maxRetries = 3
      const retryDelay = 1000 // 1 second between retries
      let lastError: Error | null = null

      for (let attempt = 1; attempt <= maxRetries; attempt++) {
        try {
          if (!chatId) {
            const response = await apiClientRef.current.post<CreateChatResponse>(
              `/workspaces/${workspaceId}/chats`,
              { goal: content, model: model.id } as CreateChatRequest
            )
            if (!response?.chat_id) throw new Error('Invalid server response')
            setChatId(response.chat_id)
            onChatCreatedRef.current?.(response.chat_id)

            // Update message status to completed
            setMessages((prev) => {
              const newMessages = [...prev]
              const last = newMessages[newMessages.length - 1]
              if (last?.id === userMessage.id) last.status = "completed"
              return newMessages
            })
          } else {
            const response = await apiClientRef.current.post<PostChatMessageResponse>(
              `/workspaces/${workspaceId}/chats/${chatId}`,
              { content, model: model.id, metadata } as PostChatMessageRequest
            )
            if (response?.status !== "accepted") throw new Error('Message not accepted')

            // Only reconnect if SSE is not currently connected to this chat
            if (connectingRef.current !== chatId) {
              connectToSse(chatId)
            }

            // Update message status to completed
            setMessages((prev) => {
              const newMessages = [...prev]
              const last = newMessages[newMessages.length - 1]
              if (last?.id === userMessage.id) last.status = "completed"
              return newMessages
            })
          }

          // Success! Exit retry loop
          return
        } catch (error) {
          lastError = error instanceof Error ? error : new Error(String(error))

          // Check if error is retryable (network errors, timeouts, 5xx)
          const apiError = error as { code?: string; status?: number }
          const isRetryable = !apiError.status || apiError.status >= 500 || apiError.status === 408

          if (!isRetryable || attempt === maxRetries) {
            // Not retryable or max retries reached
            console.error(`[Chat] Send error (attempt ${attempt}/${maxRetries})`, error)

            // Update message status to error
            setMessages((prev) => {
              const newMessages = [...prev]
              const last = newMessages[newMessages.length - 1]
              if (last?.id === userMessage.id) last.status = "error"
              return newMessages
            })

            setIsStreaming(false)
            hasReceivedStreamingEventRef.current = false

            // Re-throw for caller to handle
            throw lastError
          }

          // Retry after delay
          console.warn(`[Chat] Retrying message send (attempt ${attempt + 1}/${maxRetries})...`)
          await new Promise(resolve => setTimeout(resolve, retryDelay))
        }
      }

      // Should not reach here, but TypeScript needs it
      throw lastError || new Error('Failed to send message')
    },
    [workspaceId, chatId, connectToSse, model]
  )

  const clearMessages = React.useCallback(() => {
    setMessages([])
    stopGeneration()
  }, [stopGeneration])

  // Plan Mode: Submit answer to pending question
  const submitAnswer = React.useCallback(
    async (answer: any) => {
      if (!pendingQuestionSession || !chatId) return

      const currentQ = currentQuestion
      if (!currentQ) return

      // Save answer for this question
      const newAnswers = {
        ...pendingQuestionSession.answers,
        [currentQ.name!]: answer
      }

      // Check if there are more questions
      const nextIndex = pendingQuestionSession.currentIndex + 1
      const hasMoreQuestions = nextIndex < pendingQuestionSession.allQuestions.length

      if (hasMoreQuestions) {
        // Move to next question without sending message yet
        setPendingQuestionSession({
          ...pendingQuestionSession,
          currentIndex: nextIndex,
          answers: newAnswers
        })
      } else {
        // All questions answered - send with structured metadata
        const answerCount = pendingQuestionSession.allQuestions.length

        // Build summary with each answer on separate lines
        const answerLines: string[] = []
        answerLines.push(`[User answered ${answerCount} question${answerCount > 1 ? 's' : ''}]`)

        // Add each answer with question text
        for (const q of pendingQuestionSession.allQuestions) {
          const ans = newAnswers[q.name!]
          answerLines.push(`Q: ${q.question}`)
          if (q.buttons && q.buttons.length > 0) {
            const matchingButton = q.buttons.find((b: any) => b.value === ans)
            if (matchingButton) {
              answerLines.push(`[Answered: "${matchingButton.label}"]`)
            }
          } else {
            // Non-button answers (text input, etc.)
            answerLines.push(`[Answered: ${JSON.stringify(ans)}]`)
          }
        }

        let summaryText = answerLines.join('\n')

        try {
          // Send message with structured metadata
          await sendMessage(summaryText, undefined, {
            question_answer: {
              question_id: pendingQuestionSession.questionId,
              answers: newAnswers
            }
          })
          setPendingQuestionSession(null)
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Unknown error'
          toast.error(`Failed to send answers after 3 attempts: ${errorMessage}`)
          console.error('[Chat] Submit answers error', error)
        }
      }
    },
    [pendingQuestionSession, currentQuestion, chatId, sendMessage]
  )

  // Plan Mode: Dismiss pending question
  const dismissQuestion = React.useCallback(() => {
    setPendingQuestionSession(null)
  }, [])

  // Plan Mode: Update chat mode
  const setMode = React.useCallback(
    async (newMode: ChatMode, newPlanFile?: string) => {
      try {
        let targetChatId = chatId

        // If no chat exists, create one first with a default message
        if (!targetChatId) {
          const response = await apiClientRef.current.post<CreateChatResponse>(
            `/workspaces/${workspaceId}/chats`,
            {
              goal: `Starting in ${newMode} mode`,
              model: model.id
            } as CreateChatRequest
          )

          if (!response?.chat_id) {
            throw new Error('Failed to create chat')
          }

          targetChatId = response.chat_id
          // Update chatId state so we don't create again
          setChatId(targetChatId)
          onChatCreatedRef.current?.(targetChatId)
        }

        // Now update the mode
        await apiClientRef.current.patch(
          `/workspaces/${workspaceId}/chats/${targetChatId}`,
          {
            app_data: {
              mode: newMode,
              plan_file: newPlanFile || null
            }
          }
        )

        // Only update state on successful API call
        setModeState(newMode)
        if (newPlanFile !== undefined) {
          setPlanFileState(newPlanFile)
        }

        const modeLabel = newMode === 'plan' ? 'Plan' : 'Build'
        toast.success(`Switched to ${modeLabel} Mode`)
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error'
        toast.error(`Failed to change mode: ${errorMessage}`)
        console.error('[Chat] Set mode error', error)
      }
    },
    [workspaceId, chatId, model, setChatId, onChatCreatedRef]
  )

  const value = React.useMemo(
    () => ({
      messages, isStreaming, isLoading, sendMessage, stopGeneration, clearMessages, chatId,
      model, setModel, availableModels,
      // Plan Mode
      mode, planFile, pendingQuestionSession, currentQuestion,
      submitAnswer, dismissQuestion, setMode
    }),
    [messages, isStreaming, isLoading, sendMessage, stopGeneration, clearMessages, chatId, model, setModel, availableModels,
     mode, planFile, pendingQuestionSession, currentQuestion, submitAnswer, dismissQuestion, setMode]
  )

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>
}
