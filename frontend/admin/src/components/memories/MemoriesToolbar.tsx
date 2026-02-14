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
import { RefreshCw, Plus, Search, Lock, Globe } from "lucide-react"

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
    <div className="flex items-center justify-between p-2 gap-2 overflow-hidden flex-wrap">
      <div className="flex items-center gap-2 flex-1 min-w-0">
        {/* Search */}
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search memories..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8"
          />
        </div>

        {/* Scope Filter */}
        <Select
          value={scopeFilter}
          onValueChange={(value: 'all' | 'user' | 'global') => setScopeFilter(value)}
        >
          <SelectTrigger className="w-[130px]">
            <SelectValue placeholder="Scope" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Scopes</SelectItem>
            <SelectItem value="user">
              <div className="flex items-center gap-2">
                <Lock className="h-3.5 w-3.5" />
                User
              </div>
            </SelectItem>
            <SelectItem value="global">
              <div className="flex items-center gap-2">
                <Globe className="h-3.5 w-3.5" />
                Global
              </div>
            </SelectItem>
          </SelectContent>
        </Select>

        {/* Category Filter */}
        {categories.length > 0 && (
          <Select
            value={categoryFilter}
            onValueChange={setCategoryFilter}
          >
            <SelectTrigger className="w-[150px]">
              <SelectValue placeholder="All Categories" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="">All Categories</SelectItem>
              {categories.map((cat) => (
                <SelectItem key={cat} value={cat}>
                  {cat}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>

      <div className="flex items-center gap-2 shrink-0">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => refresh()}
          disabled={isLoading}
          title="Refresh"
        >
          <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
        </Button>
        <Button size="sm" onClick={handleNewMemory} className="h-8">
          <Plus className="h-4 w-4 mr-2" />
          New Memory
        </Button>
      </div>
    </div>
  )
}
