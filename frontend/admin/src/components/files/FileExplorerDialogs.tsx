import React, { useEffect, useState } from 'react'
import { useFileExplorer } from './FileExplorerContext'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetFooter,
  Button,
  Input,
  Label,
} from "@buildscale/sdk"
import { Loader2 } from "lucide-react"

export function FileExplorerDialogs() {
  return (
    <>
      <FileEditor />
      <FileViewer />
      <DeleteDialog />
    </>
  )
}

function FileEditor() {
  const { isEditorOpen, setEditorOpen, activeFile, createFile, updateFile, currentPath } = useFileExplorer()
  const [name, setName] = useState('')
  const [content, setContent] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const nameId = React.useId()
  const contentId = React.useId()

  // Reset form when opening
  useEffect(() => {
    if (isEditorOpen) {
      if (activeFile) {
        setName(activeFile.name)
        // Ideally we fetch content here if editing, but for "write" tool we overwrite.
        // If we want to edit existing file content, we need to read it first.
        // Let's defer that logic to the Viewer or implement fetch-on-edit here.
        // For now, let's assume activeFile means we want to edit, so we should fetch content.
      } else {
        setName('')
        setContent('')
      }
    }
  }, [isEditorOpen, activeFile])
  
  // Fetch content for editing
  useEffect(() => {
    const fetchContent = async () => {
      if (isEditorOpen && activeFile) {
        // Implementation note: we need a way to read file content.
        // But we can't easily access the `readFile` from context inside useEffect if it's not stable or if we don't want to trigger loops.
        // Actually, we can.
      }
    }
    fetchContent()
  }, [isEditorOpen, activeFile])

  // We need to access readFile from context, but we can't use `useFileExplorer` inside this component again if we already destructured it.
  // Actually we can, or just use the props passed down if we unified it.
  // But `FileEditor` is inside `FileExplorerDialogs` which is inside `FileExplorer`.
  // So `useFileExplorer` works.
  
  // Let's handle the read inside the component body
  const { readFile } = useFileExplorer()
  
  useEffect(() => {
    let mounted = true
    const loadContent = async () => {
      if (isEditorOpen && activeFile) {
        const result = await readFile(activeFile.path)
        if (mounted && result && result.content) {
          // Handle content: could be string or object
          const text = typeof result.content === 'string' 
            ? result.content 
            : result.content.text || JSON.stringify(result.content, null, 2)
          setContent(text)
        }
      }
    }
    loadContent()
    return () => { mounted = false }
  }, [isEditorOpen, activeFile, readFile])

  const handleSave = async () => {
    setIsSaving(true)
    try {
      if (activeFile) {
        await updateFile(activeFile.path, content)
      } else {
        await createFile(name, content)
      }
      setEditorOpen(false)
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <Sheet open={isEditorOpen} onOpenChange={setEditorOpen}>
      <SheetContent className="w-full sm:max-w-[90vw]">
        <SheetHeader>
          <SheetTitle>{activeFile ? 'Edit File' : 'New File'}</SheetTitle>
          <SheetDescription>
            {activeFile ? `Editing ${activeFile.path}` : `Create a new file in ${currentPath}`}
          </SheetDescription>
        </SheetHeader>
        <div className="grid gap-4 py-4 h-[calc(100vh-180px)]">
          {!activeFile && (
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor={nameId} className="text-right">
                Name
              </Label>
              <Input
                id={nameId}
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="col-span-3"
                placeholder="filename.txt"
              />
            </div>
          )}
          <div className="flex flex-col gap-2 h-full">
            <Label htmlFor={contentId}>Content</Label>
            <textarea
              id={contentId}
              value={content}
              onChange={(e) => setContent(e.target.value)}
              className="flex-1 min-h-[300px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 font-mono"
            />
          </div>
        </div>
        <SheetFooter>
          <Button variant="outline" onClick={() => setEditorOpen(false)}>Cancel</Button>
          <Button onClick={handleSave} disabled={isSaving || (!activeFile && !name)}>
            {isSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Save
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  )
}

function FileViewer() {
  const { isViewerOpen, setViewerOpen, activeFile, readFile } = useFileExplorer()
  const [content, setContent] = useState<string>('')
  const [isLoading, setIsLoading] = useState(false)

  useEffect(() => {
    let mounted = true
    const loadContent = async () => {
      if (isViewerOpen && activeFile) {
        setIsLoading(true)
        try {
          const result = await readFile(activeFile.path)
          if (mounted && result) {
            const text = typeof result.content === 'string' 
              ? result.content 
              : result.content.text || JSON.stringify(result.content, null, 2)
            setContent(text)
          }
        } finally {
          if (mounted) setIsLoading(false)
        }
      }
    }
    loadContent()
    return () => { mounted = false }
  }, [isViewerOpen, activeFile, readFile])

  return (
    <Dialog open={isViewerOpen} onOpenChange={setViewerOpen}>
      <DialogContent className="w-[95vw] max-w-5xl max-h-[80vh] flex flex-col p-0 gap-0 sm:p-6 sm:gap-4">
        <DialogHeader className="p-4 sm:p-0 border-b sm:border-0">
          <DialogTitle>{activeFile?.name}</DialogTitle>
          <DialogDescription className="break-all">
            {activeFile?.path}
          </DialogDescription>
        </DialogHeader>
        <div className="flex-1 overflow-auto border-0 sm:border rounded-none sm:rounded-md p-4 bg-muted/30 font-mono text-sm whitespace-pre-wrap">
          {isLoading ? (
            <div className="flex items-center justify-center h-40">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : (
            content || <span className="text-muted-foreground italic">Empty file</span>
          )}
        </div>
        <DialogFooter className="p-4 sm:p-0 border-t sm:border-0">
          <Button onClick={() => setViewerOpen(false)}>Close</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function DeleteDialog() {
  const { isDeleteOpen, setDeleteOpen, activeFile, deleteItem } = useFileExplorer()
  const [isDeleting, setIsDeleting] = useState(false)

  const handleDelete = async () => {
    if (!activeFile) return
    setIsDeleting(true)
    try {
      await deleteItem(activeFile.path)
      setDeleteOpen(false)
    } finally {
      setIsDeleting(false)
    }
  }

  return (
    <Dialog open={isDeleteOpen} onOpenChange={setDeleteOpen}>
      <DialogContent className="w-[95vw] sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Delete {activeFile?.file_type === 'folder' ? 'Folder' : 'File'}?</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete <span className="font-medium text-foreground">{activeFile?.name}</span>?
            This action cannot be undone.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => setDeleteOpen(false)}>Cancel</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={isDeleting}>
            {isDeleting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
