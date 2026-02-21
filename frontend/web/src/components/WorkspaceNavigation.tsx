/**
 * WorkspaceNavigation - Workspace-specific navigation with agent sessions
 *
 * Provides navigation menu for a workspace with:
 * - Active agents section with session management
 * - Recent chats section
 * - Settings and other workspace links
 *
 * ## Usage
 *
 * ```tsx
 * <WorkspaceNavigation workspaceId="workspace-123" />
 * ```
 */

import { useState } from 'react'
import { Home, Settings } from 'lucide-react'
import {
  NavigationMenu,
  useAgentSessions,
  AgentSessionMenuItem,
} from '@buildscale/sdk'

export interface WorkspaceNavigationProps {
  workspaceId: string
}

export function WorkspaceNavigation({ workspaceId }: WorkspaceNavigationProps) {
  const [isOpen, setIsOpen] = useState(false)
  const { sessions, loading, pauseSession, resumeSession, cancelSession } = useAgentSessions()

  // Filter active sessions (not completed or error)
  const activeSessions = sessions.filter(
    (s) => s.status !== 'completed' && s.status !== 'error'
  )

  return (
    <NavigationMenu open={isOpen} onOpenChange={setIsOpen} title="Workspace">
      {/* Workspace Home */}
      <NavigationMenu.Item
        to={`/workspaces/${workspaceId}`}
        icon={<Home className="h-5 w-5" />}
      >
        Overview
      </NavigationMenu.Item>

      <NavigationMenu.Separator />

      {/* Recent Chats Section */}
      <NavigationMenu.Section title="Recent Chats" defaultOpen>
        <div className="text-sm text-muted-foreground px-3 py-2">
          No recent chats
        </div>
      </NavigationMenu.Section>

      {/* Active Agents Section */}
      <NavigationMenu.Section
        title={`Active Agents${activeSessions.length > 0 ? ` (${activeSessions.length})` : ''}`}
        defaultOpen
      >
        {loading ? (
          <div className="text-sm text-muted-foreground px-3 py-2">
            Loading...
          </div>
        ) : activeSessions.length === 0 ? (
          <div className="text-sm text-muted-foreground px-3 py-2">
            No active agents
          </div>
        ) : (
          activeSessions.map((session) => (
            <AgentSessionMenuItem
              key={session.id}
              session={session}
              onPause={() => pauseSession(session.id)}
              onResume={() => resumeSession(session.id)}
              onCancel={() => cancelSession(session.id)}
            />
          ))
        )}
      </NavigationMenu.Section>

      <NavigationMenu.Separator />

      {/* Workspace Settings */}
      <NavigationMenu.Item
        to={`/workspaces/${workspaceId}/settings`}
        icon={<Settings className="h-5 w-5" />}
      >
        Settings
      </NavigationMenu.Item>
    </NavigationMenu>
  )
}
