import React, { createContext, useContext, useState, useEffect, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import type { LsEntry, LsResult, ReadResult, FileExplorerContextType, ViewMode } from './types'
import { toast, useAuth } from '@buildscale/sdk'

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
  const { executeTool } = useAuth()

  const [currentPath, setCurrentPath] = useState(initialPath)
  const [files, setFiles] = useState<LsEntry[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [viewMode, setViewMode] = useState<ViewMode>('list')
  const [rowSelection, setRowSelection] = useState({})
  
  // UI States
  const [isEditorOpen, setEditorOpen] = useState(false)
  const [isViewerOpen, setViewerOpen] = useState(false)
  const [isDeleteOpen, setDeleteOpen] = useState(false)
  const [activeFile, setActiveFile] = useState<LsEntry | null>(null)

  // API Helper
  const callTool = useCallback(async <T,>(tool: string, args: any): Promise<T | null> => {
    const result = await executeTool<T>(workspaceId, tool, args)
    if (!result.success) {
      toast.error(result.error?.message || 'Tool execution failed')
      return null
    }
    return result.data || null
  }, [executeTool, workspaceId])

  const refresh = useCallback(async () => {
    setIsLoading(true)
    const result = await callTool<LsResult>('ls', { path: currentPath })
    if (result) {
      // Sort: Folders first, then files
      const sorted = result.entries.sort((a, b) => {
        if (a.file_type === 'folder' && b.file_type !== 'folder') return -1
        if (a.file_type !== 'folder' && b.file_type === 'folder') return 1
        return a.name.localeCompare(b.name)
      })
      setFiles(sorted)
    }
    setIsLoading(false)
  }, [callTool, currentPath])

  // Initial load
  useEffect(() => {
    refresh()
  }, [refresh])

  // Sync state with prop change (e.g. browser back button)
  useEffect(() => {
    if (initialPath !== currentPath) {
      setCurrentPath(initialPath)
    }
  }, [initialPath, currentPath])

  const routerNavigate = useNavigate()

  const navigate = (path: string) => {
    setCurrentPath(path)
    setRowSelection({}) // Clear selection on navigation
    // Update URL
    routerNavigate({
      to: '.',
      search: (prev: any) => ({ ...prev, path }),
      replace: true, // Replace history entry to avoid clutter
    })
  }

  const createFile = async (name: string, content: string) => {
    const filePath = currentPath === '/' ? `/${name}` : `${currentPath}/${name}`
    await callTool('write', { path: filePath, content: { text: content } })
    refresh()
  }

  const createFolder = async (name: string) => {
    const filePath = currentPath === '/' ? `/${name}` : `${currentPath}/${name}`
    await callTool('write', { 
      path: filePath, 
      content: {}, 
      file_type: 'folder' 
    })
    refresh()
  }

  const updateFile = async (path: string, content: string) => {
    await callTool('write', { path, content: { text: content } })
    refresh()
  }

  const deleteItem = async (path: string) => {
    await callTool('rm', { path })
    refresh()
    setRowSelection({})
  }

  const readFile = async (path: string) => {
    return await callTool<ReadResult>('read', { path })
  }

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
      deleteItem,
      readFile,
      rowSelection,
      setRowSelection,
      isEditorOpen,
      setEditorOpen,
      isViewerOpen,
      setViewerOpen,
      isDeleteOpen,
      setDeleteOpen,
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
