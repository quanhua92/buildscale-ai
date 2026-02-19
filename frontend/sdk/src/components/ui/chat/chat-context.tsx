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
  type ChatFile,
  type SessionStatus,
} from "../../../api/types"
import { useMultiChatSSEManager } from "./multi-chat-sse-manager"

export type MessageRole = "user" | "assistant" | "system" | "tool"

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

// ============================================================================
// Multi-Chat State Types
// ============================================================================

export interface ChatSessionState {
  chatId: string
  messages: ChatMessageItem[]
  isStreaming: boolean
  isLoading: boolean
  model: ChatModel
  mode: ChatMode
  planFile: string | null
  pendingQuestionSession: QuestionSession | null
  lastAccessedAt: number
  sessionStatus?: SessionStatus
}

interface ChatContextValue {
  // Current active chat state
  messages: ChatMessageItem[]
  isStreaming: boolean
  isLoading: boolean
  sendMessage: (content: string, attachments?: string[], metadata?: Record<string, any>) => Promise<void>
  stopGeneration: () => void
  clearMessages: () => void
  chatId?: string
  workspaceId: string
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

  // Multi-Chat State
  activeChatId: string | null
  setActiveChatId: (chatId: string | null) => void
  chatSessions: Map<string, ChatSessionState>
  switchToChat: (chatId: string) => Promise<void>
  recentChats: ChatFile[]
  refreshRecentChats: () => Promise<void>
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
  initialChatId?: string  // Only for initial deep linking from URL
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
  initialChatId,
  onChatCreated,
}: ChatProviderProps) {
  const { apiClient } = useAuth()
  const sseManager = useMultiChatSSEManager()
  const apiClientRef = React.useRef(apiClient)
  React.useEffect(() => {
    apiClientRef.current = apiClient
    sseManager.setApiClient(apiClient)
  }, [apiClient, sseManager])

  const [messages, setMessages] = React.useState<ChatMessageItem[]>([])
  // NOTE: isStreaming is now derived from the current chat's session state (see useMemo below)
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

  // ============================================================================
  // Multi-Chat State
  // ============================================================================

  // Active chat ID (client-side state, NOT from URL)
  const [activeChatId, setActiveChatId] = React.useState<string | null>(initialChatId ?? null)

  // Chat sessions cache - stores state for multiple chats
  const [chatSessions, setChatSessions] = React.useState<Map<string, ChatSessionState>>(new Map())

  // Recent chats list
  const [recentChats, setRecentChats] = React.useState<ChatFile[]>([])

  // Add new chat optimistically to recentChats for instant tab appearance
  const addRecentChatOptimistic = React.useCallback((newChatId: string) => {
    setRecentChats((prev) => {
      // Avoid duplicates
      if (prev.some(c => c.chat_id === newChatId)) {
        return prev
      }
      // Add new chat at the beginning with minimal data
      const newChat: ChatFile = {
        id: newChatId,
        chat_id: newChatId,
        name: 'New Chat', // Will be updated by server refresh
        path: newChatId, // Will be updated by server refresh
        updated_at: new Date().toISOString(),
        created_at: new Date().toISOString(),
      }
      return [newChat, ...prev]
    })
  }, [])

  // Track if initial chat has been loaded
  const initialChatLoadedRef = React.useRef(false)

  // Track which chatId has been fully loaded to prevent duplicate loads
  const loadedChatIdRef = React.useRef<string | null>(null)

  // Track which chatId has an active SSE connection to prevent duplicate connections
  const connectedChatIdRef = React.useRef<string | null>(null)

  const abortControllerRef = React.useRef<AbortController | null>(null)
  const connectingRef = React.useRef<string | null>(null)
  const connectionIdRef = React.useRef<number>(0)
  const streamingTimeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null)
  const hasReceivedStreamingEventRef = React.useRef<boolean>(false)

  // CRITICAL: Track current chatId with a ref to avoid stale closure issues in SSE callbacks
  // Refs always have the current value, unlike closure-captured state variables
  const currentChatIdRef = React.useRef<string | null>(chatId ?? null)
  React.useEffect(() => {
    currentChatIdRef.current = chatId ?? null
  }, [chatId])

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
    hasReceivedStreamingEventRef.current = false
    // Update per-chat streaming state
    setChatSessions((prev) => {
      const newSessions = new Map(prev)
      const session = newSessions.get(chatId)
      if (session) {
        newSessions.set(chatId, { ...session, isStreaming: false })
      }
      return newSessions
    })

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

  const connectToSse = React.useCallback(
    async (targetChatId: string) => {
      console.log('[Chat] Connecting to SSE for chat:', targetChatId)

      // Skip if already connected to this chat
      if (connectedChatIdRef.current === targetChatId) {
        console.log('[Chat] Already connected to SSE for this chat, skipping')
        return
      }

      // Mark as connecting immediately to prevent race conditions
      connectedChatIdRef.current = targetChatId

      // Use the MultiChatSSEManager to maintain multiple connections
      await sseManager.connectChat(targetChatId, workspaceId, (event) => {
        const { type, data } = event

        if (type === 'ping') return

        // IMPORTANT: Log to debug cross-contamination
        console.log('[SSE] Event received', {
          eventType: type,
          targetChatId,
          capturedChatId: chatId, // Closure value (stale)
          currentChatIdRef: currentChatIdRef.current, // Actual current
          match: targetChatId === currentChatIdRef.current,
          message: `SSE event ${type} for ${targetChatId}, ref has ${currentChatIdRef.current}`
        })

        // Detect streaming events
        const isStreamingEvent = ['thought', 'chunk', 'call', 'observation'].includes(type)

        if (isStreamingEvent) {
          if (!hasReceivedStreamingEventRef.current) {
            hasReceivedStreamingEventRef.current = true
            // Update per-chat streaming state
            setChatSessions((prev) => {
              const newSessions = new Map(prev)
              const session = newSessions.get(targetChatId)
              if (session) {
                newSessions.set(targetChatId, { ...session, isStreaming: true })
              }
              return newSessions
            })
          }
        }

        setMessages((prev) => {
          // CRITICAL: Use ref to get the ACTUAL current chat, not stale closure value
          const isCurrentChat = targetChatId === currentChatIdRef.current

          console.log('[SSE] Processing in setMessages', {
            eventType: type,
            targetChatId,
            chatIdRef: currentChatIdRef.current, // Always current
            isCurrentChat,
            willProcess: isCurrentChat
          })

          if (!isCurrentChat) {
            // Event is for background chat - update cache
            setChatSessions((prevSessions) => {
              const newSessions = new Map(prevSessions)
              const cachedSession = newSessions.get(targetChatId)

              if (cachedSession) {
                // Apply event to cached messages
                const cachedMessages = cachedSession.messages
                const newCached = [...cachedMessages]
                let lastMessage = newCached[newCached.length - 1]

                if (!lastMessage || lastMessage.role !== 'assistant' || lastMessage.status === 'completed') {
                  if (type === 'session_init' || type === 'file_updated') return prevSessions

                  lastMessage = {
                    id: generateId(),
                    role: 'assistant',
                    parts: [],
                    status: 'streaming',
                    created_at: new Date().toISOString(),
                  }
                  newCached.push(lastMessage)
                }

                const updatedMessage = { ...lastMessage, parts: [...lastMessage.parts] }
                const lastPart = updatedMessage.parts[updatedMessage.parts.length - 1]

                // Process event types (same logic as current chat below)
                switch (type) {
                  case 'session_init':
                    if (data.chat_id && data.chat_id !== targetChatId) {
                      // Chat ID changed, update in cache
                    }
                    return prevSessions
                  case 'thought':
                    if (lastPart?.type === 'thought') {
                      lastPart.content += (data.text || '')
                    } else {
                      updatedMessage.parts.push({ type: 'thought', content: (data.text || '') })
                    }
                    updatedMessage.status = 'streaming'
                    break
                  case 'chunk':
                    if (lastPart?.type === 'text') {
                      lastPart.content += (data.text || '')
                    } else {
                      updatedMessage.parts.push({ type: 'text', content: (data.text || '') })
                    }
                    updatedMessage.status = 'streaming'
                    break
                  case 'call': {
                    const callId = generateId()
                    updatedMessage.parts.push({ type: 'call', tool: data.tool, args: data.args, id: callId })
                    updatedMessage.status = 'streaming'
                    break
                  }
                  case 'observation':
                    updatedMessage.parts.push({
                      type: 'observation',
                      output: data.output,
                      success: data.success ?? true,
                      callId: '',
                    })
                    updatedMessage.status = 'streaming'
                    break
                  case 'done':
                    updatedMessage.status = 'completed'
                    // CRITICAL: Also update the session's isStreaming state for background chats
                    newSessions.set(targetChatId, { ...cachedSession, messages: newCached, isStreaming: false })
                    return newSessions
                  case 'error':
                    updatedMessage.status = 'error'
                    updatedMessage.parts.push({ type: 'text', content: `\nError: ${data.message}` })
                    // CRITICAL: Also update the session's isStreaming state for background chats
                    newSessions.set(targetChatId, { ...cachedSession, messages: newCached, isStreaming: false })
                    return newSessions
                  case 'stopped':
                    updatedMessage.status = 'completed'
                    // CRITICAL: Also update the session's isStreaming state for background chats
                    newSessions.set(targetChatId, { ...cachedSession, messages: newCached, isStreaming: false })
                    return newSessions
                  case 'file_updated':
                  case 'question_pending':
                  case 'mode_changed':
                    return prevSessions
                }

                newCached[newCached.length - 1] = updatedMessage
                newSessions.set(targetChatId, { ...cachedSession, messages: newCached })
              }

              return newSessions
            })
            return prev // Don't update current chat's messages
          }

          // Event is for current chat - process normally

          const newMessages = [...prev]
          let lastMessage = newMessages[newMessages.length - 1]

          if (!lastMessage || lastMessage.role !== 'assistant' || lastMessage.status === 'completed') {
            if (type === 'session_init' || type === 'file_updated') return prev

            lastMessage = {
              id: generateId(),
              role: 'assistant',
              parts: [],
              status: 'streaming',
              created_at: new Date().toISOString(),
            }
            newMessages.push(lastMessage)
          }

          const updatedMessage = { ...lastMessage, parts: [...lastMessage.parts] }
          const lastPart = updatedMessage.parts[updatedMessage.parts.length - 1]

          switch (type) {
            case 'session_init':
              if (data.chat_id && data.chat_id !== targetChatId) {
                setChatId(data.chat_id)
                onChatCreatedRef.current?.(data.chat_id)
                // Immediately add to recent chats for instant tab appearance
                addRecentChatOptimistic(data.chat_id)
              }
              return prev
            case 'thought':
              if (lastPart?.type === 'thought') {
                lastPart.content += (data.text || '')
              } else {
                updatedMessage.parts.push({ type: 'thought', content: (data.text || '') })
              }
              updatedMessage.status = 'streaming'
              break
            case 'chunk':
              if (lastPart?.type === 'text') {
                lastPart.content += (data.text || '')
              } else {
                updatedMessage.parts.push({ type: 'text', content: (data.text || '') })
              }
              updatedMessage.status = 'streaming'
              break
            case 'call': {
              const callId = generateId()
              updatedMessage.parts.push({ type: 'call', tool: data.tool, args: data.args, id: callId })
              updatedMessage.status = 'streaming'
              break
            }
            case 'observation':
              updatedMessage.parts.push({
                type: 'observation',
                output: data.output,
                success: data.success ?? true,
                callId: '',
              })
              updatedMessage.status = 'streaming'
              break
            case 'done':
              updatedMessage.status = 'completed'
              // Update per-chat streaming state
              setChatSessions((prev) => {
                const newSessions = new Map(prev)
                const session = newSessions.get(targetChatId)
                if (session) {
                  newSessions.set(targetChatId, { ...session, isStreaming: false })
                }
                return newSessions
              })
              break
            case 'error':
              updatedMessage.status = 'error'
              updatedMessage.parts.push({ type: 'text', content: `\nError: ${data.message}` })
              // Update per-chat streaming state
              setChatSessions((prev) => {
                const newSessions = new Map(prev)
                const session = newSessions.get(targetChatId)
                if (session) {
                  newSessions.set(targetChatId, { ...session, isStreaming: false })
                }
                return newSessions
              })
              break
            case 'stopped':
              updatedMessage.status = 'completed'
              // Update per-chat streaming state
              setChatSessions((prev) => {
                const newSessions = new Map(prev)
                const session = newSessions.get(targetChatId)
                if (session) {
                  newSessions.set(targetChatId, { ...session, isStreaming: false })
                }
                return newSessions
              })
              break
            case 'file_updated':
              return prev
            case 'question_pending':
              const questionData: QuestionPendingData = data
              if (questionData.questions && questionData.questions.length > 0) {
                setPendingQuestionSession({
                  questionId: questionData.question_id,
                  allQuestions: questionData.questions.map((q) => ({
                    ...q,
                    id: questionData.question_id,
                    createdAt: new Date(questionData.created_at),
                  })),
                  currentIndex: 0,
                  answers: {},
                  createdAt: new Date(questionData.created_at),
                })
              }
              return prev
            case 'mode_changed':
              const modeData: ModeChangedData = data
              setModeState(modeData.mode)
              setPlanFileState(modeData.plan_file)
              return prev
          }

          newMessages[newMessages.length - 1] = updatedMessage
          return newMessages
        })
      })
    },
    [workspaceId, chatId, sseManager, setChatId, setMessages, setPendingQuestionSession, setModeState, setPlanFileState, setChatSessions, chatSessions, addRecentChatOptimistic]
  )

  React.useEffect(() => {
    let mounted = true
    const initChat = async () => {
      console.log('[Chat] initChat effect called', { chatId, workspaceId, mounted, loadedChatId: loadedChatIdRef.current })

      if (!chatId) {
        stopGeneration()
        loadedChatIdRef.current = null
        return
      }

      // Skip loading if this chat was already loaded (prevent duplicates from availableModels changes)
      if (loadedChatIdRef.current === chatId && mounted) {
        console.log('[Chat] Chat already loaded, skipping', { chatId })
        return
      }

      setIsLoading(true)
      try {
        const session = await apiClientRef.current.getChat(workspaceId, chatId)
         if (mounted) {
           // Group messages by reasoning_id to merge reasoning, tools, and response parts into a single bubble per turn
           const messageGroups = new Map<string, ChatMessageItem>();

            for (const msg of session.messages) {
              const role = msg.role as MessageRole;
              const meta = msg.metadata;
              const reasoningId = meta?.reasoning_id;
             // If reasoning_id exists, group by it. Otherwise use msg.id for unique messages (user, system, etc)
             const groupKey = reasoningId || msg.id;

             let group = messageGroups.get(groupKey);
             if (!group) {
               group = {
                 id: groupKey,
                 role,
                 parts: [],
                 status: "completed",
                 created_at: msg.created_at,
               };
               messageGroups.set(groupKey, group);
             }

             const messageType = meta?.message_type;
             if (messageType === "reasoning_chunk" || messageType === "reasoning_complete") {
               const lastPart = group.parts[group.parts.length - 1];
               if (lastPart?.type === "thought") {
                 lastPart.content += msg.content;
               } else {
                 group.parts.push({ type: "thought", content: msg.content });
               }
              } else if (messageType === "tool_call") {
                group.parts.push({
                  type: "call",
                  tool: meta?.tool_name || "unknown",
                  args: meta?.tool_arguments,
                  id: msg.id,
                });
              } else if (messageType === "tool_result") {
                group.parts.push({
                  type: "observation",
                  output: meta?.tool_output || "",
                  success: meta?.tool_success ?? true,
                  callId: msg.id,
                });
              } else {
               // Normal text message
               group.parts.push({ type: "text", content: msg.content });
             }
           }

           const historyMessages: ChatMessageItem[] = Array.from(messageGroups.values())
             .sort((a, b) => a.created_at.localeCompare(b.created_at));

          setMessages(historyMessages);

          // Mark this chat as successfully loaded to prevent duplicate loads
          loadedChatIdRef.current = chatId

          // Add current chat to chatSessions cache
          setChatSessions((prev) => {
            const newSessions = new Map(prev)
            newSessions.set(chatId, {
              chatId,
              messages: historyMessages,
              isStreaming: false,
              isLoading: false,
              model,
              mode: session.agent_config.mode || 'plan',
              planFile: session.agent_config.plan_file || null,
              pendingQuestionSession: null,
              lastAccessedAt: Date.now(),
            })
            return newSessions
          })

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

          // Add current chat to chatSessions cache
          setChatSessions((prev) => {
            const newSessions = new Map(prev)
            newSessions.set(chatId, {
              chatId,
              messages: historyMessages,
              isStreaming: false,
              isLoading: false,
              model,
              mode: session.agent_config.mode || 'plan',
              planFile: session.agent_config.plan_file || null,
              pendingQuestionSession: null,
              lastAccessedAt: Date.now(),
            })
            console.log('[Chat] Added chat to sessions cache:', chatId, 'Total sessions:', newSessions.size)
            return newSessions
          })

          // Connect to SSE only after history is loaded
          connectToSse(chatId)
        }
      } catch (error) {
        console.error('[Chat] Failed to load history:', error)
        if (mounted) {
          // Even if history fails, add an empty session to chatSessions
          setChatSessions((prev) => {
            const newSessions = new Map(prev)
            newSessions.set(chatId, {
              chatId,
              messages: [],
              isStreaming: false,
              isLoading: false,
              model,
              mode: 'plan',
              planFile: null,
              pendingQuestionSession: null,
              lastAccessedAt: Date.now(),
            })
            console.log('[Chat] Added empty chat session due to error:', chatId)
            return newSessions
          })
          // Try to connect to SSE
          connectToSse(chatId)
        }
      } finally {
        if (mounted) setIsLoading(false)
      }
    }

    initChat()

    return () => {
      mounted = false
      // Note: Don't abort SSE connection here - it's managed by connectToSse
      // The connectToSse function will abort old connections when connecting to a new chat
      // Clear streaming timeout on cleanup
      if (streamingTimeoutRef.current) {
        clearTimeout(streamingTimeoutRef.current)
        streamingTimeoutRef.current = null
      }
    }
  }, [chatId, workspaceId, stopGeneration, availableModels])

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

      // Set streaming state when sending message (update per-chat state)
      setChatSessions((prev) => {
        const newSessions = new Map(prev)
        if (chatId) {
          const session = newSessions.get(chatId)
          if (session) {
            newSessions.set(chatId, { ...session, isStreaming: true })
          }
        }
        return newSessions
      })

      const maxRetries = 3
      const retryDelay = 1000 // 1 second between retries
      let lastError: Error | null = null

      for (let attempt = 1; attempt <= maxRetries; attempt++) {
        try {
          if (!chatId) {
            // Determine role based on current mode (backend uses role to set mode correctly)
            const role = mode === 'build' ? 'builder' : 'planner'
            const response = await apiClientRef.current.post<CreateChatResponse>(
              `/workspaces/${workspaceId}/chats`,
              { goal: content, model: model.id, role } as CreateChatRequest
            )
            if (!response?.chat_id) throw new Error('Invalid server response')
            setChatId(response.chat_id)
            onChatCreatedRef.current?.(response.chat_id)
            // Immediately add to recent chats for instant tab appearance
            addRecentChatOptimistic(response.chat_id)

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

            hasReceivedStreamingEventRef.current = false
            // Update per-chat streaming state
            setChatSessions((prev) => {
              const newSessions = new Map(prev)
              if (chatId) {
                const session = newSessions.get(chatId)
                if (session) {
                  newSessions.set(chatId, { ...session, isStreaming: false })
                }
              }
              return newSessions
            })

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
    [workspaceId, chatId, connectToSse, model, addRecentChatOptimistic]
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
        // Update local state immediately
        setModeState(newMode)
        if (newPlanFile !== undefined) {
          setPlanFileState(newPlanFile)
        }

        // Only update on server if chat exists
        if (chatId) {
          await apiClientRef.current.patch(
            `/workspaces/${workspaceId}/chats/${chatId}`,
            {
              app_data: {
                mode: newMode,
                plan_file: newPlanFile || null
              }
            }
          )
        }

        const modeLabel = newMode === 'plan' ? 'Plan' : 'Build'
        toast.success(`Switched to ${modeLabel} Mode`)
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error'
        toast.error(`Failed to change mode: ${errorMessage}`)
        console.error('[Chat] Set mode error', error)
      }
    },
    [workspaceId, chatId]
  )

  // ============================================================================
  // Multi-Chat Functions
  // ============================================================================

  // Refresh the list of recent chats
  const refreshRecentChats = React.useCallback(async () => {
    try {
      console.log('[Chat] Loading recent chats for workspace:', workspaceId)
      const result = await apiClientRef.current.get<ChatFile[]>(
        `/workspaces/${workspaceId}/chats`
      )
      if (result) {
        setRecentChats(result)
      }
    } catch (error) {
      console.error('[Chat] Failed to load recent chats:', error)
    }
  }, [workspaceId])

  // Load recent chats on mount
  React.useEffect(() => {
    refreshRecentChats()
  }, [refreshRecentChats])

  // Switch to a different chat (client-side, no router navigation)
  const switchToChat = React.useCallback(
    async (targetChatId: string) => {
      console.log('[Chat] switchToChat called', { targetChatId, activeChatId })

      if (targetChatId === activeChatId) {
        console.log('[Chat] Already on this chat, skipping')
        return
      }

      // Save current chat state to cache
      if (activeChatId) {
        setChatSessions((prev) => {
          const newSessions = new Map(prev)
          newSessions.set(activeChatId, {
            chatId: activeChatId,
            messages,
            isStreaming,
            isLoading,
            model,
            mode,
            planFile,
            pendingQuestionSession,
            lastAccessedAt: Date.now(),
          })
          return newSessions
        })
      }

      // Check if target chat is already cached
      const cachedSession = chatSessions.get(targetChatId)
      if (cachedSession) {
        // Load from cache - instant switch
        console.log('[Chat] Loading chat from cache:', targetChatId)

        setActiveChatId(targetChatId)
        setChatId(targetChatId)
        setMessages(cachedSession.messages)
        // Note: isStreaming is now derived from chatSessions, no need to set it
        setIsLoading(cachedSession.isLoading)
        setModel(cachedSession.model)
        setModeState(cachedSession.mode)
        setPlanFileState(cachedSession.planFile)
        setPendingQuestionSession(cachedSession.pendingQuestionSession)

        // Connect to SSE for this chat (will skip if already connected)
        console.log('[Chat] Connecting to SSE for cached chat:', targetChatId)
        connectToSse(targetChatId)
      } else {
        // Not cached - load from API
        console.log('[Chat] Loading chat from API:', targetChatId)

        setActiveChatId(targetChatId)
        setChatId(targetChatId)
        // The initChat effect will handle loading messages
      }
    },
    [
      activeChatId,
      chatSessions,
      messages,
      isLoading,
      model,
      mode,
      planFile,
      pendingQuestionSession,
      connectToSse,
      setChatId,
    ]
  )

  // Handle initial chat from URL (one-time sync for deep linking)
  React.useEffect(() => {
    if (initialChatId && !initialChatLoadedRef.current) {
      setActiveChatId(initialChatId)
      setChatId(initialChatId)
      initialChatLoadedRef.current = true
    }
  }, [initialChatId, setChatId])

  // Derive isStreaming from the current chat's session state
  // This ensures the button shows the correct state for the active chat
  const isStreaming = React.useMemo(() => {
    if (!chatId) return false
    const session = chatSessions.get(chatId)
    return session?.isStreaming ?? false
  }, [chatId, chatSessions])

  const value = React.useMemo(
    () => ({
      messages, isStreaming, isLoading, sendMessage, stopGeneration, clearMessages, chatId, workspaceId,
      model, setModel, availableModels,
      // Plan Mode
      mode, planFile, pendingQuestionSession, currentQuestion,
      submitAnswer, dismissQuestion, setMode,
      // Multi-Chat
      activeChatId, setActiveChatId, chatSessions, switchToChat, recentChats, refreshRecentChats
    }),
    [messages, isStreaming, isLoading, sendMessage, stopGeneration, clearMessages, chatId, workspaceId, model, setModel, availableModels,
     mode, planFile, pendingQuestionSession, currentQuestion, submitAnswer, dismissQuestion, setMode,
     activeChatId, chatSessions, switchToChat, recentChats, refreshRecentChats]
  )

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>
}
