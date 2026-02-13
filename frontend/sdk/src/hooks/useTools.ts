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

  return { callTool, ls, find, read, write, rm, mv }
}
