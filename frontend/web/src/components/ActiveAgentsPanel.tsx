import { useEffect, useState } from 'react'
import { Card, CardHeader, CardTitle, CardContent, Button, useAuth } from '@buildscale/sdk'
import type { AgentSession, SessionStatus } from '@buildscale/sdk'
import { Circle, Pause, Play, X, Clock } from 'lucide-react'

interface ActiveAgentsPanelProps {
  workspaceId: string
}

export function ActiveAgentsPanel({ workspaceId }: ActiveAgentsPanelProps) {
  const { apiClient } = useAuth()
  const [sessions, setSessions] = useState<AgentSession[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Poll for active sessions every 5 seconds
  useEffect(() => {
    const fetchSessions = async () => {
      try {
        const response = await apiClient.getWorkspaceAgentSessions(workspaceId)
        setSessions(response.sessions)
        setError(null)
      } catch (err) {
        console.error('Failed to fetch agent sessions:', err)
        setError('Failed to load active agents')
      } finally {
        setLoading(false)
      }
    }

    fetchSessions()
    const interval = setInterval(fetchSessions, 5000)

    return () => clearInterval(interval)
  }, [workspaceId, apiClient])

  const handlePause = async (sessionId: string) => {
    try {
      await apiClient.pauseAgentSession(sessionId)
      // Refresh will happen on next poll
    } catch (err) {
      console.error('Failed to pause session:', err)
    }
  }

  const handleResume = async (sessionId: string) => {
    try {
      await apiClient.resumeAgentSession(sessionId)
      // Refresh will happen on next poll
    } catch (err) {
      console.error('Failed to resume session:', err)
    }
  }

  const handleCancel = async (sessionId: string) => {
    try {
      await apiClient.cancelAgentSession(sessionId)
      // Refresh will happen on next poll
    } catch (err) {
      console.error('Failed to cancel session:', err)
    }
  }

  const getStatusIcon = (status: SessionStatus) => {
    switch (status) {
      case 'running':
        return <Circle className="h-2 w-2 fill-green-500 text-green-500" />
      case 'idle':
        return <Circle className="h-2 w-2 fill-yellow-500 text-yellow-500" />
      case 'paused':
        return <Pause className="h-2 w-2 text-orange-500" />
      case 'completed':
        return <Circle className="h-2 w-2 fill-blue-500 text-blue-500" />
      case 'error':
        return <X className="h-2 w-2 text-red-500" />
      default:
        return <Circle className="h-2 w-2 text-gray-500" />
    }
  }

  const getStatusText = (status: SessionStatus) => {
    switch (status) {
      case 'running':
        return 'Running'
      case 'idle':
        return 'Idle'
      case 'paused':
        return 'Paused'
      case 'completed':
        return 'Completed'
      case 'error':
        return 'Error'
      default:
        return 'Unknown'
    }
  }

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

  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Active Agents</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-muted-foreground">Loading...</div>
        </CardContent>
      </Card>
    )
  }

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Active Agents</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-destructive">{error}</div>
        </CardContent>
      </Card>
    )
  }

  if (sessions.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Active Agents</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-muted-foreground">No active agents</div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm font-medium">
          Active Agents ({sessions.length})
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          {sessions.map((session) => (
            <div
              key={session.id}
              className="flex items-start gap-3 p-3 rounded-lg border bg-card"
            >
              <div className="mt-0.5">{getStatusIcon(session.status)}</div>

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-sm capitalize">
                    {session.agent_type}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {session.model}
                  </span>
                </div>

                {session.current_task && (
                  <p className="text-xs text-muted-foreground mt-1 truncate">
                    {session.current_task}
                  </p>
                )}

                <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {formatTimeAgo(session.last_heartbeat)}
                  </span>
                  <span>{getStatusText(session.status)}</span>
                </div>
              </div>

              <div className="flex items-center gap-1">
                {session.status === 'running' || session.status === 'idle' ? (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7"
                    onClick={() => handlePause(session.id)}
                    title="Pause session"
                  >
                    <Pause className="h-3 w-3" />
                  </Button>
                ) : session.status === 'paused' ? (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7"
                    onClick={() => handleResume(session.id)}
                    title="Resume session"
                  >
                    <Play className="h-3 w-3" />
                  </Button>
                ) : null}

                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-destructive"
                  onClick={() => handleCancel(session.id)}
                  title="Cancel session"
                >
                  <X className="h-3 w-3" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}
