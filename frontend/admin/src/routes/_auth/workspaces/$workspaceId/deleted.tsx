import { createFileRoute } from '@tanstack/react-router'
import { useEffect, useState, useCallback } from 'react'
import { 
  useAuth, 
  type File, 
  toast,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Button
} from '@buildscale/sdk'
import { DeletedFilesList } from '@/components/deleted/DeletedFilesList'
import { Loader2, Trash2 } from 'lucide-react'

export const Route = createFileRoute('/_auth/workspaces/$workspaceId/deleted')({
  component: DeletedFilesPage,
})

function DeletedFilesPage() {
  const { workspaceId } = Route.useParams()
  const { apiClient } = useAuth()
  const [files, setFiles] = useState<File[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [fileToPurge, setFileToPurge] = useState<File | null>(null)

  const fetchDeletedFiles = useCallback(async () => {
    try {
      setIsLoading(true)
      const deletedFiles = await apiClient.listDeletedFiles(workspaceId)
      setFiles(deletedFiles)
    } catch (error) {
      console.error('Failed to fetch deleted files:', error)
      toast.error("Failed to load deleted files")
    } finally {
      setIsLoading(false)
    }
  }, [apiClient, workspaceId])

  useEffect(() => {
    fetchDeletedFiles()
  }, [fetchDeletedFiles])

  const handleRestore = async (file: File) => {
    try {
      await apiClient.restoreFile(workspaceId, file.id)
      toast.success(`Restored ${file.name}`)
      // Refresh list
      fetchDeletedFiles()
    } catch (error) {
      console.error('Failed to restore file:', error)
      toast.error(`Failed to restore ${file.name}`)
    }
  }

  const handleBatchRestore = async (filesToRestore: File[]) => {
    try {
      const promises = filesToRestore.map(file => apiClient.restoreFile(workspaceId, file.id))
      await Promise.all(promises)
      toast.success(`Restored ${filesToRestore.length} files`)
      fetchDeletedFiles()
    } catch (error) {
      console.error('Failed to restore files:', error)
      toast.error("Failed to restore some files")
    }
  }

  const handlePurge = (file: File) => {
    setFileToPurge(file)
  }

  const confirmPurge = async () => {
    if (!fileToPurge) return
    
    try {
      await apiClient.purgeFile(workspaceId, fileToPurge.id)
      toast.success(`Permanently deleted ${fileToPurge.name}`)
      fetchDeletedFiles()
    } catch (error) {
      console.error('Failed to purge file:', error)
      toast.error(`Failed to delete ${fileToPurge.name}`)
    } finally {
      setFileToPurge(null)
    }
  }

  return (
    <div className="flex flex-col h-full p-6 space-y-4">
      <div className="flex items-center space-x-2">
        <Trash2 className="w-6 h-6 text-muted-foreground" />
        <h1 className="text-2xl font-bold tracking-tight">Recently Deleted</h1>
      </div>
      
      <div className="flex-1 overflow-hidden">
        {isLoading ? (
          <div className="flex h-full items-center justify-center">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <DeletedFilesList 
            files={files} 
            onRestore={handleRestore} 
            onRestoreBatch={handleBatchRestore} 
            onPurge={handlePurge}
          />
        )}
      </div>

      <Dialog open={!!fileToPurge} onOpenChange={(open) => !open && setFileToPurge(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Permanently Delete File</DialogTitle>
            <DialogDescription>
              Are you sure you want to permanently delete "{fileToPurge?.name}"? 
              This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setFileToPurge(null)}>Cancel</Button>
            <Button variant="destructive" onClick={confirmPurge}>Delete Forever</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
