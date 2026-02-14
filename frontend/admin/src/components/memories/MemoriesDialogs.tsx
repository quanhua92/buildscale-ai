import { useState, useEffect } from 'react'
import { useMemoriesExplorer } from './MemoriesContext'
import type { CreateMemoryData } from './types'
import {
  Button,
  Input,
  Label,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  useTools,
} from "@buildscale/sdk"

// Simple Textarea component
function Textarea({ className, ...props }: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={`flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 ${className || ''}`}
      {...props}
    />
  )
}

// Memory Editor Dialog
export function MemoryEditorDialog() {
  const {
    isEditorOpen,
    setEditorOpen,
    activeMemory,
    createMemory,
    updateMemory,
    categories,
  } = useMemoriesExplorer()

  const [scope, setScope] = useState<'user' | 'global'>('user')
  const [category, setCategory] = useState('')
  const [key, setKey] = useState('')
  const [title, setTitle] = useState('')
  const [content, setContent] = useState('')
  const [tags, setTags] = useState('')
  const [isNewCategory, setIsNewCategory] = useState(false)

  const isEditing = !!activeMemory

  // Populate form when editing
  useEffect(() => {
    if (activeMemory) {
      setScope(activeMemory.scope)
      setCategory(activeMemory.category)
      setKey(activeMemory.key)
      setTitle(activeMemory.title)
      setContent('') // Content needs to be fetched separately
      setTags(activeMemory.tags.join(', '))
      setIsNewCategory(false)
    } else {
      // Reset form for new memory
      setScope('user')
      setCategory('')
      setKey('')
      setTitle('')
      setContent('')
      setTags('')
      setIsNewCategory(false)
    }
  }, [activeMemory, isEditorOpen])

  const handleSubmit = async () => {
    if (!category || !key || !title || !content) {
      return
    }

    const data: CreateMemoryData = {
      scope,
      category,
      key,
      title,
      content,
      tags: tags ? tags.split(',').map(t => t.trim()).filter(Boolean) : undefined,
    }

    if (isEditing) {
      await updateMemory(data)
    } else {
      await createMemory(data)
    }

    setEditorOpen(false)
  }

  return (
    <Dialog open={isEditorOpen} onOpenChange={setEditorOpen}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{isEditing ? 'Edit Memory' : 'New Memory'}</DialogTitle>
          <DialogDescription>
            {isEditing
              ? 'Update the memory content and metadata.'
              : 'Create a new memory to store information for later recall.'}
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 py-4">
          {/* Scope */}
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="scope" className="text-right">Scope</Label>
            <Select value={scope} onValueChange={(v: 'user' | 'global') => setScope(v)}>
              <SelectTrigger className="col-span-3">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="user">User (Private)</SelectItem>
                <SelectItem value="global">Global (Shared)</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Category */}
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="category" className="text-right">Category</Label>
            {isNewCategory ? (
              <div className="col-span-3 flex gap-2">
                <Input
                  id="category"
                  value={category}
                  onChange={(e) => setCategory(e.target.value)}
                  placeholder="e.g., preferences, project, decisions"
                />
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setIsNewCategory(false)}
                >
                  Select
                </Button>
              </div>
            ) : (
              <div className="col-span-3 flex gap-2">
                <Select value={category} onValueChange={setCategory}>
                  <SelectTrigger className="flex-1">
                    <SelectValue placeholder="Select category" />
                  </SelectTrigger>
                  <SelectContent>
                    {categories.map((cat) => (
                      <SelectItem key={cat} value={cat}>{cat}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setCategory('')
                    setIsNewCategory(true)
                  }}
                >
                  New
                </Button>
              </div>
            )}
          </div>

          {/* Key */}
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="key" className="text-right">Key</Label>
            <Input
              id="key"
              value={key}
              onChange={(e) => setKey(e.target.value)}
              placeholder="e.g., coding-style, api-endpoints"
              className="col-span-3"
              disabled={isEditing}
            />
          </div>

          {/* Title */}
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="title" className="text-right">Title</Label>
            <Input
              id="title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Human-readable title"
              className="col-span-3"
            />
          </div>

          {/* Content */}
          <div className="grid grid-cols-4 items-start gap-4">
            <Label htmlFor="content" className="text-right pt-2">Content</Label>
            <Textarea
              id="content"
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder="Memory content in markdown..."
              className="col-span-3 min-h-[200px]"
            />
          </div>

          {/* Tags */}
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="tags" className="text-right">Tags</Label>
            <Input
              id="tags"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="Comma-separated tags (optional)"
              className="col-span-3"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => setEditorOpen(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={!category || !key || !title || !content}>
            {isEditing ? 'Update' : 'Create'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// Memory Viewer Dialog
export function MemoryViewerDialog() {
  const {
    isViewerOpen,
    setViewerOpen,
    activeMemory,
    setEditorOpen,
    setDeleteOpen,
    workspaceId,
  } = useMemoriesExplorer()

  const { callTool } = useTools(workspaceId)
  const [content, setContent] = useState('')
  const [isLoading, setIsLoading] = useState(false)

  // Fetch content when dialog opens
  useEffect(() => {
    if (isViewerOpen && activeMemory) {
      setIsLoading(true)
      callTool<{ content: string }>('memory_get', {
        scope: activeMemory.scope,
        category: activeMemory.category,
        key: activeMemory.key,
      })
        .then((result) => {
          if (result) {
            setContent(result.content)
          }
        })
        .finally(() => setIsLoading(false))
    }
  }, [isViewerOpen, activeMemory, callTool])

  const handleEdit = () => {
    setViewerOpen(false)
    setEditorOpen(true)
  }

  const handleDelete = () => {
    setViewerOpen(false)
    setDeleteOpen(true)
  }

  if (!activeMemory) return null

  return (
    <Dialog open={isViewerOpen} onOpenChange={setViewerOpen}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{activeMemory.title}</DialogTitle>
          <DialogDescription>
            <span className="font-mono text-xs">
              {activeMemory.scope}/{activeMemory.category}/{activeMemory.key}
            </span>
          </DialogDescription>
        </DialogHeader>

        <div className="py-4">
          {isLoading ? (
            <div className="text-muted-foreground">Loading...</div>
          ) : (
            <pre className="whitespace-pre-wrap text-sm bg-muted p-4 rounded-md overflow-auto max-h-[400px]">
              {content}
            </pre>
          )}

          {activeMemory.tags.length > 0 && (
            <div className="mt-4 flex items-center gap-2">
              <span className="text-sm text-muted-foreground">Tags:</span>
              {activeMemory.tags.map((tag) => (
                <span
                  key={tag}
                  className="inline-flex items-center px-2 py-1 text-xs bg-muted rounded"
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={handleDelete} className="text-destructive">
            Delete
          </Button>
          <Button onClick={handleEdit}>
            Edit
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// Memory Delete Confirmation Dialog
export function MemoryDeleteDialog() {
  const {
    isDeleteOpen,
    setDeleteOpen,
    activeMemory,
    deleteMemory,
  } = useMemoriesExplorer()

  const handleDelete = async () => {
    if (activeMemory) {
      await deleteMemory(activeMemory.scope, activeMemory.category, activeMemory.key)
    }
    setDeleteOpen(false)
  }

  if (!activeMemory) return null

  return (
    <Dialog open={isDeleteOpen} onOpenChange={setDeleteOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete Memory</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete &quot;{activeMemory.title}&quot;?
            <br />
            <span className="text-muted-foreground text-xs">
              {activeMemory.scope}/{activeMemory.category}/{activeMemory.key}
            </span>
          </DialogDescription>
        </DialogHeader>

        <div className="py-4">
          <p className="text-sm text-muted-foreground">
            This will move the memory to the deleted files. You can recover it later from the deleted files view.
          </p>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => setDeleteOpen(false)}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={handleDelete}>
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// Combined Dialogs component
export function MemoriesDialogs() {
  return (
    <>
      <MemoryEditorDialog />
      <MemoryViewerDialog />
      <MemoryDeleteDialog />
    </>
  )
}
