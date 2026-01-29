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
  type File
} from "@buildscale/sdk"
import { RotateCcw } from "lucide-react"
import { columns } from "./columns"

interface DeletedFilesListProps {
  files: File[]
  onRestore: (file: File) => void
  onRestoreBatch?: (files: File[]) => void
  onPurge: (file: File) => void
}

export function DeletedFilesList({ files, onRestore, onRestoreBatch, onPurge }: DeletedFilesListProps) {
  const [sorting, setSorting] = React.useState<SortingState>([])
  const [rowSelection, setRowSelection] = React.useState({})

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
      onRestore,
      onPurge,
    },
  })

  const selectedRows = table.getFilteredSelectedRowModel().rows
  const selectedCount = selectedRows.length

  const handleBatchAction = () => {
    if (onRestoreBatch) {
      const selectedFiles = selectedRows.map(row => row.original)
      onRestoreBatch(selectedFiles)
      setRowSelection({}) // Clear selection after action
    }
  }

  return (
    <div className="rounded-md border bg-card text-card-foreground shadow-sm h-full overflow-hidden flex flex-col">
      <div className="flex items-center justify-between p-4 border-b bg-muted/20">
        <div className="text-sm text-muted-foreground">
          {selectedCount > 0 ? (
            <span className="font-medium text-foreground">{selectedCount} selected</span>
          ) : (
            "Select items to restore"
          )}
        </div>
        {selectedCount > 0 && (
          <Button size="sm" onClick={handleBatchAction} variant="outline" className="gap-2">
            <RotateCcw className="h-4 w-4" />
            Restore Selected
          </Button>
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
                      className={header.id === 'name' ? 'w-full' : ''}
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
                  className="hover:bg-muted/50"
                >
                  {row.getVisibleCells().map((cell) => (
                    <TableCell key={cell.id}>
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </TableCell>
                  ))}
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell colSpan={columns.length} className="h-24 text-center">
                  No recently deleted files.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  )
}
