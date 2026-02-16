/**
 * RecentChats Component
 *
 * Displays recent chats in a ChatGPT/Gemini-style sidebar with:
 * - Time-based grouping (Today, Yesterday, Previous 7 Days, Older)
 * - Hover states for quick actions
 * - Click to navigate to chat
 * - Loading and error states
 */

import { useState, useEffect, useMemo } from 'react'
import { Link } from '@tanstack/react-router'
import { MessageSquare, Clock, Trash2 } from 'lucide-react'
import { useAuth } from '@buildscale/sdk'
import type { ChatFile } from '@buildscale/sdk'

interface RecentChatsProps {
  workspaceId: string
  currentChatId?: string
}

interface ChatGroup {
  label: string
  chats: ChatFile[]
}

// Helper functions for date grouping (avoiding date-fns dependency)
const isToday = (date: Date): boolean => {
  const today = new Date()
  return date.getDate() === today.getDate() &&
    date.getMonth() === today.getMonth() &&
    date.getFullYear() === today.getFullYear()
}

const isYesterday = (date: Date): boolean => {
  const yesterday = new Date()
  yesterday.setDate(yesterday.getDate() - 1)
  return date.getDate() === yesterday.getDate() &&
    date.getMonth() === yesterday.getMonth() &&
    date.getFullYear() === yesterday.getFullYear()
}

const isWithinLast7Days = (date: Date): boolean => {
  const sevenDaysAgo = new Date()
  sevenDaysAgo.setDate(sevenDaysAgo.getDate() - 7)
  return date > sevenDaysAgo && !isToday(date) && !isYesterday(date)
}

const formatTimeAgo = (date: Date): string => {
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 1) return 'just now'
  if (diffMins < 60) return `${diffMins}m ago`
  if (diffHours < 24) return `${diffHours}h ago`
  if (diffDays === 1) return 'yesterday'
  if (diffDays < 7) return `${diffDays}d ago`
  return date.toLocaleDateString()
}

export function RecentChats({ workspaceId, currentChatId }: RecentChatsProps) {
  const { getRecentChats } = useAuth()
  const [chats, setChats] = useState<ChatFile[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set(['Today']))

  // Load recent chats
  useEffect(() => {
    const loadChats = async () => {
      setLoading(true)
      setError(null)
      try {
        const result = await getRecentChats(workspaceId)
        if (result.success) {
          setChats(result.data || [])
        } else {
          setError(result.error?.message || 'Failed to load chats')
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load chats')
      } finally {
        setLoading(false)
      }
    }

    loadChats()
  }, [workspaceId, getRecentChats])

  // Group chats by time period
  const chatGroups = useMemo((): ChatGroup[] => {
    const groups: Record<string, ChatFile[]> = {
      Today: [],
      Yesterday: [],
      'Previous 7 Days': [],
      Older: [],
    }

    chats.forEach(chat => {
      const updatedDate = new Date(chat.updated_at)
      if (isToday(updatedDate)) {
        groups.Today.push(chat)
      } else if (isYesterday(updatedDate)) {
        groups.Yesterday.push(chat)
      } else if (isWithinLast7Days(updatedDate)) {
        groups['Previous 7 Days'].push(chat)
      } else {
        groups.Older.push(chat)
      }
    })

    // Sort each group by updated_at descending (most recent first)
    Object.values(groups).forEach(groupChats => {
      groupChats.sort((a, b) => {
        const dateA = new Date(a.updated_at).getTime()
        const dateB = new Date(b.updated_at).getTime()
        return dateB - dateA // Most recent first
      })
    })

    // Convert to array and filter out empty groups
    return Object.entries(groups)
      .filter(([_, chats]) => chats.length > 0)
      .map(([label, chats]) => ({ label, chats }))
  }, [chats])

  // Toggle group expansion
  const toggleGroup = (label: string) => {
    setExpandedGroups(prev => {
      const next = new Set(prev)
      if (next.has(label)) {
        next.delete(label)
      } else {
        next.add(label)
      }
      return next
    })
  }

  if (loading) {
    return (
      <div className="px-3 py-2">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <MessageSquare className="h-4 w-4" />
          <span>Loading chats...</span>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="px-3 py-2">
        <div className="flex items-center gap-2 text-sm text-destructive">
          <span>Failed to load chats: {error}</span>
        </div>
      </div>
    )
  }

  if (chats.length === 0) {
    return (
      <div className="px-3 py-2">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <MessageSquare className="h-4 w-4" />
          <span>No recent chats</span>
        </div>
      </div>
    )
  }

  return (
    <div className="px-2 py-1">
      {chatGroups.map(group => (
        <div key={group.label} className="mb-2">
          {/* Group header */}
          <button
            onClick={() => toggleGroup(group.label)}
            className="flex items-center gap-1 w-full px-2 py-1 text-xs font-semibold text-muted-foreground hover:text-foreground transition-colors"
          >
            <Clock className="h-3 w-3" />
            <span>{group.label}</span>
            <span className="ml-auto text-muted-foreground/60">
              {expandedGroups.has(group.label) ? '▼' : '▶'}
            </span>
          </button>

          {/* Chat items */}
          {expandedGroups.has(group.label) && (
            <div className="mt-1 space-y-0.5">
              {group.chats.map(chat => {
                const isActive = chat.chat_id === currentChatId
                const timeAgo = formatTimeAgo(new Date(chat.updated_at))

                return (
                  <div
                    key={chat.chat_id}
                    className="group relative"
                  >
                    <Link
                      to="/workspaces/$workspaceId/chat"
                      params={{ workspaceId }}
                      search={{ chatId: chat.chat_id }}
                      className={`
                        flex items-start gap-2 px-2 py-1.5 rounded-md text-sm
                        transition-colors
                        ${isActive
                          ? 'bg-accent text-accent-foreground font-medium'
                          : 'hover:bg-accent/50 text-muted-foreground hover:text-foreground'
                        }
                      `}
                    >
                      <MessageSquare className="h-4 w-4 mt-0.5 flex-shrink-0" />
                      <div className="flex-1 min-w-0">
                        <div className="truncate font-medium">
                          {chat.name || 'Untitled Chat'}
                        </div>
                        <div className="text-xs text-muted-foreground truncate">
                          {timeAgo}
                        </div>
                      </div>

                      {/* Quick actions on hover */}
                      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                        <button
                          onClick={(e) => {
                            e.preventDefault()
                            // TODO: Implement delete functionality
                            console.log('Delete chat:', chat.chat_id)
                          }}
                          className="p-1 hover:bg-destructive/10 hover:text-destructive rounded transition-colors"
                          title="Delete chat"
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      </div>
                    </Link>
                  </div>
                )
              })}
            </div>
          )}
        </div>
      ))}
    </div>
  )
}
