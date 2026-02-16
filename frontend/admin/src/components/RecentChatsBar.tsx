/**
 * RecentChatsBar Component
 *
 * Displays recent chats as a horizontally scrollable bar with buttons,
 * allowing users to quickly switch between chats like terminal tabs.
 * Each chat shows a status dot indicating the agent session state.
 */

import { useState, useEffect, useMemo } from 'react'
import { Link } from '@tanstack/react-router'
import { useAuth, useAgentSessions, AgentStatusIndicator } from '@buildscale/sdk'
import type { ChatFile } from '@buildscale/sdk'

interface RecentChatsBarProps {
  workspaceId: string
  currentChatId?: string
}

const formatTimeAgo = (date: Date): string => {
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 1) return 'just now'
  if (diffMins < 60) return `${diffMins}m`
  if (diffHours < 24) return `${diffHours}h`
  if (diffDays === 1) return '1d'
  return `${diffDays}d`
}

export function RecentChatsBar({ workspaceId, currentChatId }: RecentChatsBarProps) {
  const { getRecentChats } = useAuth()
  const { sessions } = useAgentSessions()
  const [chats, setChats] = useState<ChatFile[]>([])
  const [loading, setLoading] = useState(true)

  // Load recent chats
  useEffect(() => {
    const loadChats = async () => {
      setLoading(true)
      try {
        const result = await getRecentChats(workspaceId)
        if (result.success) {
          setChats(result.data || [])
        }
      } catch {
        // Silently fail - the bar just won't show any chats
      } finally {
        setLoading(false)
      }
    }

    loadChats()
  }, [workspaceId, getRecentChats])

  // Sort chats by updated_at descending (most recent first)
  const sortedChats = useMemo(() => {
    return [...chats].sort((a, b) => {
      const dateA = new Date(a.updated_at).getTime()
      const dateB = new Date(b.updated_at).getTime()
      return dateB - dateA // Most recent first
    })
  }, [chats])

  // Create a map of chat_id to most recent session status
  const chatStatusMap = useMemo(() => {
    const map = new Map<string, string>()
    sessions.forEach((session) => {
      // If a chat has multiple sessions, use the most recent one
      const existing = map.get(session.chat_id)
      if (!existing || new Date(session.updated_at) > new Date(existing)) {
        map.set(session.chat_id, session.status)
      }
    })
    return map
  }, [sessions])

  // Don't render anything while loading or if no chats
  if (loading || chats.length === 0) {
    return null
  }

  return (
    <div className="flex items-center gap-1 px-4 border-b border-border bg-muted/30 overflow-x-auto no-scrollbar">
      {sortedChats.map((chat) => {
        const isActive = chat.chat_id === currentChatId
        const timeAgo = formatTimeAgo(new Date(chat.updated_at))
        const sessionStatus = chatStatusMap.get(chat.chat_id) as Parameters<typeof AgentStatusIndicator>[0]['status'] || undefined

        return (
          <Link
            key={chat.chat_id}
            to="/workspaces/$workspaceId/chat"
            params={{ workspaceId }}
            search={{ chatId: chat.chat_id }}
            className={`
              flex items-center gap-2 px-3 py-2 text-sm font-medium
              border-b-2 transition-colors whitespace-nowrap
              ${isActive
                ? 'border-primary text-foreground'
                : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/50'
              }
            `}
            title={`${chat.name || 'Untitled Chat'} • ${timeAgo}${sessionStatus ? ` • ${sessionStatus}` : ''}`}
          >
            {sessionStatus ? (
              <AgentStatusIndicator status={sessionStatus} size="sm" />
            ) : (
              <div className="h-2 w-2 rounded-full bg-muted-foreground/30" />
            )}
            <span className="truncate max-w-[120px] sm:max-w-[200px]">
              {chat.name || 'Untitled Chat'}
            </span>
          </Link>
        )
      })}
    </div>
  )
}
