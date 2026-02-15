import * as React from "react"
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
  getSortedRowModel,
  type SortingState,
} from "@tanstack/react-table"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Button,
  cn,
  useTools,
} from "@buildscale/sdk"
import { Trash2, Move } from "lucide-react"
import { useFileExplorer } from "./FileExplorerContext"
import { columns } from "./columns"
import type { LsEntry } from "./types"

export function FileExplorerList() {
  const { files, rowSelection, setRowSelection, setEditorOpen, setDeleteOpen, setMoveOpen, setActiveFile, navigate, setViewerOpen, workspaceId } = useFileExplorer()
  const { read } = useTools(workspaceId)
  const [sorting, setSorting] = React.useState<SortingState>([])

  const handleEdit = (file: LsEntry) => {
    setActiveFile(file)
    setEditorOpen(true)
  }

  const handleDelete = (file: LsEntry) => {
    setActiveFile(file)
    setDeleteOpen(true)
  }

  const handleMove = (file: LsEntry) => {
    setActiveFile(file)
    setMoveOpen(true)
  }

  const handleView = (file: LsEntry) => {
    if (file.file_type === 'folder') {
      navigate(file.path)
    } else {
      setActiveFile(file)
      setViewerOpen(true)
    }
  }

  const handleDownload = async (file: LsEntry) => {
    if (file.file_type === 'folder') return

    const result = await read(file.path)  // Uses default limit: 0 (unlimited)
    if (!result) return

    const content = typeof result.content === 'string'
      ? result.content
      : JSON.stringify(result.content, null, 2)

    const blob = new Blob([content], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = file.display_name || file.name
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleRowClick = (file: LsEntry) => {
    if (file.file_type === 'folder') {
      // Navigate to folder using its absolute path from the backend
      navigate(file.path)
    } else {
      // View file
      handleView(file)
    }
  }

  const table = useReactTable({
    data: files,
    columns,
    getRowId: (row) => row.path, // Use path as unique key since id can be null for unsynced files
    getCoreRowModel: getCoreRowModel(),
    onSortingChange: setSorting,
    getSortedRowModel: getSortedRowModel(),
    onRowSelectionChange: setRowSelection,
    state: {
      sorting,
      rowSelection,
    },
    meta: {
      onEdit: handleEdit,
      onDelete: handleDelete,
      onView: handleView,
      onMove: handleMove,
      onDownload: handleDownload,
    },
  })

  const selectedRows = table.getFilteredSelectedRowModel().rows
  const selectedCount = selectedRows.length

  const handleBatchDelete = () => {
    setActiveFile(null) // Signal batch mode to the dialog
    setDeleteOpen(true)
  }

  const handleBatchMove = () => {
    setActiveFile(null) // Signal batch mode to the dialog
    setMoveOpen(true)
  }

  const getColumnClassName = (columnId: string) => {
    switch (columnId) {
      case 'file_type':
        return 'hidden sm:table-cell'
      case 'updated_at':
        return 'hidden md:table-cell'
      default:
        return ''
    }
  }

  return (
    <div className="rounded-md border bg-card text-card-foreground shadow-sm h-full overflow-hidden flex flex-col">
      <div className="flex items-center justify-between p-4 border-b bg-muted/20 h-[65px]">
        <div className="text-sm text-muted-foreground">
          {selectedCount > 0 ? (
            <span className="font-medium text-foreground">{selectedCount} selected</span>
          ) : (
            "Select items to manage"
          )}
        </div>
        {selectedCount > 0 && (
          <div className="flex items-center gap-2">
            <Button size="sm" onClick={handleBatchMove} variant="outline" className="gap-2">
              <Move className="h-4 w-4" />
              Move
            </Button>
            <Button size="sm" onClick={handleBatchDelete} variant="outline" className="gap-2 text-destructive hover:text-destructive">
              <Trash2 className="h-4 w-4" />
              Move to Trash
            </Button>
          </div>
        )}
      </div>
      <div className="flex-1 overflow-auto">
        <Table>
          <TableHeader>
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow key={headerGroup.id}>
                {headerGroup.headers.map((header) => {
                  return (
                    <TableHead 
                      key={header.id} 
                      style={{ width: header.id === 'name' ? 'auto' : header.getSize() }}
                      className={cn(getColumnClassName(header.id), header.id === 'name' ? 'w-full' : '')}
                    >
                      {header.isPlaceholder
                        ? null
                        : flexRender(
                            header.column.columnDef.header,
                            header.getContext()
                          )}
                    </TableHead>
                  )
                })}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {table.getRowModel().rows?.length ? (
              table.getRowModel().rows.map((row) => (
                <TableRow
                  key={row.id}
                  data-state={row.getIsSelected() && "selected"}
                  onClick={() => handleRowClick(row.original)}
                  className="cursor-pointer hover:bg-muted/50"
                >
                  {row.getVisibleCells().map((cell) => (
                    <TableCell 
                      key={cell.id}
                      className={getColumnClassName(cell.column.id)}
                    >
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </TableCell>
                  ))}
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell colSpan={columns.length} className="h-24 text-center">
                  No files found.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  )
}
