import React from "react"

// Re-export memory types from SDK
export type { MemoryScope, MemorySetResult, MemoryGetResult, MemorySearchResult, MemoryMatch, MemoryDeleteResult, MemoryMetadata } from '@buildscale/sdk'

// Memory entry for display in table
export interface MemoryEntry {
  id: string
  path: string
  scope: 'user' | 'global'
  category: string
  key: string
  title: string
  tags: string[]
  updated_at: string
}

// Context type for MemoriesExplorer
export interface MemoriesExplorerContextType {
  workspaceId: string
  memories: MemoryEntry[]
  isLoading: boolean
  refresh: () => Promise<void>

  // Filters
  searchQuery: string
  setSearchQuery: (query: string) => void
  scopeFilter: 'all' | 'user' | 'global'
  setScopeFilter: (scope: 'all' | 'user' | 'global') => void
  categoryFilter: string
  setCategoryFilter: (category: string) => void
  categories: string[]

  // CRUD operations
  createMemory: (data: CreateMemoryData) => Promise<void>
  updateMemory: (data: UpdateMemoryData) => Promise<void>
  deleteMemory: (scope: 'user' | 'global', category: string, key: string) => Promise<void>

  // Selection
  rowSelection: Record<string, boolean>
  setRowSelection: React.Dispatch<React.SetStateAction<Record<string, boolean>>>
  selectedEntries: MemoryEntry[]

  // UI State
  isEditorOpen: boolean
  setEditorOpen: (open: boolean) => void
  isViewerOpen: boolean
  setViewerOpen: (open: boolean) => void
  isDeleteOpen: boolean
  setDeleteOpen: (open: boolean) => void
  activeMemory: MemoryEntry | null
  setActiveMemory: (memory: MemoryEntry | null) => void
}

export interface CreateMemoryData {
  scope: 'user' | 'global'
  category: string
  key: string
  title: string
  content: string
  tags?: string[]
}

export interface UpdateMemoryData extends CreateMemoryData {}

// View modes
export type ViewMode = "list" | "grid"
