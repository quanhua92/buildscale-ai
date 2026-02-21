/**
 * Workspace Route - Main workspace page with agent sessions navigation
 *
 * Provides a workspace layout with:
 * - Sidebar navigation with active agents
 * - Main content area for workspace content
 * - AgentSessionsProvider for session management
 *
 * ## Usage
 *
 * This route is accessed via /workspaces/:workspaceId
 */

import { createFileRoute, Outlet } from '@tanstack/react-router'
import { AgentSessionsProvider } from '@buildscale/sdk'
import { WorkspaceNavigation } from '../components/WorkspaceNavigation'

export const Route = createFileRoute('/workspaces/$workspaceId')({
  component: WorkspacePage,
})

function WorkspacePage() {
  const { workspaceId } = Route.useParams()

  return (
    <AgentSessionsProvider workspaceId={workspaceId}>
      <div className="flex h-screen overflow-hidden">
        {/* Sidebar with navigation */}
        <aside className="w-64 border-r bg-background flex-shrink-0">
          <WorkspaceNavigation workspaceId={workspaceId} />
        </aside>

        {/* Main content area */}
        <main className="flex-1 overflow-auto">
          <Outlet />
        </main>
      </div>
    </AgentSessionsProvider>
  )
}
