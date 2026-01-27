import { useEffect, useState } from 'react'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { useAuth, Button, Card, CardHeader, CardTitle, CardDescription, CardContent, formatDateTime } from '@buildscale/sdk'
import { MessageSquare, FolderOpen, Settings, Edit, ArrowRight } from 'lucide-react'
import type { Workspace } from '@buildscale/sdk'

export const Route = createFileRoute('/_auth/workspaces/$workspaceId/')({
  component: WorkspaceDetail,
})

function WorkspaceDetail() {
  const { workspaceId } = Route.useParams()
  const { getWorkspace } = useAuth()
  const navigate = useNavigate()
  const [workspace, setWorkspace] = useState<Workspace | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchWorkspace = async () => {
      const result = await getWorkspace(workspaceId)
      
      if (result.success && result.data) {
        setWorkspace(result.data)
      } else if (result.error) {
        setError(result.error.message)
      }
      
      setIsLoading(false)
    }

    fetchWorkspace()
  }, [getWorkspace, workspaceId])

  if (isLoading) {
    return (
      <div className="p-8 flex justify-center">
        <div className="animate-pulse text-muted-foreground">Loading workspace details...</div>
      </div>
    )
  }

  if (error || !workspace) {
    return (
      <div className="p-8 max-w-7xl mx-auto">
        <div className="bg-destructive/10 text-destructive p-6 rounded-lg border border-destructive/20">
          <h2 className="text-lg font-semibold mb-2">Error Loading Workspace</h2>
          <p>{error || 'Workspace not found'}</p>
          <Link to="/workspaces/all" className="mt-4 inline-block">
            <Button variant="outline">Back to Workspaces</Button>
          </Link>
        </div>
      </div>
    )
  }

  return (
    <div className="p-8 max-w-7xl mx-auto">
      <div className="mb-8">
        <Link to="/workspaces/all" className="text-muted-foreground hover:text-foreground text-sm flex items-center gap-1 mb-4">
          ← Back to Workspaces
        </Link>
        <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
          <div>
            <h1 className="text-3xl font-bold tracking-tight">{workspace.name}</h1>
            <p className="text-muted-foreground mt-2">
              ID: <code className="bg-muted px-1 py-0.5 rounded text-sm">{workspace.id}</code>
            </p>
          </div>
          <div className="flex flex-wrap gap-2 w-full md:w-auto">
            <Button 
              className="gap-2 flex-1 md:flex-none"
              onClick={() => navigate({ to: '/workspaces/$workspaceId/chat', params: { workspaceId } })}
            >
              <MessageSquare size={16} />
              Open Chat
            </Button>
            <Button 
              variant="secondary"
              className="gap-2 flex-1 md:flex-none"
              onClick={() => navigate({ to: '/workspaces/$workspaceId/files', params: { workspaceId } })}
            >
              <FolderOpen size={16} />
              Browse Files
            </Button>
            <div className="flex gap-2 ml-auto md:ml-0">
              <Button 
                variant="outline" 
                size="icon"
                onClick={() => navigate({ to: '/workspaces/$workspaceId/settings', params: { workspaceId } })}
                title="Settings"
              >
                <Settings size={16} />
              </Button>
              <Button 
                variant="outline" 
                size="icon"
                onClick={() => navigate({ to: '/workspaces/$workspaceId/edit', params: { workspaceId } })}
                title="Edit"
              >
                <Edit size={16} />
              </Button>
            </div>
          </div>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
        <Card 
          className="cursor-pointer hover:border-primary/50 transition-colors group"
          onClick={() => navigate({ to: '/workspaces/$workspaceId/chat', params: { workspaceId } })}
        >
          <CardHeader>
            <div className="flex justify-between items-start">
              <div className="p-2 bg-primary/10 rounded-lg text-primary mb-2">
                <MessageSquare size={20} />
              </div>
              <ArrowRight size={16} className="text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
            </div>
            <CardTitle>Agentic Chat</CardTitle>
            <CardDescription>Collaborate with the AI to build and manage files</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">Start a new session to plan architectural changes, search code, or generate features.</p>
          </CardContent>
        </Card>

        <Card 
          className="cursor-pointer hover:border-primary/50 transition-colors group"
          onClick={() => navigate({ to: '/workspaces/$workspaceId/files', params: { workspaceId } })}
        >
          <CardHeader>
            <div className="flex justify-between items-start">
              <div className="p-2 bg-primary/10 rounded-lg text-primary mb-2">
                <FolderOpen size={20} />
              </div>
              <ArrowRight size={16} className="text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
            </div>
            <CardTitle>File Explorer</CardTitle>
            <CardDescription>Manage and browse your workspace documents</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">Access your mirrored filesystem, view file history, and manage your assets.</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Overview</CardTitle>
            <CardDescription>Workspace details and statistics</CardDescription>
          </CardHeader>
          <CardContent>
            <dl className="space-y-4">
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Owner ID</dt>
                <dd className="text-sm mt-1 break-all">{workspace.owner_id}</dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Created At</dt>
                <dd className="text-sm mt-1">{formatDateTime(workspace.created_at)}</dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Last Updated</dt>
                <dd className="text-sm mt-1">{formatDateTime(workspace.updated_at)}</dd>
              </div>
            </dl>
          </CardContent>
        </Card>
        
        {/* Placeholder cards for future features */}
        <Card className="opacity-60">
          <CardHeader>
            <CardTitle>Members</CardTitle>
            <CardDescription>Manage team access</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">Member management coming soon.</p>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
