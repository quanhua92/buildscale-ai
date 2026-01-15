import { useEffect, useState } from 'react'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { 
  useAuth, 
  Button, 
  Card, 
  CardHeader, 
  CardTitle, 
  CardDescription, 
  CardContent,
  Auth,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  toast,
} from '@buildscale/sdk'
import type { Workspace, WorkspaceMemberDetailed } from '@buildscale/sdk'

export const Route = createFileRoute('/_auth/workspaces/$workspaceId/settings')({
  component: WorkspaceSettings,
})

function WorkspaceSettings() {
  const { workspaceId } = Route.useParams()
  const { user, getWorkspace, getMembership, deleteWorkspace } = useAuth()
  const navigate = useNavigate()
  
  const [workspace, setWorkspace] = useState<Workspace | null>(null)
  const [membership, setMembership] = useState<WorkspaceMemberDetailed | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [isDeleting, setIsDeleting] = useState(false)

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [workspaceResult, membershipResult] = await Promise.all([
          getWorkspace(workspaceId).catch(e => ({ success: false, data: undefined, error: { message: `getWorkspace threw: ${e instanceof Error ? e.message : String(e)}` } })),
          getMembership(workspaceId).catch(e => ({ success: false, data: undefined, error: { message: `getMembership threw: ${e instanceof Error ? e.message : String(e)}` } }))
        ])
        
        if (workspaceResult.success && workspaceResult.data) {
          setWorkspace(workspaceResult.data)
        } else if (workspaceResult.error) {
          setError(`Workspace Error: ${workspaceResult.error.message}`)
        }
        
        if (membershipResult.success && membershipResult.data) {
          setMembership(membershipResult.data)
        } else if (membershipResult.error) {
          setError(prev => prev ? `${prev} | Membership Error: ${membershipResult.error?.message}` : `Membership Error: ${membershipResult.error?.message}`)
        }
      } catch (err) {
        setError(`Unexpected Error: ${err instanceof Error ? err.message : String(err)}`)
      } finally {
        setIsLoading(false)
      }
    }

    fetchData()
  }, [getWorkspace, getMembership, workspaceId])

  const handleDeleteWorkspace = async () => {
    if (!workspace) return
    
    setIsDeleting(true)
    try {
      const result = await deleteWorkspace(workspace.id)
      if (result.success) {
        navigate({ to: '/workspaces/all' })
      } else if (result.error) {
        toast.error(`Error deleting workspace: ${result.error.message}`)
      }
    } catch (err) {
      toast.error(`Unexpected error during deletion: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      setIsDeleting(false)
    }
  }

  if (isLoading) {
    return (
      <div className="p-8 flex justify-center flex-col items-center gap-4">
        <div className="animate-pulse text-muted-foreground">Loading settings for {workspaceId}...</div>
      </div>
    )
  }

  const isOwner = workspace?.owner_id === user?.id
  const isAdmin = membership?.role_name === 'admin'

  if (error || !workspace || (!isOwner && !isAdmin)) {
    return (
      <div className="p-8 max-w-7xl mx-auto">
        <div className="bg-destructive/10 text-destructive p-6 rounded-lg border border-destructive/20">
          <h2 className="text-lg font-semibold mb-2">Access Denied or Error</h2>
          <div className="space-y-2">
            <p className="font-mono text-sm bg-background/50 p-2 rounded">{error || 'Permission check failed'}</p>
            {!workspace && <p className="text-sm">Workspace data is missing.</p>}
            {workspace && !isOwner && !isAdmin && (
              <div className="text-sm space-y-1">
                <p>User ID: <code className="bg-muted px-1 rounded">{user?.id}</code></p>
                <p>Workspace Owner ID: <code className="bg-muted px-1 rounded">{workspace.owner_id}</code></p>
                <p>Your Role: <code className="bg-muted px-1 rounded">{membership?.role_name || 'unknown'}</code></p>
              </div>
            )}
          </div>
          <Link 
            to="/workspaces/$workspaceId" 
            params={{ workspaceId }} 
            className="mt-4 inline-block"
          >
            <Button variant="outline">Back to Workspace Dashboard</Button>
          </Link>
        </div>
      </div>
    )
  }

  return (
    <div className="p-8 max-w-4xl mx-auto">
      <div className="mb-8">
        <Link 
          to="/workspaces/$workspaceId" 
          params={{ workspaceId }}
          className="text-muted-foreground hover:text-foreground text-sm flex items-center gap-1 mb-4"
        >
          ← Back to Workspace
        </Link>
        <h1 className="text-3xl font-bold tracking-tight">Workspace Settings</h1>
        <p className="text-muted-foreground mt-2">
          Manage your workspace preferences and configuration.
        </p>
      </div>

      <div className="space-y-8">
        <Card>
          <CardHeader>
            <CardTitle>General</CardTitle>
            <CardDescription>Basic workspace information.</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="max-w-md">
              <Auth.EditWorkspace workspaceId={workspaceId} />
            </div>
          </CardContent>
        </Card>

        <section className="space-y-4 pt-8 border-t">
          <div className="flex flex-col gap-1">
            <h2 className="text-xl font-semibold text-destructive">Danger Zone</h2>
            <p className="text-sm text-muted-foreground">
              Sensitive operations that can't be undone.
            </p>
          </div>

          <Card className="border-destructive/50">
            <CardHeader>
              <CardTitle className="text-lg">Delete Workspace</CardTitle>
              <CardDescription>
                Permanently delete this workspace and all its data.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                <div className="space-y-1">
                  <p className="text-sm font-medium">Delete this workspace</p>
                  {!isOwner && (
                    <p className="text-xs text-destructive font-semibold">
                      Owner access required to delete workspace.
                    </p>
                  )}
                  <p className="text-xs text-muted-foreground">
                    Once you delete a workspace, there is no going back. Please be certain.
                  </p>
                </div>

                <Dialog>
                  <DialogTrigger asChild>
                    <Button 
                      variant="destructive" 
                      disabled={!isOwner || isDeleting}
                    >
                      {isDeleting ? 'Deleting...' : 'Delete Workspace'}
                    </Button>
                  </DialogTrigger>
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>Are you absolutely sure?</DialogTitle>
                      <DialogDescription>
                        This action cannot be undone. This will permanently delete the
                        <span className="font-bold text-foreground mx-1">"{workspace.name}"</span>
                        workspace and remove all associated data.
                      </DialogDescription>
                    </DialogHeader>
                    <DialogFooter className="gap-2 sm:gap-0">
                      <DialogTrigger asChild>
                        <Button variant="outline">Cancel</Button>
                      </DialogTrigger>
                      <Button 
                        variant="destructive" 
                        onClick={handleDeleteWorkspace}
                        disabled={isDeleting}
                      >
                        {isDeleting ? 'Deleting...' : 'Yes, delete workspace'}
                      </Button>
                    </DialogFooter>
                  </DialogContent>
                </Dialog>
              </div>
            </CardContent>
          </Card>
        </section>
      </div>
    </div>
  )
}
