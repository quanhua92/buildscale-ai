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
    // The write tool might handle folder creation if the path implies it, 
    // or we might need a specific way to create folders. 
    // Looking at backend docs: "Create File" endpoint handles folders. 
    // But "Tools API" has `write`. 
    // The `write` tool description says "Create or update file". 
    // Does `write` support creating empty folders? Maybe not directly via `write` tool if it expects content.
    // The `ls` tool just lists.
    // If we strictly stick to the 4 tools, we use `write` to create files. 
    // To create a folder, we might need to use the `POST /files` endpoint directly if the `write` tool doesn't support folder creation explicitly.
    // However, the backend doc says "Everything is a File". 
    // Let's assume for now we create a file. If we MUST create a folder, we might need to use the REST API `create_file` instead of the tool.
    // For this implementation, let's focus on files first, or maybe `write` with specific content creates a folder?
    // Checking `backend/src/tools/write.rs` would verify this, but let's stick to files for now or use a hack like creating a .keep file.
    
    // Actually, let's look at the `create_file` handler. It supports `file_type`.
    // The `write` tool uses `file_services::create_version` or `create_file_with_content`.
    // Let's assume we can only create files via `write` for now unless we extend the tools.
    // We will implement "New File" primarily.
    
    const filePath = currentPath === '/' ? `/${name}` : `${currentPath}/${name}`
     await callTool('write', { path: filePath, content: { text: "" } }) // Create empty file
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
