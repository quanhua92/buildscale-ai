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

// Form field wrapper - responsive label/input layout
function FormField({ label, htmlFor, children }: { label: string; htmlFor: string; children: React.ReactNode }) {
  return (
    <div className="grid gap-2 sm:grid-cols-4 sm:items-center sm:gap-4">
      <Label htmlFor={htmlFor} className="sm:text-right">{label}</Label>
      <div className="sm:col-span-3">{children}</div>
    </div>
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
      <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
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
          <FormField label="Scope" htmlFor="scope">
            <Select value={scope} onValueChange={(v: 'user' | 'global') => setScope(v)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="user">User (Private)</SelectItem>
                <SelectItem value="global">Global (Shared)</SelectItem>
              </SelectContent>
            </Select>
          </FormField>

          {/* Category */}
          <FormField label="Category" htmlFor="category">
            {isNewCategory ? (
              <div className="flex gap-2">
                <Input
                  id="category"
                  value={category}
                  onChange={(e) => setCategory(e.target.value)}
                  placeholder="e.g., preferences, project"
                  className="flex-1"
                />
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setIsNewCategory(false)}
                  className="shrink-0"
                >
                  Select
                </Button>
              </div>
            ) : (
              <div className="flex gap-2">
                <Select value={category} onValueChange={setCategory}>
                  <SelectTrigger className="flex-1">
                    <SelectValue placeholder="Select category" />
                  </SelectTrigger>
                  <SelectContent>
                    {categories.length > 0 ? (
                      categories.map((cat) => (
                        <SelectItem key={cat} value={cat}>{cat}</SelectItem>
                      ))
                    ) : (
                      <SelectItem value="_none" disabled>No categories yet - click New</SelectItem>
                    )}
                  </SelectContent>
                </Select>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setCategory('')
                    setIsNewCategory(true)
                  }}
                  className="shrink-0"
                >
                  New
                </Button>
              </div>
            )}
          </FormField>

          {/* Key */}
          <FormField label="Key" htmlFor="key">
            <Input
              id="key"
              value={key}
              onChange={(e) => setKey(e.target.value)}
              placeholder="e.g., coding-style, api-endpoints"
              disabled={isEditing}
            />
          </FormField>

          {/* Title */}
          <FormField label="Title" htmlFor="title">
            <Input
              id="title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Human-readable title"
            />
          </FormField>

          {/* Content */}
          <div className="grid gap-2 sm:grid-cols-4 sm:items-start sm:gap-4">
            <Label htmlFor="content" className="sm:text-right sm:pt-2">Content</Label>
            <div className="sm:col-span-3">
              <Textarea
                id="content"
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder="Memory content in markdown..."
                className="min-h-[150px] sm:min-h-[200px]"
              />
            </div>
          </div>

          {/* Tags */}
          <FormField label="Tags" htmlFor="tags">
            <Input
              id="tags"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="Comma-separated (optional)"
            />
          </FormField>
        </div>

        <DialogFooter className="flex-col gap-2 sm:flex-row">
          <Button variant="outline" onClick={() => setEditorOpen(false)} className="w-full sm:w-auto">
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={!category || !key || !title || !content} className="w-full sm:w-auto">
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
      <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="pr-8">{activeMemory.title}</DialogTitle>
          <DialogDescription>
            <span className="font-mono text-xs">
              {activeMemory.scope}/{activeMemory.category}/{activeMemory.key}
            </span>
          </DialogDescription>
        </DialogHeader>

        <div className="py-4">
          {isLoading ? (
            <div className="text-muted-foreground text-center py-8">Loading...</div>
          ) : (
            <pre className="whitespace-pre-wrap text-sm bg-muted p-3 sm:p-4 rounded-md overflow-auto max-h-[50vh]">
              {content}
            </pre>
          )}

          {activeMemory.tags.length > 0 && (
            <div className="mt-4 flex flex-wrap items-center gap-2">
              <span className="text-sm text-muted-foreground">Tags:</span>
              {activeMemory.tags.map((tag) => (
                <span
                  key={tag}
                  className="inline-flex items-center px-2 py-0.5 text-xs bg-muted rounded"
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>

        <DialogFooter className="flex-col gap-2 sm:flex-row">
          <Button variant="outline" onClick={handleDelete} className="w-full sm:w-auto text-destructive hover:text-destructive">
            Delete
          </Button>
          <Button onClick={handleEdit} className="w-full sm:w-auto">
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

        <DialogFooter className="flex-col gap-2 sm:flex-row">
          <Button variant="outline" onClick={() => setDeleteOpen(false)} className="w-full sm:w-auto">
            Cancel
          </Button>
          <Button variant="destructive" onClick={handleDelete} className="w-full sm:w-auto">
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
