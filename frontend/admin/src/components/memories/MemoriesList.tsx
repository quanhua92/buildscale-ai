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
  cn,
} from "@buildscale/sdk"
import { useMemoriesExplorer } from "./MemoriesContext"
import { columns } from "./columns"
import type { MemoryEntry } from "./types"

export function MemoriesList() {
  const {
    memories,
    isLoading,
    rowSelection,
    setRowSelection,
    setViewerOpen,
    setEditorOpen,
    setDeleteOpen,
    setActiveMemory,
  } = useMemoriesExplorer()

  const [sorting, setSorting] = React.useState<SortingState>([])

  const handleView = (memory: MemoryEntry) => {
    setActiveMemory(memory)
    setViewerOpen(true)
  }

  const handleEdit = (memory: MemoryEntry) => {
    setActiveMemory(memory)
    setEditorOpen(true)
  }

  const handleDelete = (memory: MemoryEntry) => {
    setActiveMemory(memory)
    setDeleteOpen(true)
  }

  const handleRowClick = (memory: MemoryEntry) => {
    handleView(memory)
  }

  const table = useReactTable({
    data: memories,
    columns,
    getRowId: (row) => row.id,
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
      case 'scope':
        return 'hidden sm:table-cell'
      case 'category':
        return 'hidden md:table-cell'
      case 'tags':
        return 'hidden lg:table-cell'
      case 'updated_at':
        return 'hidden xl:table-cell'
      default:
        return ''
    }
  }

  if (isLoading) {
    return (
      <div className="rounded-md border bg-card text-card-foreground shadow-sm h-full flex items-center justify-center">
        <div className="text-muted-foreground">Loading memories...</div>
      </div>
    )
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
                      style={{ width: header.id === 'title' ? 'auto' : header.getSize() }}
                      className={cn(getColumnClassName(header.id), header.id === 'title' ? 'w-full' : '')}
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
                  <div className="flex flex-col items-center gap-2">
                    <div className="text-muted-foreground">No memories found</div>
                    <div className="text-sm text-muted-foreground">
                      Create your first memory to get started
                    </div>
                  </div>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  )
}
