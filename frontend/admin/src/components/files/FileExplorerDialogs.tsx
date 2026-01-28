import React, { useEffect, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useFileExplorer } from './FileExplorerContext'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Button,
  Input,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  toast,
} from "@buildscale/sdk"
import { Loader2 } from "lucide-react"
import { getContentAsString } from './utils'

export function FileExplorerDialogs() {
  return (
    <>
      <FileEditor />
      <FileViewer />
      <NewFolderDialog />
      <DeleteDialog />
    </>
  )
}

function NewFolderDialog() {
  const { isFolderOpen, setFolderOpen, createFolder, currentPath } = useFileExplorer()
  const [name, setName] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const nameId = React.useId()

  useEffect(() => {
    if (isFolderOpen) {
      setName('')
    }
  }, [isFolderOpen])

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name) return
    setIsSaving(true)
    try {
      await createFolder(name)
      setFolderOpen(false)
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <Dialog open={isFolderOpen} onOpenChange={setFolderOpen}>
      <DialogContent className="w-[95vw] sm:max-w-md">
        <form onSubmit={handleSave}>
          <DialogHeader>
            <DialogTitle>New Folder</DialogTitle>
            <DialogDescription>
              Create a new folder in {currentPath}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor={nameId}>Name</Label>
              <Input
                id={nameId}
                value={name}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
                placeholder="Folder name"
                autoFocus
              />
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setFolderOpen(false)}>Cancel</Button>
            <Button type="submit" disabled={isSaving || !name}>
              {isSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Create
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

function FileEditor() {
  const { isEditorOpen, setEditorOpen, activeFile, createFile, updateFile, currentPath } = useFileExplorer()
  const [name, setName] = useState('')
  const [content, setContent] = useState('')
  const [fileType, setFileType] = useState('document')
  const [isSaving, setIsSaving] = useState(false)
  const nameId = React.useId()
  const typeId = React.useId()
  const contentId = React.useId()

  // Reset form when opening
  useEffect(() => {
    if (isEditorOpen) {
      if (activeFile) {
        setName(activeFile.name)
        setFileType(activeFile.file_type)
        setContent('') // Reset content while loading
      } else {
        setName('')
        setContent('')
        setFileType('document')
      }
    }
  }, [isEditorOpen, activeFile])
  
  // We need to access readFile from context
  const { readFile } = useFileExplorer()
  
  useEffect(() => {
    let mounted = true
    const loadContent = async () => {
      if (isEditorOpen && activeFile && activeFile.file_type !== 'folder') {
        const result = await readFile(activeFile.path)
        if (mounted && result) {
          setContent(getContentAsString(result.content))
        }
      }
    }
    loadContent()
    return () => { mounted = false }
  }, [isEditorOpen, activeFile, readFile])

  const handleSave = async () => {
    setIsSaving(true)
    try {
      // Construct appropriate content structure based on file type
      let structuredContent: any
      
      if (fileType === 'document') {
        structuredContent = { text: content }
      } else {
        try {
          // For specialized types, content must be valid JSON. 
          // Treat empty content as empty object.
          structuredContent = content.trim() === '' ? {} : JSON.parse(content)
        } catch (e) {
          toast.error(`Invalid JSON content for ${fileType}. Please check your syntax.`)
          setIsSaving(false)
          return
        }
      }

      if (activeFile) {
        await updateFile(activeFile.path, structuredContent)
      } else {
        await createFile(name, structuredContent, fileType)
      }
      setEditorOpen(false)
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <Dialog open={isEditorOpen} onOpenChange={setEditorOpen}>
      <DialogContent className="w-[95vw] max-w-5xl max-h-[90vh] flex flex-col p-0 gap-0 sm:p-6 sm:gap-4">
        <DialogHeader className="p-4 sm:p-0 border-b sm:border-0">
          <DialogTitle>{activeFile ? 'Edit File' : 'New File'}</DialogTitle>
          <DialogDescription className="break-all">
            {activeFile ? `Editing ${activeFile.path}` : `Create a new file in ${currentPath}`}
          </DialogDescription>
        </DialogHeader>
        <div className="flex-1 overflow-y-auto p-4 sm:p-0 flex flex-col gap-4">
          {!activeFile && (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div className="flex flex-col gap-2">
                <Label htmlFor={nameId}>Name</Label>
                <Input
                  id={nameId}
                  value={name}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
                  placeholder="filename.txt"
                />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor={typeId}>Type</Label>
                <Select value={fileType} onValueChange={setFileType}>
                  <SelectTrigger id={typeId}>
                    <SelectValue placeholder="Select type" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="document">Document</SelectItem>
                    <SelectItem value="canvas">Canvas</SelectItem>
                    <SelectItem value="chat">Chat</SelectItem>
                    <SelectItem value="whiteboard">Whiteboard</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          )}
          <div className="flex flex-col gap-2 flex-1 min-h-[200px]">
            <Label htmlFor={contentId}>Content</Label>
            <textarea
              id={contentId}
              value={content}
              onChange={(e) => setContent(e.target.value)}
              className="flex-1 w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 font-mono resize-none"
            />
          </div>
        </div>
        <DialogFooter className="p-4 sm:p-0 border-t sm:border-0 mt-auto">
          <Button variant="outline" onClick={() => setEditorOpen(false)}>Cancel</Button>
          <Button onClick={handleSave} disabled={isSaving || (!activeFile && !name)}>
            {isSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function FileViewer() {
  const { isViewerOpen, setViewerOpen, activeFile, readFile, workspaceId } = useFileExplorer()
  const [content, setContent] = useState<string>('')
  const [isLoading, setIsLoading] = useState(false)
  const navigate = useNavigate()

  useEffect(() => {
    let mounted = true
    const loadContent = async () => {
      if (isViewerOpen && activeFile) {
        setIsLoading(true)
        try {
          const result = await readFile(activeFile.path)
          if (mounted && result) {
            setContent(getContentAsString(result.content))
          }
        } finally {
          if (mounted) setIsLoading(false)
        }
      }
    }
    loadContent()
    return () => { mounted = false }
  }, [isViewerOpen, activeFile, readFile])

  const isChat = activeFile?.file_type === 'chat' || (activeFile?.is_virtual && activeFile?.name.startsWith('chat-'))

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
        <DialogFooter className="p-4 sm:p-0 border-t sm:border-0 flex flex-col gap-2 sm:flex-col sm:space-x-0">
          {isChat && activeFile && (
            <Button 
              className="w-full"
              onClick={() => {
                setViewerOpen(false)
                navigate({
                  to: '/workspaces/$workspaceId/chat',
                  params: { workspaceId },
                  search: { chatId: activeFile.id }
                })
              }}
            >
              Continue with this chat
            </Button>
          )}
          <Button className="w-full" onClick={() => setViewerOpen(false)}>Close</Button>
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
