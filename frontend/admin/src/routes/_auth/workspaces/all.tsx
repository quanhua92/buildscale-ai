import { useEffect, useState } from 'react'
import { createFileRoute, Link } from '@tanstack/react-router'
import { 
  useAuth, 
  Button, 
  Table, 
  TableHeader, 
  TableBody, 
  TableRow, 
  TableHead, 
  TableCell,
  formatDate
} from '@buildscale/sdk'
import type { Workspace } from '@buildscale/sdk'

export const Route = createFileRoute('/_auth/workspaces/all')({
  component: AllWorkspaces,
})

function AllWorkspaces() {
  const { listWorkspaces, user } = useAuth()
  const [workspaces, setWorkspaces] = useState<Workspace[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchWorkspaces = async () => {
      const result = await listWorkspaces()
      
      if (result.success && result.data) {
        setWorkspaces(result.data.workspaces)
      } else if (result.error) {
        setError(result.error.message)
      }
      
      setIsLoading(false)
    }

    fetchWorkspaces()
  }, [listWorkspaces])

  return (
    <div className="p-8 max-w-7xl mx-auto">
      <div className="flex justify-between items-center mb-8">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Workspaces</h1>
          <p className="text-muted-foreground mt-2">
            Manage your workspaces and team members.
          </p>
        </div>
        <Link to="/workspaces/new">
          <Button>Create Workspace</Button>
        </Link>
      </div>

      {error && (
        <div className="bg-destructive/10 text-destructive p-4 rounded-md mb-6">
          {error}
        </div>
      )}

      <div className="border rounded-lg shadow-sm">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Created At</TableHead>
              <TableHead>Role</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {isLoading ? (
              <TableRow>
                <TableCell colSpan={4} className="h-24 text-center">
                  Loading workspaces...
                </TableCell>
              </TableRow>
            ) : workspaces.length === 0 ? (
              <TableRow>
                <TableCell colSpan={4} className="h-24 text-center">
                  No workspaces found. Create one to get started.
                </TableCell>
              </TableRow>
            ) : (
              workspaces.map((workspace) => (
                <TableRow key={workspace.id}>
                  <TableCell className="font-medium">
                    <Link 
                      to="/workspaces/$workspaceId" 
                      params={{ workspaceId: workspace.id }}
                      className="hover:underline text-primary"
                    >
                      {workspace.name}
                    </Link>
                  </TableCell>
                  <TableCell>
                    {formatDate(workspace.created_at)}
                  </TableCell>
                  <TableCell>
                    <span className="inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80 capitalize">
                      {workspace.role_name || (workspace.owner_id === user?.id ? 'Owner' : 'Member')}
                    </span>
                  </TableCell>
                  <TableCell className="text-right">
                    <Link 
                      to="/workspaces/$workspaceId" 
                      params={{ workspaceId: workspace.id }}
                    >
                      <Button variant="ghost" size="sm">
                        View
                      </Button>
                    </Link>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  )
}
