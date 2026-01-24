import React from 'react'
import { useFileExplorer } from './FileExplorerContext'
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
  Button,
} from "@buildscale/sdk"
import { RefreshCw, FolderPlus, FilePlus, ChevronRight, Home } from "lucide-react"

export function FileExplorerToolbar() {
  const { currentPath, navigate, refresh, isLoading, setEditorOpen, setActiveFile, createFolder } = useFileExplorer()

  const pathParts = currentPath.split('/').filter(Boolean)
  
  const handleNavigate = (index: number) => {
    if (index === -1) {
      navigate('/')
      return
    }
    const newPath = '/' + pathParts.slice(0, index + 1).join('/')
    navigate(newPath)
  }

  const handleNewFile = () => {
    setActiveFile(null)
    setEditorOpen(true)
  }

  const handleNewFolder = async () => {
    const folderName = prompt("Enter folder name:")
    if (folderName) {
      await createFolder(folderName)
    }
  }

  return (
    <div className="flex items-center justify-between p-2 gap-2 overflow-hidden">
      <div className="flex items-center flex-1 overflow-x-auto min-w-0 no-scrollbar mask-fade-right">
        <Breadcrumb className="whitespace-nowrap">
          <BreadcrumbList className="flex-nowrap">
            <BreadcrumbItem>
              <BreadcrumbLink 
                className="cursor-pointer flex items-center"
                onClick={() => handleNavigate(-1)}
              >
                <Home className="h-4 w-4" />
              </BreadcrumbLink>
            </BreadcrumbItem>
            {pathParts.map((part, index) => {
              const isLast = index === pathParts.length - 1
              return (
                <React.Fragment key={`${index}-${part}`}>
                  <BreadcrumbSeparator>
                    <ChevronRight className="h-4 w-4" />
                  </BreadcrumbSeparator>
                  <BreadcrumbItem>
                    {isLast ? (
                      <BreadcrumbPage>{part}</BreadcrumbPage>
                    ) : (
                      <BreadcrumbLink 
                        className="cursor-pointer"
                        onClick={() => handleNavigate(index)}
                      >
                        {part}
                      </BreadcrumbLink>
                    )}
                  </BreadcrumbItem>
                </React.Fragment>
              )
            })}
          </BreadcrumbList>
        </Breadcrumb>
      </div>
      
      <div className="flex items-center gap-1 sm:gap-2 shrink-0">
        <Button variant="ghost" size="icon" onClick={refresh} disabled={isLoading}>
          <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
        </Button>
        <Button variant="outline" size="sm" onClick={handleNewFolder} className="h-8 w-8 sm:w-auto px-0 sm:px-3">
          <FolderPlus className="h-4 w-4 sm:mr-2" />
          <span className="hidden sm:inline">New Folder</span>
        </Button>
        <Button size="sm" onClick={handleNewFile} className="h-8 w-8 sm:w-auto px-0 sm:px-3">
          <FilePlus className="h-4 w-4 sm:mr-2" />
          <span className="hidden sm:inline">New File</span>
        </Button>
      </div>
    </div>
  )
}
