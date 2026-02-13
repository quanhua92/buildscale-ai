import { useCallback } from 'react'
import { useAuth } from '../context/AuthContext'
import { toast } from 'sonner'

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

  // Convenience methods with sensible defaults
  const ls = useCallback(async (
    path: string,
    options: { recursive?: boolean; limit?: number } = {}
  ) => {
    const { recursive = false, limit = 0 } = options
    return callTool<{ path: string; entries: unknown[] }>('ls', { path, recursive, limit })
  }, [callTool])

  return { callTool, ls }
}
