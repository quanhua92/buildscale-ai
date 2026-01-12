import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/_auth/workspaces/all')({
  component: AllWorkspaces,
})

function AllWorkspaces() {
  return (
    <div className="p-8">
      <h1 className="text-3xl font-bold mb-4">All Workspaces</h1>
      <p className="text-muted-foreground">
        This is a protected route. Only authenticated users can see this page.
      </p>
      <div className="mt-8 p-4 bg-accent rounded-lg">
        <p className="text-sm">Workspaces content will be displayed here.</p>
      </div>
    </div>
  )
}
