/**
 * AgentSessionsContext - Manages active agent sessions for a workspace
 *
 * Provides polling-based session state management with action methods
 * for pause, resume, and cancel. Designed for easy migration to SSE.
 *
 * ## Usage
 *
 * ```tsx
 * function App() {
 *   return (
 *     <AgentSessionsProvider workspaceId="workspace-123">
 *       <WorkspacePage />
 *     </AgentSessionsProvider>
 *   )
 * }
 *
 * function WorkspacePage() {
 *   const { sessions, loading, pauseSession } = useAgentSessions()
 *   return <div>{sessions.map(...)}</div>
 * }
 * ```
 */

import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react'
import type { AgentSession, SessionStatus } from '../api/types'
import { useAuth } from './AuthContext'

export interface AgentSessionsContextValue {
  sessions: AgentSession[]
  loading: boolean
  error: string | null
  refresh: () => Promise<void>
  pauseSession: (sessionId: string, reason?: string) => Promise<void>
  resumeSession: (sessionId: string, task?: string) => Promise<void>
  cancelSession: (sessionId: string) => Promise<void>
  getSessionById: (sessionId: string) => AgentSession | undefined
  getSessionsByChatId: (chatId: string) => AgentSession[]
  getSessionsByStatus: (status: SessionStatus) => AgentSession[]
}

const AgentSessionsContext = createContext<AgentSessionsContextValue | undefined>(undefined)

export interface AgentSessionsProviderProps {
  workspaceId: string
  children: ReactNode
  /**
   * Polling interval in milliseconds (default: 5000)
   * Set to 0 to disable polling (for SSE migration)
   */
  pollInterval?: number
}

export function AgentSessionsProvider({
  workspaceId,
  children,
  pollInterval = 5000,
}: AgentSessionsProviderProps) {
  const { apiClient } = useAuth()
  const [sessions, setSessions] = useState<AgentSession[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Fetch sessions from API
  const fetchSessions = useCallback(async () => {
    // Guard against undefined/invalid workspaceId
    if (!workspaceId || workspaceId === 'undefined') {
      console.warn('[AgentSessions] Cannot fetch sessions: workspaceId is invalid')
      return
    }

    try {
      const response = await apiClient.getWorkspaceAgentSessions(workspaceId)
      setSessions(response.sessions)
      setError(null)
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to fetch agent sessions'
      setError(message)
      console.error('[AgentSessions] Failed to fetch sessions:', err)
    } finally {
      setLoading(false)
    }
  }, [workspaceId, apiClient])

  // Manual refresh method
  const refresh = useCallback(async () => {
    await fetchSessions()
  }, [fetchSessions])

  // Poll for updates
  useEffect(() => {
    if (!workspaceId || pollInterval === 0) return

    // Initial fetch
    fetchSessions()

    const interval = setInterval(fetchSessions, pollInterval)
    return () => clearInterval(interval)
  }, [workspaceId, pollInterval, fetchSessions])

  // Pause session
  const pauseSession = useCallback(async (sessionId: string, reason?: string) => {
    try {
      await apiClient.pauseAgentSession(sessionId, reason)
      // Optimistic update - will be verified on next poll
      setSessions(prev =>
        prev.map(s =>
          s.id === sessionId ? { ...s, status: 'paused' as SessionStatus } : s
        )
      )
    } catch (err) {
      console.error('[AgentSessions] Failed to pause session:', err)
      throw err
    }
  }, [apiClient])

  // Resume session
  const resumeSession = useCallback(async (sessionId: string, task?: string) => {
    try {
      await apiClient.resumeAgentSession(sessionId, task)
      // Optimistic update - will be verified on next poll
      setSessions(prev =>
        prev.map(s =>
          s.id === sessionId ? { ...s, status: 'running' as SessionStatus } : s
        )
      )
    } catch (err) {
      console.error('[AgentSessions] Failed to resume session:', err)
      throw err
    }
  }, [apiClient])

  // Cancel session
  const cancelSession = useCallback(async (sessionId: string) => {
    try {
      await apiClient.cancelAgentSession(sessionId)
      // Remove from local list immediately
      setSessions(prev => prev.filter(s => s.id !== sessionId))
    } catch (err) {
      console.error('[AgentSessions] Failed to cancel session:', err)
      throw err
    }
  }, [apiClient])

  // Get session by ID
  const getSessionById = useCallback((sessionId: string) => {
    return sessions.find(s => s.id === sessionId)
  }, [sessions])

  // Get sessions by chat ID
  const getSessionsByChatId = useCallback((chatId: string) => {
    return sessions.filter(s => s.chat_id === chatId)
  }, [sessions])

  // Get sessions by status
  const getSessionsByStatus = useCallback((status: SessionStatus) => {
    return sessions.filter(s => s.status === status)
  }, [sessions])

  const value: AgentSessionsContextValue = {
    sessions,
    loading,
    error,
    refresh,
    pauseSession,
    resumeSession,
    cancelSession,
    getSessionById,
    getSessionsByChatId,
    getSessionsByStatus,
  }

  return (
    <AgentSessionsContext.Provider value={value}>
      {children}
    </AgentSessionsContext.Provider>
  )
}

export function useAgentSessions(): AgentSessionsContextValue {
  const context = useContext(AgentSessionsContext)
  if (context === undefined) {
    throw new Error('useAgentSessions must be used within an AgentSessionsProvider')
  }
  return context
}
