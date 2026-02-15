import React, { createContext, useContext, useState, useEffect, useCallback } from 'react'
import { useTools, toast } from '@buildscale/sdk'
import type {
  MemoriesExplorerContextType,
  MemoryEntry,
  CreateMemoryData,
  UpdateMemoryData,
} from './types'

const MemoriesExplorerContext = createContext<MemoriesExplorerContextType | undefined>(undefined)

interface MemoriesExplorerProviderProps {
  children: React.ReactNode
  workspaceId: string
}

export function MemoriesExplorerProvider({
  children,
  workspaceId,
}: MemoriesExplorerProviderProps) {
  const { memorySet, memorySearch, memoryDelete } = useTools(workspaceId)

  const [memories, setMemories] = useState<MemoryEntry[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [allCategories, setAllCategories] = useState<string[]>([])

  // Filters
  const [searchQuery, setSearchQuery] = useState('')
  const [scopeFilter, setScopeFilter] = useState<'all' | 'user' | 'global'>('all')
  const [categoryFilter, setCategoryFilter] = useState('')

  // UI States
  const [isEditorOpen, setEditorOpen] = useState(false)
  const [isViewerOpen, setViewerOpen] = useState(false)
  const [isDeleteOpen, setDeleteOpen] = useState(false)
  const [activeMemory, setActiveMemory] = useState<MemoryEntry | null>(null)
  const [rowSelection, setRowSelection] = useState<Record<string, boolean>>({})

  // Categories from all memories (for dropdown)
  const categories = allCategories

  // Selected entries derived from rowSelection
  const selectedEntries = React.useMemo(() => {
    return Object.keys(rowSelection)
      .filter(id => rowSelection[id])
      .map(id => memories.find(m => m.id === id))
      .filter((m): m is MemoryEntry => !!m)
  }, [rowSelection, memories])

  // Fetch all categories (once on mount)
  const fetchCategories = useCallback(async () => {
    try {
      const result = await memorySearch('.', { limit: 0 })
      if (result) {
        const cats = new Set(result.matches.map(m => m.category))
        setAllCategories(Array.from(cats).sort())
      }
    } catch (error) {
      console.error('Failed to fetch categories:', error)
    }
  }, [memorySearch])

  // Load memories with backend filtering
  const refresh = useCallback(async () => {
    setIsLoading(true)
    try {
      // Build search options - pass filters to backend
      const searchOptions: {
        limit: number
        scope?: 'user' | 'global'
        category?: string
      } = { limit: 0 }

      // Pass scope filter to backend (undefined means 'all')
      if (scopeFilter !== 'all') {
        searchOptions.scope = scopeFilter
      }

      // Pass category filter to backend
      if (categoryFilter) {
        searchOptions.category = categoryFilter
      }

      // Use searchQuery as pattern if provided, otherwise match all
      const pattern = searchQuery || '.'

      const result = await memorySearch(pattern, searchOptions)
      if (result) {
        const entries: MemoryEntry[] = result.matches.map((match) => ({
          id: `${match.scope}-${match.category}-${match.key}`,
          path: match.path,
          scope: match.scope,
          category: match.category,
          key: match.key,
          title: match.title,
          tags: match.tags,
          updated_at: match.updated_at,
        }))

        setMemories(entries)
      }
    } catch (error) {
      console.error('Failed to load memories:', error)
      toast.error('Failed to load memories')
    } finally {
      setIsLoading(false)
    }
  }, [memorySearch, scopeFilter, categoryFilter, searchQuery])

  // Initial load - fetch categories and memories
  useEffect(() => {
    fetchCategories()
  }, [fetchCategories])

  // Refresh memories when filters change
  useEffect(() => {
    refresh()
  }, [refresh])

  // Create memory
  const createMemory = useCallback(async (data: CreateMemoryData) => {
    try {
      const result = await memorySet(
        data.scope,
        data.category,
        data.key,
        data.title,
        data.content,
        data.tags
      )
      if (result) {
        toast.success('Memory created successfully')
        await refresh()
      }
    } catch (error) {
      console.error('Failed to create memory:', error)
      toast.error('Failed to create memory')
    }
  }, [memorySet, refresh])

  // Update memory
  const updateMemory = useCallback(async (data: UpdateMemoryData) => {
    try {
      const result = await memorySet(
        data.scope,
        data.category,
        data.key,
        data.title,
        data.content,
        data.tags
      )
      if (result) {
        toast.success('Memory updated successfully')
        await refresh()
      }
    } catch (error) {
      console.error('Failed to update memory:', error)
      toast.error('Failed to update memory')
    }
  }, [memorySet, refresh])

  // Delete memory
  const deleteMemory = useCallback(async (
    scope: 'user' | 'global',
    category: string,
    key: string
  ) => {
    try {
      const result = await memoryDelete(scope, category, key)
      if (result) {
        toast.success('Memory deleted successfully')
        await refresh()
        setRowSelection({})
      }
    } catch (error) {
      console.error('Failed to delete memory:', error)
      toast.error('Failed to delete memory')
    }
  }, [memoryDelete, refresh])

  return (
    <MemoriesExplorerContext.Provider
      value={{
        workspaceId,
        memories,
        isLoading,
        refresh,
        searchQuery,
        setSearchQuery,
        scopeFilter,
        setScopeFilter,
        categoryFilter,
        setCategoryFilter,
        categories,
        createMemory,
        updateMemory,
        deleteMemory,
        rowSelection,
        setRowSelection,
        selectedEntries,
        isEditorOpen,
        setEditorOpen,
        isViewerOpen,
        setViewerOpen,
        isDeleteOpen,
        setDeleteOpen,
        activeMemory,
        setActiveMemory,
      }}
    >
      {children}
    </MemoriesExplorerContext.Provider>
  )
}

export function useMemoriesExplorer() {
  const context = useContext(MemoriesExplorerContext)
  if (context === undefined) {
    throw new Error('useMemoriesExplorer must be used within a MemoriesExplorerProvider')
  }
  return context
}
