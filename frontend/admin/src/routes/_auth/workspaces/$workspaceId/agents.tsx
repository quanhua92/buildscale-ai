import { createFileRoute } from '@tanstack/react-router'
import { AgentSessionsProvider, useAgentSessions, AgentStatusIndicator, Button } from '@buildscale/sdk'
import { Pause, Play, X, Search, Clock, Bot, ChevronDown, Filter, MessageSquare } from 'lucide-react'
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
  const { workspaceId } = Route.useParams()
  const { sessions, loading, pauseSession, resumeSession, cancelSession } = useAgentSessions()
  const [filterStatus, setFilterStatus] = useState<SessionStatus | 'all'>('all')
  const [searchQuery, setSearchQuery] = useState('')
  const [showFilters, setShowFilters] = useState(false)

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
    <div className="h-[calc(100vh-var(--header-height))] p-2 sm:p-4">
      <div className="flex flex-col h-full space-y-3 sm:space-y-4 max-w-7xl mx-auto">
        {/* Header */}
        <div className="space-y-3 sm:space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-xl sm:text-2xl font-bold">Agent Sessions</h1>
              <p className="text-muted-foreground text-sm sm:text-base">
                {loading ? 'Loading...' : `${sessions.length} session${sessions.length !== 1 ? 's' : ''}`}
              </p>
            </div>
          </div>

          {/* Search Bar */}
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <input
              type="text"
              placeholder="Search agents..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2.5 border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-ring text-sm"
            />
          </div>

          {/* Mobile Filter Toggle */}
          <div className="sm:hidden">
            <button
              onClick={() => setShowFilters(!showFilters)}
              className="w-full flex items-center justify-between px-4 py-2.5 border rounded-lg bg-background hover:bg-muted/50 transition-colors"
            >
              <span className="flex items-center gap-2 text-sm font-medium">
                <Filter className="h-4 w-4" />
                {filterStatus === 'all' ? 'All Status' : filterStatus.charAt(0).toUpperCase() + filterStatus.slice(1)}
              </span>
              <ChevronDown className={`h-4 w-4 transition-transform ${showFilters ? 'rotate-180' : ''}`} />
            </button>

            {showFilters && (
              <div className="grid grid-cols-3 gap-2 mt-2">
                {(['all', 'running', 'idle', 'paused', 'completed', 'error'] as const).map((status) => (
                  <button
                    key={status}
                    onClick={() => {
                      setFilterStatus(status)
                      setShowFilters(false)
                    }}
                    className={`px-3 py-2 text-xs font-medium rounded-md transition-colors ${
                      filterStatus === status
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted text-muted-foreground hover:bg-muted/80'
                    }`}
                  >
                    {status === 'all' ? 'All' : status.charAt(0).toUpperCase() + status.slice(1)} ({statusCounts[status]})
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Desktop Filters */}
          <div className="hidden sm:flex gap-2 flex-wrap">
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

        {/* Sessions List - Table-like single row layout */}
        <div className="flex-1 border rounded-lg overflow-auto">
          {loading ? (
            <div className="flex items-center justify-center h-full">
              <div className="text-muted-foreground">Loading sessions...</div>
            </div>
          ) : filteredSessions.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-center p-4">
              <Bot className="h-10 w-10 sm:h-12 sm:w-12 text-muted-foreground mb-4" />
              <h3 className="text-base sm:text-lg font-medium mb-2">
                {searchQuery || filterStatus !== 'all' ? 'No matching sessions' : 'No agent sessions'}
              </h3>
              <p className="text-muted-foreground text-xs sm:text-sm">
                {searchQuery || filterStatus !== 'all'
                  ? 'Try adjusting your search or filters'
                  : 'Agent sessions will appear here when agents are running'}
              </p>
            </div>
          ) : (
            <div className="divide-y">
              {filteredSessions.map((session) => (
                <div
                  key={session.id}
                  className="flex items-center gap-3 p-3 sm:p-4 hover:bg-muted/30 transition-colors"
                >
                  {/* Status */}
                  <div className="shrink-0">
                    <AgentStatusIndicator status={session.status} size="sm" />
                  </div>

                  {/* Agent Info */}
                  <div className="flex-1 min-w-0 grid grid-cols-1 sm:grid-cols-[1fr_auto_auto] gap-1 sm:gap-4">
                    <div className="space-y-0.5">
                      {/* Primary info: Agent type + mode + unique identifier */}
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="font-medium capitalize text-sm">{session.agent_type}</span>
                        <span className="text-xs text-muted-foreground lowercase">· {session.mode}</span>
                        <span className="text-xs text-muted-foreground font-mono">
                          #{session.chat_id.slice(0, 6)}
                        </span>
                      </div>

                      {/* Secondary info: Task or Model */}
                      {session.current_task ? (
                        <div className="text-xs text-muted-foreground truncate" title={session.current_task}>
                          {session.current_task}
                        </div>
                      ) : (
                        <div className="text-xs text-muted-foreground">
                          {session.model}
                        </div>
                      )}
                    </div>

                    {/* Timestamp */}
                    <div className="flex items-center gap-1 text-xs text-muted-foreground sm:hidden">
                      <Clock className="h-3 w-3 shrink-0" />
                      <span>{formatTimeAgo(session.last_heartbeat)}</span>
                    </div>

                    {/* Desktop timestamp */}
                    <div className="hidden sm:flex items-center gap-1 text-xs text-muted-foreground">
                      <Clock className="h-3 w-3 shrink-0" />
                      <span>{formatTimeAgo(session.last_heartbeat)}</span>
                    </div>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-1 sm:gap-2 shrink-0">
                    {session.status === 'running' || session.status === 'idle' ? (
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => pauseSession(session.id)}
                        title="Pause session"
                        className="h-8 w-8"
                      >
                        <Pause className="h-3 w-3 sm:h-4 sm:w-4" />
                      </Button>
                    ) : session.status === 'paused' ? (
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => resumeSession(session.id)}
                        title="Resume session"
                        className="h-8 w-8"
                      >
                        <Play className="h-3 w-3 sm:h-4 sm:w-4" />
                      </Button>
                    ) : null}

                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => cancelSession(session.id)}
                      className="text-destructive hover:text-destructive h-8 w-8"
                      title="Cancel session"
                    >
                      <X className="h-3 w-3 sm:h-4 sm:w-4" />
                    </Button>

                    <Button
                      variant="outline"
                      size="sm"
                      asChild
                      className="h-8 px-2 sm:h-9 sm:px-3"
                    >
                      <a href={`/admin/workspaces/${workspaceId}/chat?chatId=${session.chat_id}`}>
                        <MessageSquare className="h-3 w-3 sm:h-4 sm:w-4 sm:mr-1" />
                        <span className="hidden sm:inline">Chat</span>
                      </a>
                    </Button>
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
