import React, { createContext, useContext, useState, useEffect, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import type { LsEntry, ReadResult, FileExplorerContextType, ViewMode } from './types'
import { toast, useTools } from '@buildscale/sdk'

const FileExplorerContext = createContext<FileExplorerContextType | undefined>(undefined)

interface FileExplorerProviderProps {
  children: React.ReactNode
  workspaceId: string
  initialPath?: string
}

export function FileExplorerProvider({
  children,
  workspaceId,
  initialPath = '/'
}: FileExplorerProviderProps) {
  const { callTool, ls } = useTools(workspaceId)

  const [currentPath, setCurrentPath] = useState(initialPath)
  const [files, setFiles] = useState<LsEntry[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [viewMode, setViewMode] = useState<ViewMode>('list')
  const [rowSelection, setRowSelection] = useState<Record<string, boolean>>({})

  // UI States
  const [isEditorOpen, setEditorOpen] = useState(false)
  const [isViewerOpen, setViewerOpen] = useState(false)
  const [isFolderOpen, setFolderOpen] = useState(false)
  const [isDeleteOpen, setDeleteOpen] = useState(false)
  const [isMoveOpen, setMoveOpen] = useState(false)
  const [activeFile, setActiveFile] = useState<LsEntry | null>(null)

  const refresh = useCallback(async (path?: string) => {
    setIsLoading(true)
    const targetPath = path || initialPath
    // Use limit: 0 to get all entries (unlimited) for file explorer
    const result = await ls(targetPath, { limit: 0 })
    if (result) {
      // Sort: Folders first, then files
      const sorted = (result.entries as LsEntry[]).sort((a, b) => {
        if (a.file_type === 'folder' && b.file_type !== 'folder') return -1
        if (a.file_type !== 'folder' && b.file_type === 'folder') return 1
        return a.name.localeCompare(b.name)
      })
      setFiles(sorted)
    }
    setIsLoading(false)
  }, [ls, initialPath])

  // Fetch when path changes
  useEffect(() => {
    refresh(initialPath)
    setCurrentPath(initialPath)
    setRowSelection({})
  }, [initialPath, refresh])

  const routerNavigate = useNavigate()

  const buildNewItemPath = useCallback((name: string) => {
    const base = initialPath || '/'
    const cleanPath = base.endsWith('/') ? base : `${base}/`
    return `${cleanPath}${name}`.replace(/\/+/g, '/')
  }, [initialPath])

  const navigate = useCallback((path: string) => {
    // Just update URL, effect will handle fetching and state sync
    routerNavigate({
      to: '.',
      search: (prev: { path?: string }) => ({ ...prev, path }),
      replace: true,
    })
  }, [routerNavigate])

  const createFile = useCallback(async (name: string, content: any, fileType: string = 'document') => {
    const filePath = buildNewItemPath(name)
    
    await callTool('write', { 
      path: filePath, 
      content,
      file_type: fileType
    })
    await refresh()
  }, [buildNewItemPath, callTool, refresh])

  const createFolder = useCallback(async (name: string) => {
    const filePath = buildNewItemPath(name)

    await callTool('write', { 
      path: filePath, 
      content: {}, 
      file_type: 'folder' 
    })
    await refresh()
  }, [buildNewItemPath, callTool, refresh])

  const updateFile = useCallback(async (path: string, content: any) => {
    await callTool('write', { path, content })
    await refresh()
  }, [callTool, refresh])

  const performBatchOperation = useCallback(async <T,>(
    items: T[],
    operation: (item: T) => Promise<any>,
    successMessage: (count: number) => string,
    failureMessage: (failedCount: number, totalCount: number) => string
  ) => {
    setIsLoading(true)
    try {
      const results = await Promise.allSettled(items.map(item => operation(item)))
      const successfulCount = results.filter(r => r.status === 'fulfilled' && r.value !== null).length
      const failedCount = items.length - successfulCount

      if (failedCount === 0) {
        toast.success(successMessage(successfulCount))
      } else if (successfulCount > 0) {
        toast.warning(failureMessage(failedCount, items.length))
      } else {
        toast.error(failureMessage(items.length, items.length))
      }

      await refresh()
      setRowSelection({})
    } finally {
      setIsLoading(false)
    }
  }, [refresh])

  const deleteItems = useCallback(async (paths: string[]) => {
    await performBatchOperation(
      paths,
      path => callTool('rm', { path }),
      count => `Moved ${count} items to trash`,
      (failed, total) => `Failed to move ${failed} of ${total} items to trash`
    )
  }, [callTool, performBatchOperation])

  const moveItems = useCallback(async (sources: string[], destination: string) => {
    // Ensure destination ends with / for directory move
    const targetDir = destination.endsWith('/') ? destination : `${destination}/`
    
    await performBatchOperation(
      sources,
      source => callTool('mv', { source, destination: targetDir }),
      count => `Moved ${count} items to ${destination}`,
      (failed, total) => `Failed to move ${failed} of ${total} items. See console for errors.`
    )
  }, [callTool, performBatchOperation])

  const readFile = useCallback(async (path: string) => {
    return await callTool<ReadResult>('read', { path })
  }, [callTool])

  const selectedEntries = React.useMemo(() => {
    return Object.keys(rowSelection)
      .filter(id => rowSelection[id])
      .map(id => files.find(f => f.id === id))
      .filter((f): f is LsEntry => !!f)
  }, [rowSelection, files])

  const selectedPaths = React.useMemo(() => {
    return selectedEntries.map(e => e.path)
  }, [selectedEntries])

  return (
    <FileExplorerContext.Provider value={{
      workspaceId,
      currentPath,
      navigate,
      refresh,
      files,
      isLoading,
      viewMode,
      setViewMode,
      createFile,
      createFolder,
      updateFile,
      deleteItems,
      moveItems,
      readFile,
      callTool,
      rowSelection,
      setRowSelection,
      selectedEntries,
      selectedPaths,
      isEditorOpen,
      setEditorOpen,
      isViewerOpen,
      setViewerOpen,
      isFolderOpen,
      setFolderOpen,
      isDeleteOpen,
      setDeleteOpen,
      isMoveOpen,
      setMoveOpen,
      activeFile,
      setActiveFile
    }}>
      {children}
    </FileExplorerContext.Provider>
  )
}

export function useFileExplorer() {
  const context = useContext(FileExplorerContext)
  if (context === undefined) {
    throw new Error('useFileExplorer must be used within a FileExplorerProvider')
  }
  return context
}
