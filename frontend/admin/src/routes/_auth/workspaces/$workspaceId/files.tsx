import { createFileRoute } from '@tanstack/react-router'
import { FileExplorer } from '@/components/files/FileExplorer'
import { z } from 'zod'

const searchSchema = z.object({
  path: z.string().optional(),
})

export const Route = createFileRoute('/_auth/workspaces/$workspaceId/files')({
  component: FilesRoute,
  validateSearch: (search) => searchSchema.parse(search),
})

function FilesRoute() {
  const { workspaceId } = Route.useParams()
  const { path } = Route.useSearch()

  return (
    <div className="h-[calc(100vh-4rem)] p-4">
      <FileExplorer workspaceId={workspaceId} initialPath={path || '/'}>
        <div className="flex flex-col h-full space-y-4">
          <FileExplorer.Toolbar />
          <div className="flex-1 border rounded-md overflow-hidden">
            <FileExplorer.List />
          </div>
          <FileExplorer.Dialogs />
        </div>
      </FileExplorer>
    </div>
  )
}
