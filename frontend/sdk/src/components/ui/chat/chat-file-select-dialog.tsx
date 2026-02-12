import * as React from "react"
import { toast } from "sonner"
import { useAuth } from "../../../context/AuthContext"
import { sortFileEntries } from "../../../utils"
import { Button } from "../button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "../dialog"
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "../breadcrumb"
import { FileSearchInput } from "../file-search-input"
import { Loader2, FolderIcon, FileText, ChevronRight, Home } from "lucide-react"
import type { LsEntry, LsResult, FindMatch } from "../../../api/types"

interface ChatFileSelectDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSelect: (path: string) => void
  workspaceId: string
}

export function ChatFileSelectDialog({
  open,
  onOpenChange,
  onSelect,
  workspaceId,
}: ChatFileSelectDialogProps) {
  const { executeTool } = useAuth()
  const [browsingPath, setBrowsingPath] = React.useState("/")
  const [entries, setEntries] = React.useState<LsEntry[]>([])
  const [isLoading, setIsLoading] = React.useState(false)

  // Search state
  const [searchQuery, setSearchQuery] = React.useState("")
  const [searchResults, setSearchResults] = React.useState<FindMatch[]>([])
  const [isSearching, setIsSearching] = React.useState(false)

  const isSearchMode = searchQuery.trim().length > 0

  // API Helper
  const callTool = React.useCallback(async <T,>(tool: string, args: any): Promise<T | null> => {
    const result = await executeTool<T>(workspaceId, tool, args)
    if (!result.success) {
      toast.error(result.error?.message || 'Tool execution failed')
      return null
    }
    return result.data || null
  }, [executeTool, workspaceId])

  const fetchEntries = React.useCallback(async (path: string) => {
    setIsLoading(true)
    try {
      const result = await callTool<LsResult>('ls', { path })
      if (result) {
        // Sort: Folders first, then files
        const sorted = result.entries.sort(sortFileEntries)
        setEntries(sorted)
      }
    } finally {
      setIsLoading(false)
    }
  }, [callTool])

  // Fetch when dialog opens
  React.useEffect(() => {
    if (open) {
      setBrowsingPath("/")
      fetchEntries("/")
      // Reset search state
      setSearchQuery("")
      setSearchResults([])
    }
  }, [open, fetchEntries])

  const navigateTo = (path: string) => {
    setBrowsingPath(path)
    fetchEntries(path)
  }

  const handleFileSelect = (entry: LsEntry) => {
    if (entry.file_type === 'folder') {
      navigateTo(entry.path)
    } else {
      onSelect(entry.path)
      onOpenChange(false)
    }
  }

  const handleSearchResultSelect = (result: FindMatch) => {
    onSelect(result.path)
    onOpenChange(false)
  }

  const handleSelectFolder = () => {
    onSelect(browsingPath)
    onOpenChange(false)
  }

  const pathParts = browsingPath.split("/").filter(Boolean)
  const handleBreadcrumbClick = (index: number) => {
    if (index === -1) {
      navigateTo("/")
      return
    }
    const newPath = "/" + pathParts.slice(0, index + 1).join("/")
    navigateTo(newPath)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[95vw] sm:max-w-2xl max-h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="p-6 pb-2">
          <DialogTitle>Select a File or Folder</DialogTitle>
          <DialogDescription>
            Search or browse to select a file or folder from your workspace
          </DialogDescription>
        </DialogHeader>

        {/* Search Input */}
        <FileSearchInput
          workspaceId={workspaceId}
          onResults={setSearchResults}
          onQueryChange={setSearchQuery}
          onSearchingChange={setIsSearching}
          className="px-6 mb-2"
        />

        {/* Breadcrumb - only visible when not searching */}
        {!isSearchMode && (
          <div className="px-6 py-2 border-y bg-muted/30">
            <Breadcrumb className="text-xs">
              <BreadcrumbList>
                <BreadcrumbItem>
                  <BreadcrumbLink className="cursor-pointer" onClick={() => handleBreadcrumbClick(-1)}>
                    <Home className="h-3 w-3" />
                  </BreadcrumbLink>
                </BreadcrumbItem>
                {pathParts.map((part, idx) => (
                  <React.Fragment key={`${idx}-${part}`}>
                    <BreadcrumbSeparator><ChevronRight className="h-3 w-3" /></BreadcrumbSeparator>
                    <BreadcrumbItem>
                      {idx === pathParts.length - 1 ? (
                        <BreadcrumbPage>{part}</BreadcrumbPage>
                      ) : (
                        <BreadcrumbLink className="cursor-pointer" onClick={() => handleBreadcrumbClick(idx)}>
                          {part}
                        </BreadcrumbLink>
                      )}
                    </BreadcrumbItem>
                  </React.Fragment>
                ))}
              </BreadcrumbList>
            </Breadcrumb>
          </div>
        )}

        <div className="flex-1 overflow-y-auto min-h-[300px]">
          {isSearchMode ? (
            // Search results
            isSearching ? (
              <div className="flex items-center justify-center h-full">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground opacity-20" />
              </div>
            ) : (
              <div className="divide-y">
                {searchResults.length === 0 ? (
                  <div className="p-8 text-center text-sm text-muted-foreground italic">
                    No files found matching "{searchQuery}"
                  </div>
                ) : (
                  searchResults.map(result => (
                    <button
                      key={result.path}
                      type="button"
                      onClick={() => handleSearchResultSelect(result)}
                      className="w-full flex items-center gap-3 px-6 py-3 hover:bg-muted text-sm transition-colors text-left"
                    >
                      {result.file_type === 'folder' ? (
                        <FolderIcon className="h-4 w-4 text-blue-500 fill-blue-500/10 shrink-0" />
                      ) : (
                        <FileText className="h-4 w-4 text-muted-foreground shrink-0" />
                      )}
                      <span className="flex-1 text-muted-foreground truncate text-xs font-mono">
                        {result.path}
                      </span>
                    </button>
                  ))
                )}
              </div>
            )
          ) : (
            // Browse mode
            isLoading ? (
              <div className="flex items-center justify-center h-full">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground opacity-20" />
              </div>
            ) : (
              <div className="divide-y">
                {entries.length === 0 ? (
                  <div className="p-8 text-center text-sm text-muted-foreground italic">
                    No files found in this folder
                  </div>
                ) : (
                  entries.map(entry => (
                    <button
                      key={entry.id || entry.path}
                      type="button"
                      onClick={() => handleFileSelect(entry)}
                      className="w-full flex items-center gap-3 px-6 py-3 hover:bg-muted text-sm transition-colors text-left group"
                    >
                      {entry.file_type === 'folder' ? (
                        <FolderIcon className="h-4 w-4 text-blue-500 fill-blue-500/10 shrink-0" />
                      ) : (
                        <FileText className="h-4 w-4 text-muted-foreground shrink-0" />
                      )}
                      <span className="flex-1 font-medium truncate">{entry.display_name || entry.name}</span>
                      {entry.file_type === 'folder' && (
                        <ChevronRight className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
                      )}
                    </button>
                  ))
                )}
              </div>
            )
          )}
        </div>

        <div className="p-6 border-t bg-muted/10 flex justify-end gap-2">
          {!isSearchMode && browsingPath !== "/" && (
            <Button variant="outline" onClick={handleSelectFolder}>
              Select Folder
            </Button>
          )}
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
