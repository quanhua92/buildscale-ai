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
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
  useTools,
} from "@buildscale/sdk"
import { Loader2, FolderIcon, ChevronRight, Home } from "lucide-react"
import { getContentAsString } from './utils'
import type { LsResult, LsEntry } from './types'

export function FileExplorerDialogs() {
  return (
    <>
      <FileEditor />
      <FileViewer />
      <NewFolderDialog />
      <DeleteDialog />
      <MoveDialog />
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
  const { isViewerOpen, setViewerOpen, activeFile, workspaceId } = useFileExplorer()
  const { read } = useTools(workspaceId)
  const [content, setContent] = useState<string>('')
  const [isLoading, setIsLoading] = useState(false)
  const navigate = useNavigate()

  useEffect(() => {
    let mounted = true
    const loadContent = async () => {
      if (isViewerOpen && activeFile) {
        setIsLoading(true)
        try {
          const result = await read(activeFile.path)  // Uses default limit: 0 (unlimited)
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
  }, [isViewerOpen, activeFile, read])

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
          {isChat && activeFile && activeFile.id && (
            <Button
              className="w-full"
              onClick={() => {
                setViewerOpen(false)
                const chatId = activeFile.id! // Non-null assertion since we checked above
                navigate({
                  to: '/workspaces/$workspaceId/chat',
                  params: { workspaceId },
                  search: { chatId }
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
  const { isDeleteOpen, setDeleteOpen, activeFile, deleteItems, selectedPaths } = useFileExplorer()
  const [isDeleting, setIsDeleting] = useState(false)

  const pathsToDelete = React.useMemo(() => {
    return activeFile ? [activeFile.path] : selectedPaths
  }, [activeFile, selectedPaths])

  const handleDelete = async () => {
    if (pathsToDelete.length === 0) return
    setIsDeleting(true)
    try {
      await deleteItems(pathsToDelete)
      setDeleteOpen(false)
    } finally {
      setIsDeleting(false)
    }
  }

  const title = activeFile 
    ? `Move ${activeFile.file_type === 'folder' ? 'Folder' : 'File'} to Trash?`
    : `Move ${pathsToDelete.length} items to Trash?`

  return (
    <Dialog open={isDeleteOpen} onOpenChange={setDeleteOpen}>
      <DialogContent className="w-[95vw] sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>
            {activeFile ? (
              <>Are you sure you want to move <span className="font-medium text-foreground">{activeFile.name}</span> to the trash?</>
            ) : (
              <>Are you sure you want to move the selected {pathsToDelete.length} items to the trash?</>
            )}
            <br />
            Items can be recovered from the <span className="font-medium text-foreground">Recently Deleted</span> page.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => setDeleteOpen(false)}>Cancel</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={isDeleting || pathsToDelete.length === 0}>
            {isDeleting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Move to Trash
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function MoveDialog() {
  const { isMoveOpen, setMoveOpen, activeFile, moveItems, selectedEntries, workspaceId } = useFileExplorer()
  const { ls } = useTools(workspaceId)
  const [browsingPath, setBrowsingPath] = useState('/')
  const [folders, setFolders] = useState<LsResult | null>(null)
  const [isLoadingFolders, setIsLoadingFolders] = useState(false)
  const [isMoving, setIsMoving] = useState(false)

  const itemsToMove = React.useMemo(() => {
    return activeFile ? [activeFile] : selectedEntries
  }, [activeFile, selectedEntries])

  const fetchFolders = React.useCallback(async (path: string) => {
    setIsLoadingFolders(true)
    try {
      // Use limit: 0 to get all entries for move dialog
      const result = await ls(path, { limit: 0 })
      if (result) {
        // Filter only folders
        const filtered: LsResult = {
          ...result,
          entries: (result.entries as LsEntry[]).filter((e: LsEntry) => e.file_type === 'folder')
        }
        setFolders(filtered)
      }
    } finally {
      setIsLoadingFolders(false)
    }
  }, [ls])

  useEffect(() => {
    if (isMoveOpen) {
      setBrowsingPath('/')
      fetchFolders('/')
    }
  }, [isMoveOpen, fetchFolders])

  const navigateTo = (path: string) => {
    setBrowsingPath(path)
    fetchFolders(path)
  }

  const pathParts = browsingPath.split('/').filter(Boolean)
  const handleBreadcrumbClick = (index: number) => {
    if (index === -1) {
      navigateTo('/')
      return
    }
    const newPath = '/' + pathParts.slice(0, index + 1).join('/')
    navigateTo(newPath)
  }

  const handleMove = async () => {
    if (itemsToMove.length === 0) return
    setIsMoving(true)
    try {
      const sourcePaths = itemsToMove.map(e => e.path)
      await moveItems(sourcePaths, browsingPath)
      setMoveOpen(false)
    } finally {
      setIsMoving(false)
    }
  }

  // Safety checks
  const isInvalidDestination = React.useMemo(() => {
    return itemsToMove.some(entry => {
      // An item cannot be moved to the directory it is already in.
      const lastSlash = entry.path.lastIndexOf('/')
      const parentPath = lastSlash <= 0 ? '/' : entry.path.substring(0, lastSlash)
      if (parentPath === browsingPath) {
        return true
      }

      // The following checks are only relevant when moving a folder.
      if (entry.file_type === 'folder') {
        // A folder cannot be moved into itself.
        if (entry.path === browsingPath) {
          return true
        }
        // A folder cannot be moved into one of its own sub-folders.
        if (browsingPath.startsWith(`${entry.path}/`)) {
          return true
        }
      }
      
      return false
    })
  }, [itemsToMove, browsingPath])

  return (
    <Dialog open={isMoveOpen} onOpenChange={setMoveOpen}>
      <DialogContent className="w-[95vw] sm:max-w-2xl max-h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="p-6 pb-2">
          <DialogTitle>Move {itemsToMove.length === 1 ? 'Item' : `${itemsToMove.length} Items`}</DialogTitle>
          <DialogDescription>
            Choose a destination folder in this workspace
          </DialogDescription>
        </DialogHeader>

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

        <div className="flex-1 overflow-y-auto min-h-[300px]">
          {isLoadingFolders ? (
            <div className="flex items-center justify-center h-full">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground opacity-20" />
            </div>
          ) : (
            <div className="divide-y">
              {folders?.entries.length === 0 ? (
                <div className="p-8 text-center text-sm text-muted-foreground italic">
                  No subfolders found
                </div>
              ) : (
                folders?.entries.map(folder => (
                  <button
                    key={folder.id}
                    type="button"
                    onClick={() => navigateTo(folder.path)}
                    className="w-full flex items-center gap-3 px-6 py-3 hover:bg-muted text-sm transition-colors text-left group"
                  >
                    <FolderIcon className="h-4 w-4 text-blue-500 fill-blue-500/10" />
                    <span className="flex-1 font-medium truncate">{folder.name}</span>
                    <ChevronRight className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
                  </button>
                ))
              )}
            </div>
          )}
        </div>

        <DialogFooter className="p-6 border-t bg-muted/10">
          <div className="flex-1 text-xs text-muted-foreground truncate mr-4">
            Destination: <span className="font-mono text-foreground">{browsingPath}</span>
          </div>
          <div className="flex gap-2 shrink-0">
            <Button variant="outline" onClick={() => setMoveOpen(false)}>Cancel</Button>
            <Button 
              onClick={handleMove} 
              disabled={isMoving || isInvalidDestination || itemsToMove.length === 0}
              className="min-w-[100px]"
            >
              {isMoving ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : 'Move Here'}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

