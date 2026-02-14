import { useMemoriesExplorer } from './MemoriesContext'
import {
  Button,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@buildscale/sdk"
import { RefreshCw, Plus, Search, Lock, Globe, SlidersHorizontal } from "lucide-react"

export function MemoriesToolbar() {
  const {
    refresh,
    isLoading,
    searchQuery,
    setSearchQuery,
    scopeFilter,
    setScopeFilter,
    categoryFilter,
    setCategoryFilter,
    categories,
    setEditorOpen,
    setActiveMemory,
  } = useMemoriesExplorer()

  const handleNewMemory = () => {
    setActiveMemory(null)
    setEditorOpen(true)
  }

  return (
    <div className="space-y-2 p-2">
      {/* First row: Search + Actions */}
      <div className="flex items-center gap-2">
        {/* Search - takes remaining space */}
        <div className="relative flex-1 min-w-0">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search memories..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 h-9"
          />
        </div>

        {/* Refresh button */}
        <Button
          variant="ghost"
          size="icon"
          onClick={() => refresh()}
          disabled={isLoading}
          title="Refresh"
          className="h-9 w-9 shrink-0"
        >
          <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
        </Button>

        {/* New Memory button - icon only on mobile */}
        <Button size="sm" onClick={handleNewMemory} className="h-9 w-9 sm:w-auto px-0 sm:px-3 shrink-0">
          <Plus className="h-4 w-4 sm:mr-2" />
          <span className="hidden sm:inline">New Memory</span>
        </Button>
      </div>

      {/* Second row: Filters (hidden on very small screens when no filters active) */}
      <div className="flex items-center gap-2 overflow-x-auto no-scrollbar">
        {/* Scope Filter - compact width */}
        <Select
          value={scopeFilter}
          onValueChange={(value: 'all' | 'user' | 'global') => setScopeFilter(value)}
        >
          <SelectTrigger className="h-8 w-auto min-w-[100px] text-xs">
            <SlidersHorizontal className="h-3 w-3 mr-1 shrink-0" />
            <SelectValue placeholder="Scope" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all" className="text-xs">All Scopes</SelectItem>
            <SelectItem value="user" className="text-xs">
              <div className="flex items-center gap-1.5">
                <Lock className="h-3 w-3" />
                User
              </div>
            </SelectItem>
            <SelectItem value="global" className="text-xs">
              <div className="flex items-center gap-1.5">
                <Globe className="h-3 w-3" />
                Global
              </div>
            </SelectItem>
          </SelectContent>
        </Select>

        {/* Category Filter - compact width */}
        {categories.length > 0 && (
          <Select
            value={categoryFilter || "all"}
            onValueChange={(value) => setCategoryFilter(value === "all" ? "" : value)}
          >
            <SelectTrigger className="h-8 w-auto min-w-[120px] text-xs">
              <SelectValue placeholder="Category" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all" className="text-xs">All Categories</SelectItem>
              {categories.map((cat) => (
                <SelectItem key={cat} value={cat} className="text-xs">
                  {cat}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>
    </div>
  )
}
