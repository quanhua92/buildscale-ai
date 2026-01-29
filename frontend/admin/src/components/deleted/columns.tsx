import type { ColumnDef } from "@tanstack/react-table"
import { Checkbox } from "@/components/ui/checkbox"
import type { File } from "@buildscale/sdk"
import { FolderIcon, FileTextIcon, MoreHorizontal, RotateCcw, Presentation, MessageSquare, Monitor, Trash2 } from "lucide-react"
import { formatDate, formatTime, Button } from "@buildscale/sdk"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@buildscale/sdk"

import '@tanstack/react-table'

// Extend table meta to include onRestore
declare module '@tanstack/react-table' {
  interface TableMeta<TData> {
    onRestore?: (file: TData) => void
    onPurge?: (file: TData) => void
  }
}

export const columns: ColumnDef<File>[] = [
  {
    id: "select",
    header: ({ table }) => (
      <Checkbox
        checked={table.getIsAllPageRowsSelected()}
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="Select all"
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={row.getIsSelected()}
        onCheckedChange={(value) => row.toggleSelected(!!value)}
        aria-label="Select row"
        onClick={(e) => e.stopPropagation()}
      />
    ),
    enableSorting: false,
    enableHiding: false,
    size: 40,
  },
  {
    accessorKey: "name",
    header: "Name",
    size: 500, // Give Name column more space
    cell: ({ row }) => {
      const fileType = row.original.file_type as string
      
      const config: Record<string, { Icon: any; color: string }> = {
        folder: { Icon: FolderIcon, color: "text-blue-500" },
        canvas: { Icon: Presentation, color: "text-purple-500" },
        chat: { Icon: MessageSquare, color: "text-green-500" },
        whiteboard: { Icon: Monitor, color: "text-orange-500" },
      }

      const { Icon, color } = config[fileType] || { Icon: FileTextIcon, color: "text-gray-500" }
      
      return (
        <div className="flex items-center gap-2">
          <Icon className={`h-4 w-4 ${color}`} />
          <span className="font-medium">{row.getValue("name")}</span>
        </div>
      )
    },
  },
  {
    accessorKey: "file_type",
    header: "Type",
    cell: ({ row }) => {
      return <div className="capitalize text-muted-foreground">{row.getValue("file_type")}</div>
    },
    size: 100,
  },
  {
    accessorKey: "deleted_at",
    header: "Date Deleted",
    cell: ({ row }) => {
      const date = row.getValue("deleted_at") as string
      return (
        <div className="text-muted-foreground whitespace-nowrap">
          <span>{formatDate(date)}</span>
          <span className="ml-2 text-xs opacity-70">{formatTime(date)}</span>
        </div>
      )
    },
    size: 150,
  },
  {
    id: "actions",
    cell: ({ row, table }) => {
      const entry = row.original
      const meta = table.options.meta
      
      return (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="h-8 w-8 p-0" onClick={(e) => e.stopPropagation()}>
              <span className="sr-only">Open menu</span>
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={(e) => {
              e.stopPropagation()
              meta?.onRestore?.(entry)
            }}>
              <RotateCcw className="mr-2 h-4 w-4" />
              Restore
            </DropdownMenuItem>
            <DropdownMenuItem onClick={(e) => {
              e.stopPropagation()
              meta?.onPurge?.(entry)
            }} className="text-destructive focus:text-destructive">
              <Trash2 className="mr-2 h-4 w-4" />
              Delete Forever
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      )
    },
    size: 50,
  },
]
