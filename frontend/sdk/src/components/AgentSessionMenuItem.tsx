/**
 * AgentSessionMenuItem - Navigation menu item for agent sessions
 *
 * Displays an agent session as a navigation item with:
 * - Status indicator
 * - Agent type and model
 * - Current task (truncated)
 * - Quick actions (pause/resume/cancel) on hover
 *
 * ## Usage
 *
 * ```tsx
 * <AgentSessionMenuItem
 *   session={session}
 *   onPause={() => pauseSession(session.id)}
 *   onResume={() => resumeSession(session.id)}
 *   onCancel={() => cancelSession(session.id)}
 * />
 * ```
 */

import { Link } from '@tanstack/react-router'
import { Pause, Play, X } from 'lucide-react'
import type { AgentSession } from '../api/types'
import { AgentStatusIndicator } from './AgentStatusIndicator'
import { Button } from './ui/button'
import { cn } from '../utils'

export interface AgentSessionMenuItemProps {
  session: AgentSession
  onPause: () => void
  onResume: () => void
  onCancel: () => void
  className?: string
}

export function AgentSessionMenuItem({
  session,
  onPause,
  onResume,
  onCancel,
  className,
}: AgentSessionMenuItemProps) {
  // Determine which action buttons to show based on status
  const canPause = session.status === 'running' || session.status === 'idle'
  const canResume = session.status === 'paused'

  return (
    <div className={cn('group relative', className)}>
      <Link
        to={`/workspaces/${session.workspace_id}/chats/${session.chat_id}`}
        className="flex items-center gap-3 px-3 py-2 w-full rounded-md hover:bg-accent hover:text-accent-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        {/* Status indicator */}
        <AgentStatusIndicator status={session.status} size="sm" />

        {/* Agent info */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm capitalize">
              {session.agent_type}
            </span>
            <span className="text-xs text-muted-foreground">
              {session.model}
            </span>
          </div>

          {/* Current task (truncated) */}
          {session.current_task && (
            <p className="text-xs text-muted-foreground truncate" title={session.current_task}>
              {session.current_task}
            </p>
          )}

          {/* No task message */}
          {!session.current_task && session.status === 'idle' && (
            <p className="text-xs text-muted-foreground italic">
              Waiting for task...
            </p>
          )}
        </div>

        {/* Quick actions (show on hover) */}
        <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          {canPause && (
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={(e) => {
                e.preventDefault()
                onPause()
              }}
              title="Pause session"
            >
              <Pause className="h-3 w-3" />
            </Button>
          )}
          {canResume && (
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={(e) => {
                e.preventDefault()
                onResume()
              }}
              title="Resume session"
            >
              <Play className="h-3 w-3" />
            </Button>
          )}
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 text-destructive hover:text-destructive"
            onClick={(e) => {
              e.preventDefault()
              onCancel()
            }}
            title="Cancel session"
          >
            <X className="h-3 w-3" />
          </Button>
        </div>
      </Link>
    </div>
  )
}
