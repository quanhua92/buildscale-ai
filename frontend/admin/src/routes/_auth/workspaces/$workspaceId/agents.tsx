import { createFileRoute } from '@tanstack/react-router'
import { AgentSessionsProvider, useAgentSessions, AgentStatusIndicator, Button } from '@buildscale/sdk'
import { Pause, Play, X, Search, Clock, Bot } from 'lucide-react'
import { useState } from 'react'
import type { SessionStatus } from '@buildscale/sdk'

export const Route = createFileRoute('/_auth/workspaces/$workspaceId/agents')({
  component: AgentsRoute,
})

function AgentsRoute() {
  const { workspaceId } = Route.useParams()

  return (
    <AgentSessionsProvider workspaceId={workspaceId}>
      <AgentsContent />
    </AgentSessionsProvider>
  )
}

function AgentsContent() {
  const { sessions, loading, pauseSession, resumeSession, cancelSession } = useAgentSessions()
  const [filterStatus, setFilterStatus] = useState<SessionStatus | 'all'>('all')
  const [searchQuery, setSearchQuery] = useState('')

  const filteredSessions = sessions.filter((session) => {
    const matchesStatus = filterStatus === 'all' || session.status === filterStatus
    const matchesSearch =
      !searchQuery ||
      session.agent_type.toLowerCase().includes(searchQuery.toLowerCase()) ||
      session.model.toLowerCase().includes(searchQuery.toLowerCase()) ||
      session.current_task?.toLowerCase().includes(searchQuery.toLowerCase())
    return matchesStatus && matchesSearch
  })

  const formatTimeAgo = (timestamp: string) => {
    const now = new Date()
    const time = new Date(timestamp)
    const diff = now.getTime() - time.getTime()

    const seconds = Math.floor(diff / 1000)
    const minutes = Math.floor(seconds / 60)
    const hours = Math.floor(minutes / 60)

    if (seconds < 60) return 'just now'
    if (minutes < 60) return `${minutes}m ago`
    if (hours < 24) return `${hours}h ago`
    return `${Math.floor(hours / 24)}d ago`
  }

  const statusCounts: Record<SessionStatus | 'all', number> = {
    all: sessions.length,
    running: sessions.filter((s) => s.status === 'running').length,
    idle: sessions.filter((s) => s.status === 'idle').length,
    paused: sessions.filter((s) => s.status === 'paused').length,
    completed: sessions.filter((s) => s.status === 'completed').length,
    error: sessions.filter((s) => s.status === 'error').length,
  }

  return (
    <div className="h-[calc(100vh-var(--header-height))] p-4">
      <div className="flex flex-col h-full space-y-4 max-w-7xl mx-auto">
        {/* Header */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold">Agent Sessions</h1>
              <p className="text-muted-foreground">
                {loading ? 'Loading...' : `${sessions.length} session${sessions.length !== 1 ? 's' : ''}`}
              </p>
            </div>
          </div>

          {/* Toolbar */}
          <div className="flex flex-col sm:flex-row gap-4">
            {/* Search */}
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <input
                type="text"
                placeholder="Search agents..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-10 pr-4 py-2 border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
              />
            </div>

            {/* Status Filters */}
            <div className="flex gap-2 flex-wrap">
              {(['all', 'running', 'idle', 'paused', 'completed', 'error'] as const).map((status) => (
                <button
                  key={status}
                  onClick={() => setFilterStatus(status)}
                  className={`px-3 py-2 text-sm font-medium rounded-md transition-colors ${
                    filterStatus === status
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-muted text-muted-foreground hover:bg-muted/80'
                  }`}
                >
                  {status === 'all' ? 'All' : status.charAt(0).toUpperCase() + status.slice(1)} ({statusCounts[status]})
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Sessions List */}
        <div className="flex-1 border rounded-md overflow-auto">
          {loading ? (
            <div className="flex items-center justify-center h-full">
              <div className="text-muted-foreground">Loading sessions...</div>
            </div>
          ) : filteredSessions.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-center">
              <Bot className="h-12 w-12 text-muted-foreground mb-4" />
              <h3 className="text-lg font-medium mb-2">
                {searchQuery || filterStatus !== 'all' ? 'No matching sessions' : 'No agent sessions'}
              </h3>
              <p className="text-muted-foreground text-sm">
                {searchQuery || filterStatus !== 'all'
                  ? 'Try adjusting your search or filters'
                  : 'Agent sessions will appear here when agents are running'}
              </p>
            </div>
          ) : (
            <div className="p-4 space-y-3">
              {filteredSessions.map((session) => (
                <div
                  key={session.id}
                  className="border rounded-lg p-4 hover:bg-muted/30 transition-colors"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-3 mb-2">
                        <AgentStatusIndicator status={session.status} size="md" showLabel />
                        <div className="flex items-center gap-2">
                          <span className="font-medium capitalize">{session.agent_type}</span>
                          <span className="text-sm text-muted-foreground">{session.model}</span>
                        </div>
                      </div>

                      {session.current_task && (
                        <p className="text-sm text-muted-foreground mb-2 line-clamp-2">
                          {session.current_task}
                        </p>
                      )}

                      <div className="flex items-center gap-4 text-xs text-muted-foreground">
                        <div className="flex items-center gap-1">
                          <Clock className="h-3 w-3" />
                          <span>Last activity: {formatTimeAgo(session.last_heartbeat)}</span>
                        </div>
                        <div>Created: {new Date(session.created_at).toLocaleString()}</div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2">
                      {session.status === 'running' || session.status === 'idle' ? (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => pauseSession(session.id)}
                          title="Pause session"
                        >
                          <Pause className="h-4 w-4" />
                        </Button>
                      ) : session.status === 'paused' ? (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => resumeSession(session.id)}
                          title="Resume session"
                        >
                          <Play className="h-4 w-4" />
                        </Button>
                      ) : null}

                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => cancelSession(session.id)}
                        className="text-destructive hover:text-destructive"
                        title="Cancel session"
                      >
                        <X className="h-4 w-4" />
                      </Button>

                      <Button
                        variant="outline"
                        size="sm"
                        asChild
                      >
                        <a href={`/admin/workspaces/${session.workspace_id}/chat?chatId=${session.chat_id}`}>
                          View Chat
                        </a>
                      </Button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
