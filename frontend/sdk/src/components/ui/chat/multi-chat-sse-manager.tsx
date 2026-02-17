/**
 * MultiChatSSEManager - SSE Connection Pooling for Multi-Chat Support
 *
 * Manages multiple SSE connections simultaneously with:
 * - Connection pooling (configurable limit, default 5)
 * - LRU eviction when limit is reached
 * - Per-chat event routing via pub/sub pattern
 * - Connection health monitoring
 *
 * ## Usage
 *
 * ```tsx
 * const manager = useMultiChatSSEManager()
 *
 * // Connect to a chat's SSE stream
 * await manager.connectChat(chatId, workspaceId)
 *
 * // Subscribe to events for a specific chat
 * const unsubscribe = manager.subscribeToChat(chatId, (event) => {
 *   console.log('SSE event:', event)
 * })
 *
 * // Disconnect when done
 * manager.disconnectChat(chatId)
 * ```
 */

import * as React from 'react'

// ============================================================================
// Types
// ============================================================================

export interface SSEEvent {
  type: string
  data: any
}

export interface SSEConnection {
  chatId: string
  workspaceId: string
  controller: AbortController
  reader: ReadableStreamDefaultReader<Uint8Array> | null
  connectionId: number
  isActive: boolean
  lastEventAt: number
  connectingAt: number
}

export interface SSEConnectionState {
  chatId: string
  workspaceId: string
  status: 'disconnected' | 'connecting' | 'connected' | 'error'
  lastEventAt: number
}

interface MultiChatSSEManagerValue {
  connectChat: (chatId: string, workspaceId: string, onEvent: (event: SSEEvent) => void) => Promise<void>
  disconnectChat: (chatId: string) => void
  disconnectAll: () => void
  getConnectionState: (chatId: string) => SSEConnectionState | undefined
  getAllConnectionStates: () => SSEConnectionState[]
  getConnectedChatIds: () => string[]
}

const MultiChatSSEManagerContext = React.createContext<MultiChatSSEManagerValue | null>(null)

// ============================================================================
// Configuration
// ============================================================================

const MAX_CONNECTIONS = 5
const CONNECTION_TIMEOUT_MS = 30000 // 30 seconds
const HEALTH_CHECK_INTERVAL_MS = 10000 // 10 seconds

// ============================================================================
// Provider Component
// ============================================================================

export function MultiChatSSEManagerProvider({
  children,
  maxConnections = MAX_CONNECTIONS,
}: {
  children: React.ReactNode
  maxConnections?: number
}) {
  const connectionsRef = React.useRef<Map<string, SSEConnection>>(new Map())
  const subscribersRef = React.useRef<Map<string, Set<(event: SSEEvent) => void>>>(new Map())
  const connectionStatesRef = React.useRef<Map<string, SSEConnectionState>>(new Map())
  const accessOrderRef = React.useRef<string[]>([]) // For LRU eviction
  const connectionIdCounterRef = React.useRef(0)
  const apiClientRef = React.useRef<any>(null)

  // Set up health check interval
  React.useEffect(() => {
    const interval = setInterval(() => {
      const now = Date.now()
      for (const [chatId, connection] of connectionsRef.current) {
        const timeSinceLastEvent = now - connection.lastEventAt
        const timeSinceConnecting = now - connection.connectingAt

        // Mark as error if connecting for too long without events
        if (connection.isActive && timeSinceLastEvent > CONNECTION_TIMEOUT_MS && timeSinceConnecting > CONNECTION_TIMEOUT_MS) {
          const state = connectionStatesRef.current.get(chatId)
          if (state) {
            connectionStatesRef.current.set(chatId, {
              ...state,
              status: 'error',
            })
          }
        }
      }
    }, HEALTH_CHECK_INTERVAL_MS)

    return () => clearInterval(interval)
  }, [])

  // LRU eviction: disconnect least recently used connection
  const evictLRUConnection = React.useCallback(() => {
    if (accessOrderRef.current.length <= maxConnections) return

    // Find the least recently used chat (excluding the one we're about to connect)
    for (const chatId of accessOrderRef.current) {
      const connection = connectionsRef.current.get(chatId)
      if (connection && chatId !== accessOrderRef.current[accessOrderRef.current.length - 1]) {
        console.log(`[MultiChatSSEManager] Evicting LRU connection: ${chatId}`)
        disconnectChatInternal(chatId)
        accessOrderRef.current = accessOrderRef.current.filter((id) => id !== chatId)
        break
      }
    }
  }, [maxConnections])

  const disconnectChatInternal = React.useCallback((chatId: string) => {
    const connection = connectionsRef.current.get(chatId)
    if (connection) {
      connection.controller.abort()
      if (connection.reader) {
        connection.reader.releaseLock()
      }
      connectionsRef.current.delete(chatId)

      const state = connectionStatesRef.current.get(chatId)
      if (state) {
        connectionStatesRef.current.set(chatId, {
          ...state,
          status: 'disconnected',
        })
      }
    }
  }, [])

  const connectChat = React.useCallback(
    async (chatId: string, workspaceId: string, onEvent: (event: SSEEvent) => void) => {
      // Update access order for LRU
      accessOrderRef.current = accessOrderRef.current.filter((id) => id !== chatId)
      accessOrderRef.current.push(chatId)

      // Check if already connected
      const existingConnection = connectionsRef.current.get(chatId)
      if (existingConnection && existingConnection.isActive) {
        console.log(`[MultiChatSSEManager] Already connected to ${chatId}, reusing connection`)
        // Add subscriber
        if (!subscribersRef.current.has(chatId)) {
          subscribersRef.current.set(chatId, new Set())
        }
        subscribersRef.current.get(chatId)!.add(onEvent)
        return
      }

      // Evict LRU connection if at limit
      evictLRUConnection()

      // Update state to connecting
      connectionStatesRef.current.set(chatId, {
        chatId,
        workspaceId,
        status: 'connecting',
        lastEventAt: Date.now(),
      })

      const connectionId = ++connectionIdCounterRef.current
      const controller = new AbortController()

      try {
        if (!apiClientRef.current) {
          throw new Error('API client not set. Make sure ChatProvider is initialized.')
        }

        const response = await apiClientRef.current.requestRaw(
          `/workspaces/${workspaceId}/chats/${chatId}/events`,
          {
            headers: { Accept: 'text/event-stream' },
            signal: controller.signal,
            timeout: false,
          }
        )

        if (!response.ok) {
          throw new Error(`SSE connection failed: ${response.statusText}`)
        }

        const reader = response.body?.getReader()
        if (!reader) {
          throw new Error('No reader available')
        }

        const connection: SSEConnection = {
          chatId,
          workspaceId,
          controller,
          reader,
          connectionId,
          isActive: true,
          lastEventAt: Date.now(),
          connectingAt: Date.now(),
        }

        connectionsRef.current.set(chatId, connection)
        connectionStatesRef.current.set(chatId, {
          chatId,
          workspaceId,
          status: 'connected',
          lastEventAt: Date.now(),
        })

        // Add subscriber
        if (!subscribersRef.current.has(chatId)) {
          subscribersRef.current.set(chatId, new Set())
        }
        subscribersRef.current.get(chatId)!.add(onEvent)

        console.log(`[MultiChatSSEManager] Connected to ${chatId} (connId: ${connectionId})`)

        // Start reading SSE stream
        const decoder = new TextDecoder()
        let buffer = ''

        const readStream = async () => {
          try {
            while (true) {
              const { done, value } = await reader.read()

              if (done) {
                console.log(`[MultiChatSSEManager] Stream closed for ${chatId}`)
                break
              }

              // Check if connection was aborted
              const currentConnection = connectionsRef.current.get(chatId)
              if (!currentConnection || currentConnection.connectionId !== connectionId) {
                console.log(`[MultiChatSSEManager] Connection ${chatId} was replaced, stopping reader`)
                break
              }

              // Update last event time
              if (currentConnection) {
                currentConnection.lastEventAt = Date.now()
                const state = connectionStatesRef.current.get(chatId)
                if (state) {
                  connectionStatesRef.current.set(chatId, {
                    ...state,
                    lastEventAt: Date.now(),
                  })
                }
              }

              buffer += decoder.decode(value, { stream: true })
              const lines = buffer.split('\n\n')
              buffer = lines.pop() || ''

              for (const line of lines) {
                const parts = line.split('\n')
                let eventType = 'chunk'
                let dataStr = ''

                for (const p of parts) {
                  if (p.startsWith('event: ')) {
                    eventType = p.slice(7).trim()
                  } else if (p.startsWith('data: ')) {
                    dataStr = p.slice(6).trim()
                  }
                }

                if (!dataStr) continue

                try {
                  const payload = JSON.parse(dataStr)
                  const type = payload.type || eventType
                  const data = payload.data || payload

                  if (type === 'ping') continue

                  // Emit event to subscribers
                  const subscribers = subscribersRef.current.get(chatId)
                  if (subscribers) {
                    const event: SSEEvent = { type, data }
                    subscribers.forEach((callback) => {
                      try {
                        callback(event)
                      } catch (error) {
                        console.error(`[MultiChatSSEManager] Error in subscriber callback:`, error)
                      }
                    })
                  }
                } catch (e) {
                  console.error(`[MultiChatSSEManager] Parse error for ${chatId}:`, e)
                }
              }
            }
          } catch (error) {
            if ((error as Error).name === 'AbortError') {
              console.log(`[MultiChatSSEManager] Connection ${chatId} aborted`)
              return
            }
            console.error(`[MultiChatSSEManager] Stream error for ${chatId}:`, error)

            const state = connectionStatesRef.current.get(chatId)
            if (state) {
              connectionStatesRef.current.set(chatId, {
                ...state,
                status: 'error',
              })
            }
          } finally {
            reader.releaseLock()
            connectionsRef.current.delete(chatId)

            const state = connectionStatesRef.current.get(chatId)
            if (state) {
              connectionStatesRef.current.set(chatId, {
                ...state,
                status: 'disconnected',
              })
            }
          }
        }

        readStream()
      } catch (error) {
        console.error(`[MultiChatSSEManager] Failed to connect to ${chatId}:`, error)

        controller.abort()
        connectionsRef.current.delete(chatId)

        const state = connectionStatesRef.current.get(chatId)
        if (state) {
          connectionStatesRef.current.set(chatId, {
            ...state,
            status: 'error',
          })
        }

        throw error
      }
    },
    [evictLRUConnection]
  )

  const disconnectChat = React.useCallback((chatId: string) => {
    disconnectChatInternal(chatId)
    subscribersRef.current.delete(chatId)
    accessOrderRef.current = accessOrderRef.current.filter((id) => id !== chatId)
  }, [disconnectChatInternal])

  const disconnectAll = React.useCallback(() => {
    for (const chatId of connectionsRef.current.keys()) {
      disconnectChatInternal(chatId)
    }
    connectionsRef.current.clear()
    subscribersRef.current.clear()
    accessOrderRef.current = []
  }, [disconnectChatInternal])

  const getConnectionState = React.useCallback(
    (chatId: string): SSEConnectionState | undefined => {
      return connectionStatesRef.current.get(chatId)
    },
    []
  )

  const getAllConnectionStates = React.useCallback((): SSEConnectionState[] => {
    return Array.from(connectionStatesRef.current.values())
  }, [])

  const getConnectedChatIds = React.useCallback((): string[] => {
    return Array.from(connectionsRef.current.keys())
  }, [])

  const value = React.useMemo(
    () => ({
      connectChat,
      disconnectChat,
      disconnectAll,
      getConnectionState,
      getAllConnectionStates,
      getConnectedChatIds,
    }),
    [connectChat, disconnectChat, disconnectAll, getConnectionState, getAllConnectionStates, getConnectedChatIds]
  )

  return <MultiChatSSEManagerContext.Provider value={value}>{children}</MultiChatSSEManagerContext.Provider>
}

// ============================================================================
// Hook
// ============================================================================

export function useMultiChatSSEManager(): MultiChatSSEManagerValue {
  const context = React.useContext(MultiChatSSEManagerContext)
  if (!context) {
    throw new Error('useMultiChatSSEManager must be used within MultiChatSSEManagerProvider')
  }
  return context
}

