import { useState, useEffect, useCallback } from 'react'
import type { ChatMessage } from '@buildscale/sdk'

// Default values for message polling
const DEFAULT_POLL_INTERVAL_MS = 2000
const DEFAULT_MAX_MESSAGES = 3

interface UseSessionMessagesOptions {
  apiClient: {
    getChat: (workspaceId: string, chatId: string) => Promise<{ messages: ChatMessage[] }>
  }
  workspaceId: string
  expandedSessionIds: Set<string>
  sessions: Array<{ id: string; chat_id: string }>
  pollInterval?: number
  maxMessages?: number
}

interface SessionMessagesState {
  messages: Map<string, ChatMessage[]>
  loading: Set<string>
  errors: Map<string, string>
}

/**
 * Hook to fetch and poll messages for expanded agent sessions.
 * Fetches initial messages on expansion and polls for updates at the specified interval.
 */
export function useSessionMessages({
  apiClient,
  workspaceId,
  expandedSessionIds,
  sessions,
  pollInterval = DEFAULT_POLL_INTERVAL_MS,
  maxMessages = DEFAULT_MAX_MESSAGES,
}: UseSessionMessagesOptions) {
  const [state, setState] = useState<SessionMessagesState>({
    messages: new Map(),
    loading: new Set(),
    errors: new Map(),
  })

  const fetchMessages = useCallback(async () => {
    if (expandedSessionIds.size === 0) return

    // Mark new sessions as loading
    setState((prev) => {
      const newLoading = new Set(prev.loading)
      for (const sessionId of expandedSessionIds) {
        if (!prev.messages.has(sessionId) && !prev.errors.has(sessionId)) {
          newLoading.add(sessionId)
        }
      }
      return { ...prev, loading: newLoading }
    })

    // Fetch messages for each expanded session
    for (const sessionId of expandedSessionIds) {
      const session = sessions.find((s) => s.id === sessionId)
      if (!session) continue

      try {
        const chat = await apiClient.getChat(workspaceId, session.chat_id)
        const recentMessages = chat.messages.slice(-maxMessages)

        setState((prev) => {
          const next = {
            messages: new Map(prev.messages),
            loading: new Set(prev.loading),
            errors: new Map(prev.errors),
          }
          next.messages.set(sessionId, recentMessages)
          next.loading.delete(sessionId)
          next.errors.delete(sessionId)
          return next
        })
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to fetch messages'
        setState((prev) => {
          const next = {
            messages: new Map(prev.messages),
            loading: new Set(prev.loading),
            errors: new Map(prev.errors),
          }
          next.loading.delete(sessionId)
          next.errors.set(sessionId, errorMessage)
          return next
        })
      }
    }
  }, [apiClient, workspaceId, expandedSessionIds, sessions, maxMessages])

  // Poll for message updates
  useEffect(() => {
    if (expandedSessionIds.size === 0) {
      // Clear messages for collapsed sessions to free memory
      setState((prev) => {
        const next = new Map(prev.messages)
        for (const key of next.keys()) {
          if (!expandedSessionIds.has(key)) {
            next.delete(key)
          }
        }
        return { ...prev, messages: next }
      })
      return
    }

    fetchMessages() // Initial fetch
    const interval = setInterval(fetchMessages, pollInterval)

    return () => clearInterval(interval)
  }, [expandedSessionIds, fetchMessages, pollInterval])

  return {
    messages: state.messages,
    loading: state.loading,
    errors: state.errors,
  }
}
