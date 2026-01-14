import { createFileRoute } from '@tanstack/react-router'
import { Auth } from '@buildscale/sdk'

export const Route = createFileRoute('/_auth/workspaces/$workspaceId/edit')({
  component: EditWorkspacePage,
})

function EditWorkspacePage() {
  const { workspaceId } = Route.useParams()
  
  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4">
      <Auth.EditWorkspace workspaceId={workspaceId} />
    </div>
  )
}
