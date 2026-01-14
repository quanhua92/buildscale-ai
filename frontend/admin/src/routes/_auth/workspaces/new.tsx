import { createFileRoute } from '@tanstack/react-router'
import { Auth } from '@buildscale/sdk'

export const Route = createFileRoute('/_auth/workspaces/new')({
  component: CreateWorkspacePage,
})

function CreateWorkspacePage() {
  return (
    <Auth>
      <Auth.CreateWorkspace />
    </Auth>
  )
}
