/**
 * Workspace Index Route - Workspace overview page
 *
 * Displays workspace overview with:
 * - Active agents summary
 * - Recent activity
 * - Quick actions
 *
 * ## Usage
 *
 * This route is accessed via /workspaces/:workspaceId
 */

import { createFileRoute } from '@tanstack/react-router'
import { useAgentSessions } from '@buildscale/sdk'
import { Bot, Clock, CheckCircle, AlertCircle } from 'lucide-react'

export const Route = createFileRoute('/workspaces/$workspaceId/')({
  component: WorkspaceIndexPage,
})

function WorkspaceIndexPage() {
  const { sessions, loading, getSessionsByStatus } = useAgentSessions()

  const runningCount = getSessionsByStatus('running').length
  const idleCount = getSessionsByStatus('idle').length
  const pausedCount = getSessionsByStatus('paused').length
  const completedCount = getSessionsByStatus('completed').length

  return (
    <div className="container mx-auto py-8 px-4">
      <div className="mb-8">
        <h1 className="text-3xl font-bold">Workspace Overview</h1>
        <p className="text-muted-foreground mt-2">
          Manage your agents and monitor their activity
        </p>
      </div>

      {/* Session Stats */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
        <StatCard
          title="Running"
          count={runningCount}
          icon={<Bot className="h-4 w-4" />}
          className="border-green-500/50 bg-green-50 dark:bg-green-950/20"
        />
        <StatCard
          title="Idle"
          count={idleCount}
          icon={<Clock className="h-4 w-4" />}
          className="border-yellow-500/50 bg-yellow-50 dark:bg-yellow-950/20"
        />
        <StatCard
          title="Paused"
          count={pausedCount}
          icon={<AlertCircle className="h-4 w-4" />}
          className="border-orange-500/50 bg-orange-50 dark:bg-orange-950/20"
        />
        <StatCard
          title="Completed"
          count={completedCount}
          icon={<CheckCircle className="h-4 w-4" />}
          className="border-blue-500/50 bg-blue-50 dark:bg-blue-950/20"
        />
      </div>

      {/* Recent Sessions */}
      <div className="bg-card rounded-lg border p-6">
        <h2 className="text-xl font-semibold mb-4">Recent Sessions</h2>
        {loading ? (
          <p className="text-muted-foreground">Loading...</p>
        ) : sessions.length === 0 ? (
          <p className="text-muted-foreground">No sessions yet</p>
        ) : (
          <div className="space-y-3">
            {sessions.slice(0, 10).map((session) => (
              <div
                key={session.id}
                className="flex items-center justify-between p-3 rounded-lg border bg-muted/30 hover:bg-muted/50 transition-colors"
              >
                <div className="flex items-center gap-3">
                  <span className="font-medium capitalize text-sm">
                    {session.agent_type}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {session.model}
                  </span>
                </div>
                <div className="flex items-center gap-4">
                  <span className="text-xs capitalize text-muted-foreground">
                    {session.status}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {new Date(session.created_at).toLocaleString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function StatCard({
  title,
  count,
  icon,
  className,
}: {
  title: string
  count: number
  icon: React.ReactNode
  className?: string
}) {
  return (
    <div className={className}>
      <div className="bg-card rounded-lg border p-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-muted-foreground">{title}</p>
            <p className="text-2xl font-bold mt-1">{count}</p>
          </div>
          <div className="text-muted-foreground">{icon}</div>
        </div>
      </div>
    </div>
  )
}
