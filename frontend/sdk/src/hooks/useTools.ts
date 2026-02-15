import { useCallback } from 'react'
import { useAuth } from '../context/AuthContext'
import { toast } from 'sonner'

// Type definitions for tool results
interface LsResult {
  path: string
  entries: unknown[]
}

interface FindResult {
  matches: unknown[]
}

interface ReadResult {
  path: string
  content: unknown
  hash: string
  synced: boolean
}

interface WriteResult {
  path: string
  file_id: string
  version_id: string
  hash: string
}

interface RmResult {
  path: string
  file_id: string | null
}

interface MvResult {
  from_path: string
  to_path: string
}

// Memory type definitions
export type MemoryScope = 'user' | 'global'

export interface MemorySetResult {
  path: string
  file_id: string
  version_id: string
  hash: string
  scope: MemoryScope
  category: string
  key: string
  title: string
  tags: string[]
}

export interface MemoryMetadata {
  title: string
  tags: string[]
  category: string
  created_at: string
  updated_at: string
  scope: MemoryScope
}

export interface MemoryGetResult {
  path: string
  key: string
  metadata: MemoryMetadata | null
  content: string
  hash: string
}

export interface MemoryMatch {
  path: string
  scope: MemoryScope
  category: string
  key: string
  title: string
  /** Truncated content preview (first ~100 words) */
  content_preview: string
  tags: string[]
  updated_at: string
}

export interface MemorySearchResult {
  matches: MemoryMatch[]
  total: number
}

export interface MemoryDeleteResult {
  path: string
  file_id: string
  scope: MemoryScope
  category: string
  key: string
}

/**
 * Generic hook for calling backend tools with built-in error handling.
 * Replaces duplicate callTool wrappers across components.
 */
export function useTools(workspaceId: string) {
  const { executeTool } = useAuth()

  const callTool = useCallback(async <T,>(
    tool: string,
    args: Record<string, unknown>
  ): Promise<T | null> => {
    const result = await executeTool<T>(workspaceId, tool, args)
    if (!result.success) {
      toast.error(result.error?.message || `${tool} failed`)
      return null
    }
    return result.data || null
  }, [executeTool, workspaceId])

  // ls - List directory contents
  const ls = useCallback(async (
    path: string,
    options: { recursive?: boolean; limit?: number } = {}
  ): Promise<LsResult | null> => {
    const { recursive = false, limit = 0 } = options
    return callTool<LsResult>('ls', { path, recursive, limit })
  }, [callTool])

  // find - Search for files by name pattern
  const find = useCallback(async (
    name: string,
    options: { path?: string } = {}
  ): Promise<FindResult | null> => {
    const { path = '/' } = options
    return callTool<FindResult>('find', { name, path })
  }, [callTool])

  // read - Read file contents
  const read = useCallback(async (
    path: string,
    options: { offset?: number; limit?: number } = {}
  ): Promise<ReadResult | null> => {
    const { offset, limit } = options
    const args: Record<string, unknown> = { path }
    if (offset !== undefined) args.offset = offset
    if (limit !== undefined) args.limit = limit
    return callTool<ReadResult>('read', args)
  }, [callTool])

  // write - Create or update file
  const write = useCallback(async (
    path: string,
    content: unknown,
    options: { fileType?: string; overwrite?: boolean } = {}
  ): Promise<WriteResult | null> => {
    const { fileType, overwrite } = options
    const args: Record<string, unknown> = { path, content }
    if (fileType) args.file_type = fileType
    if (overwrite !== undefined) args.overwrite = overwrite
    return callTool<WriteResult>('write', args)
  }, [callTool])

  // rm - Delete file or folder
  const rm = useCallback(async (path: string): Promise<RmResult | null> => {
    return callTool<RmResult>('rm', { path })
  }, [callTool])

  // mv - Move or rename file/folder
  const mv = useCallback(async (
    source: string,
    destination: string
  ): Promise<MvResult | null> => {
    return callTool<MvResult>('mv', { source, destination })
  }, [callTool])

  // memorySet - Store a memory with metadata
  const memorySet = useCallback(async (
    scope: MemoryScope,
    category: string,
    key: string,
    title: string,
    content: string,
    tags?: string[]
  ): Promise<MemorySetResult | null> => {
    const args: Record<string, unknown> = { scope, category, key, title, content }
    if (tags) args.tags = tags
    return callTool<MemorySetResult>('memory_set', args)
  }, [callTool])

  // memoryGet - Retrieve a memory by scope, category, and key
  const memoryGet = useCallback(async (
    scope: MemoryScope,
    category: string,
    key: string
  ): Promise<MemoryGetResult | null> => {
    return callTool<MemoryGetResult>('memory_get', { scope, category, key })
  }, [callTool])

  // memorySearch - Search memories by pattern
  const memorySearch = useCallback(async (
    pattern: string,
    options: {
      scope?: MemoryScope
      category?: string
      tags?: string[]
      caseSensitive?: boolean
      limit?: number
    } = {}
  ): Promise<MemorySearchResult | null> => {
    const args: Record<string, unknown> = { pattern }
    if (options.scope) args.scope = options.scope
    if (options.category) args.category = options.category
    if (options.tags) args.tags = options.tags
    if (options.caseSensitive !== undefined) args.case_sensitive = options.caseSensitive
    if (options.limit !== undefined) args.limit = options.limit
    return callTool<MemorySearchResult>('memory_search', args)
  }, [callTool])

  // memoryDelete - Delete a memory
  const memoryDelete = useCallback(async (
    scope: MemoryScope,
    category: string,
    key: string
  ): Promise<MemoryDeleteResult | null> => {
    return callTool<MemoryDeleteResult>('memory_delete', { scope, category, key })
  }, [callTool])

  return {
    callTool,
    ls,
    find,
    read,
    write,
    rm,
    mv,
    memorySet,
    memoryGet,
    memorySearch,
    memoryDelete,
  }
}
