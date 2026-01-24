import React from 'react'
import { FileExplorerProvider } from './FileExplorerContext'
import { FileExplorerList } from './FileExplorerList'
import { FileExplorerToolbar } from './FileExplorerToolbar'
import { FileExplorerDialogs } from './FileExplorerDialogs'

interface FileExplorerProps {
  children?: React.ReactNode
  workspaceId: string
  initialPath?: string
}

export function FileExplorer({ children, workspaceId, initialPath }: FileExplorerProps) {
  return (
    <FileExplorerProvider workspaceId={workspaceId} initialPath={initialPath}>
      {children}
    </FileExplorerProvider>
  )
}

FileExplorer.List = FileExplorerList
FileExplorer.Toolbar = FileExplorerToolbar
FileExplorer.Dialogs = FileExplorerDialogs
