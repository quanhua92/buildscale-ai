// FileType definition matching backend/docs/EVERYTHING_IS_A_FILE.md
export type FileType = "document" | "folder" | "canvas" | "chat" | "whiteboard"

export interface LsEntry {
  id: string
  name: string
  path: string
  file_type: FileType | string
  is_virtual: boolean
  updated_at: string
}

export interface LsResult {
  path: string
  entries: LsEntry[]
}

export interface ReadResult {
  path: string
  content: any
}

export interface ToolResponse<T> {
  success: boolean
  result: T
  error?: string
}

export type ViewMode = "list" | "grid"

export interface FileExplorerContextType {
  workspaceId: string
  currentPath: string
  navigate: (path: string) => void
  refresh: (path?: string) => void
  files: LsEntry[]
  isLoading: boolean
  viewMode: ViewMode
  setViewMode: (mode: ViewMode) => void
  
  // Actions
  createFile: (name: string, content: any, fileType?: FileType | string) => Promise<void>
  createFolder: (name: string) => Promise<void>
  updateFile: (path: string, content: any) => Promise<void>
  deleteItem: (path: string) => Promise<void>
  readFile: (path: string) => Promise<ReadResult | null>
  
  // Selection
  rowSelection: Record<string, boolean>
  setRowSelection: React.Dispatch<React.SetStateAction<Record<string, boolean>>>
  
  // UI State
  isEditorOpen: boolean
  setEditorOpen: (open: boolean) => void
  isViewerOpen: boolean
  setViewerOpen: (open: boolean) => void
  isFolderOpen: boolean
  setFolderOpen: (open: boolean) => void
  isDeleteOpen: boolean
  setDeleteOpen: (open: boolean) => void
  
  activeFile: LsEntry | null
  setActiveFile: (file: LsEntry | null) => void
}
