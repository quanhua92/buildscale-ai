import { createFileRoute } from '@tanstack/react-router'
import { MemoriesExplorer } from '@/components/memories/MemoriesExplorer'

export const Route = createFileRoute('/_auth/workspaces/$workspaceId/memories')({
  component: MemoriesRoute,
})

function MemoriesRoute() {
  const { workspaceId } = Route.useParams()

  return (
    <div className="h-[calc(100vh-var(--header-height))] p-4">
      <MemoriesExplorer workspaceId={workspaceId}>
        <div className="flex flex-col h-full space-y-4">
          <MemoriesExplorer.Toolbar />
          <div className="flex-1 overflow-hidden">
            <MemoriesExplorer.List />
          </div>
          <MemoriesExplorer.Dialogs />
        </div>
      </MemoriesExplorer>
    </div>
  )
}
