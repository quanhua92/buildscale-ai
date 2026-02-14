import React from 'react'
import { MemoriesExplorerProvider } from './MemoriesContext'
import { MemoriesList } from './MemoriesList'
import { MemoriesToolbar } from './MemoriesToolbar'
import { MemoriesDialogs } from './MemoriesDialogs'

interface MemoriesExplorerProps {
  children?: React.ReactNode
  workspaceId: string
}

export function MemoriesExplorer({ children, workspaceId }: MemoriesExplorerProps) {
  return (
    <MemoriesExplorerProvider workspaceId={workspaceId}>
      {children}
    </MemoriesExplorerProvider>
  )
}

MemoriesExplorer.List = MemoriesList
MemoriesExplorer.Toolbar = MemoriesToolbar
MemoriesExplorer.Dialogs = MemoriesDialogs
