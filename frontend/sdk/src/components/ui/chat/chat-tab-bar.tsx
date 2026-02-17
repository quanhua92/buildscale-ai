/**
 * ChatTabBar - Native tab bar component for multi-chat support
 *
 * Displays chat tabs in a horizontal scrollable bar (browser-tab style) with:
 * - Status indicator dots for each chat
 * - Active tab highlighting with primary color bottom border
 * - Smooth transitions without router navigation
 * - Connection health indicators
 *
 * ## Usage
 *
 * ```tsx
 * const tabs = [
 *   { chatId: 'abc', name: 'Chat 1', status: 'streaming', connectionHealth: 'connected' },
 *   { chatId: 'def', name: 'Chat 2', status: 'idle', connectionHealth: 'connected' },
 * ]
 *
 * <ChatTabBar
 *   tabs={tabs}
 *   activeTabId={activeChatId}
 *   onTabClick={(chatId) => switchToChat(chatId)}
 * />
 * ```
 */

import { X } from 'lucide-react'
import { cn } from '../../../utils'
import { AgentStatusIndicator } from '../../AgentStatusIndicator'
import type { SessionStatus } from '../../../api/types'

// ============================================================================
// Types
// ============================================================================

export type ChatTabStatus = 'idle' | 'streaming' | 'error'

export interface ChatTab {
  chatId: string
  name: string
  status: ChatTabStatus
  sessionStatus?: SessionStatus
}

export interface ChatTabBarProps {
  tabs: ChatTab[]
  activeTabId: string | null
  onTabClick: (chatId: string) => void
  onTabClose?: (chatId: string) => void
  className?: string
}

// ============================================================================
// Component
// ============================================================================

export function ChatTabBar({ tabs, activeTabId, onTabClick, onTabClose, className }: ChatTabBarProps) {
  if (tabs.length === 0) {
    return null
  }

  return (
    <div
      className={cn(
        'flex items-center gap-1 px-2 border-b border-border bg-muted/30 overflow-x-auto no-scrollbar',
        className
      )}
    >
      {tabs.map((tab) => {
        const isActive = tab.chatId === activeTabId

        return (
          <button
            key={tab.chatId}
            onClick={() => onTabClick(tab.chatId)}
            className={cn(
              'group flex items-center gap-2 px-4 py-2 text-sm font-medium border-b-2 transition-colors whitespace-nowrap',
              'w-48', // Fixed width of 192px (w-48)
              isActive
                ? 'border-primary text-foreground bg-accent'
                : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/50'
            )}
            title={tab.name || 'Untitled Chat'}
          >
            {/* Status indicator */}
            {tab.sessionStatus ? (
              <AgentStatusIndicator status={tab.sessionStatus} size="sm" />
            ) : (
              <div
                className={cn(
                  'h-2 w-2 rounded-full flex-shrink-0',
                  tab.status === 'streaming' ? 'bg-green-500 animate-pulse' : 'bg-muted-foreground/30'
                )}
              />
            )}

            {/* Tab name - with truncation */}
            <span className="truncate flex-1">{tab.name || 'Untitled Chat'}</span>

            {/* Close button (on hover) */}
            {onTabClose && (
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  onTabClose(tab.chatId)
                }}
                className={cn(
                  'flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity',
                  'p-0.5 rounded-sm hover:bg-muted-foreground/20',
                  'focus:opacity-100 focus:outline-none'
                )}
                aria-label={`Close ${tab.name || 'chat'}`}
              >
                <X className="h-3 w-3" />
              </button>
            )}
          </button>
        )
      })}
    </div>
  )
}
