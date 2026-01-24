import React, { createContext, useContext, useState, useEffect, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import type { LsEntry, LsResult, ReadResult, ToolResponse, FileExplorerContextType, ViewMode } from './types'
import { toast } from '@buildscale/sdk'

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
  // Removed useAuth as we are using fetch with credentials: include for now
  
  // Checking SDK exports again... it exports ApiClient class but we need the instance configured with tokens.
  // Ideally, the SDK should expose a way to make generic authenticated requests or specific tool requests.
  // Since we saw `ApiClient` exported in SDK, we can import it, but we need tokens.
  // The SDK's ApiClient handles token refresh automatically if configured. 
  // However, we are inside the admin app which uses the SDK.
  // Let's assume we can use `fetch` and the browser's cookies (since we are on the same domain or using credentials: include)
  // Or better, let's use the `useAuth` hook if it provided a generic `request` method, but it doesn't seem to.
  
  // Checking SDK exports again... it exports ApiClient class but we need the instance configured with tokens.
  // The AuthContext uses an instance of ApiClient.
  // For now, let's implement a simple fetcher that assumes browser cookies or we'll assume the `apiBaseUrl` from environment.

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
    try {
      // Assuming /api/v1 is the prefix, we need to construct the URL
      // The SDK context probably knows the base URL, but let's assume standard relative path for now
      // or use environment variable if available.
      const baseUrl = import.meta.env.VITE_API_URL || 'http://localhost:3000/api/v1'
      
      // Get access token from storage if needed, but if using cookies we might not need to manually attach
      // if the backend expects cookies. The SDK doc says "Browser Clients (Cookie): Cookies are set automatically".
      // So we just need `credentials: 'include'`.
      
      const response = await fetch(`${baseUrl}/workspaces/${workspaceId}/tools`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        credentials: 'include',
        body: JSON.stringify({
          tool,
          args
        })
      })

      if (!response.ok) {
        throw new Error(`Tool ${tool} failed: ${response.statusText}`)
      }

      const data: ToolResponse<T> = await response.json()
      
      if (!data.success) {
        throw new Error(data.error || 'Unknown tool error')
      }

      return data.result
    } catch (error) {
      console.error(`Error calling tool ${tool}:`, error)
      toast.error(error instanceof Error ? error.message : 'Tool execution failed')
      return null
    }
  }, [workspaceId])

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
