import { useEffect, useState } from 'react'
import { createFileRoute, Link } from '@tanstack/react-router'
import { useAuth, Button, Card, CardHeader, CardTitle, CardDescription, CardContent } from '@buildscale/sdk'
import type { Workspace } from '@buildscale/sdk'

export const Route = createFileRoute('/_auth/workspaces/$workspaceId')({
  component: WorkspaceDetail,
})

function WorkspaceDetail() {
  const { workspaceId } = Route.useParams()
  const { getWorkspace } = useAuth()
  const [workspace, setWorkspace] = useState<Workspace | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchWorkspace = async () => {
      setIsLoading(true)
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
        <div className="flex justify-between items-start">
          <div>
            <h1 className="text-3xl font-bold tracking-tight">{workspace.name}</h1>
            <p className="text-muted-foreground mt-2">
              ID: <code className="bg-muted px-1 py-0.5 rounded text-sm">{workspace.id}</code>
            </p>
          </div>
          <Button variant="outline">Settings</Button>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
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
                <dd className="text-sm mt-1">{new Date(workspace.created_at).toLocaleString()}</dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Last Updated</dt>
                <dd className="text-sm mt-1">{new Date(workspace.updated_at).toLocaleString()}</dd>
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
