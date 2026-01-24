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
} from "@buildscale/sdk"
import { useFileExplorer } from "./FileExplorerContext"
import { columns } from "./columns"
import type { LsEntry } from "./types"

export function FileExplorerList() {
  const { files, rowSelection, setRowSelection, setEditorOpen, setDeleteOpen, setActiveFile, navigate, currentPath, setViewerOpen } = useFileExplorer()
  const [sorting, setSorting] = React.useState<SortingState>([])

  const handleEdit = (file: LsEntry) => {
    setActiveFile(file)
    setEditorOpen(true)
  }

  const handleDelete = (file: LsEntry) => {
    setActiveFile(file)
    setDeleteOpen(true)
  }

  const handleView = (file: LsEntry) => {
    if (file.file_type === 'folder') {
      const newPath = currentPath === '/' ? `/${file.name}` : `${currentPath}/${file.name}`
      navigate(newPath)
    } else {
      setActiveFile(file)
      setViewerOpen(true)
    }
  }

  const handleRowClick = (file: LsEntry) => {
    if (file.file_type === 'folder') {
      // Navigate to folder
      const newPath = currentPath === '/' ? `/${file.name}` : `${currentPath}/${file.name}`
      navigate(newPath)
    } else {
      // View file
      handleView(file)
    }
  }

  const table = useReactTable({
    data: files,
    columns,
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
    },
  })

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
      <div className="flex-1 overflow-auto">
        <Table>
          <TableHeader>
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow key={headerGroup.id}>
                {headerGroup.headers.map((header) => {
                  return (
                    <TableHead 
                      key={header.id} 
                      style={{ width: header.getSize() }}
                      className={getColumnClassName(header.id)}
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
      <div className="flex items-center justify-end space-x-2 p-4 border-t text-sm text-muted-foreground">
        <div className="flex-1 text-sm text-muted-foreground">
          {table.getFilteredSelectedRowModel().rows.length} of{" "}
          {table.getFilteredRowModel().rows.length} row(s) selected.
        </div>
      </div>
    </div>
  )
}
