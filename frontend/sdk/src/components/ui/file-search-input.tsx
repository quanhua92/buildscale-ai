import * as React from "react"
import { useAuth } from "../../context/AuthContext"
import { Input } from "./input"
import { X } from "lucide-react"
import { cn, debounce } from "../../utils"
import type { FindMatch, FindResult } from "../../api/types"

interface FileSearchInputProps {
  workspaceId: string
  onResults: (results: FindMatch[]) => void
  onQueryChange?: (query: string) => void
  onSearchingChange?: (isSearching: boolean) => void
  placeholder?: string
  debounceMs?: number
  className?: string
}

export function FileSearchInput({
  workspaceId,
  onResults,
  onQueryChange,
  onSearchingChange,
  placeholder = "Search files and folders...",
  debounceMs = 300,
  className,
}: FileSearchInputProps) {
  const { executeTool } = useAuth()
  const [query, setQuery] = React.useState("")

  const searchFiles = React.useCallback(async (searchQuery: string) => {
    if (!searchQuery.trim()) {
      onResults([])
      return
    }
    onSearchingChange?.(true)
    try {
      const result = await executeTool<FindResult>(workspaceId, 'find', {
        name: `*${searchQuery}*`,
        path: '/'
      })
      if (result.success && result.data) {
        // Sort: folders first, then by name
        const sorted = result.data.matches.sort((a, b) => {
          if (a.file_type === 'folder' && b.file_type !== 'folder') return -1
          if (a.file_type !== 'folder' && b.file_type === 'folder') return 1
          return a.name.localeCompare(b.name)
        })
        onResults(sorted)
      } else {
        onResults([])
      }
    } catch (error) {
      console.error('Search failed:', error)
      onResults([])
    } finally {
      onSearchingChange?.(false)
    }
  }, [workspaceId, executeTool, onResults, onSearchingChange])

  const debouncedSearch = React.useMemo(
    () => debounce(searchFiles, debounceMs),
    [searchFiles, debounceMs]
  )

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newQuery = e.target.value
    setQuery(newQuery)
    onQueryChange?.(newQuery)
    debouncedSearch(newQuery)
  }

  const handleClear = () => {
    setQuery("")
    onQueryChange?.("")
    onResults([])
  }

  return (
    <div className={cn("relative", className)}>
      <Input
        placeholder={placeholder}
        value={query}
        onChange={handleChange}
        className="pr-8"
      />
      {query && (
        <button
          type="button"
          onClick={handleClear}
          className="absolute right-2 top-1/2 -translate-y-1/2 hover:text-foreground"
        >
          <X className="h-4 w-4 text-muted-foreground" />
        </button>
      )}
    </div>
  )
}
