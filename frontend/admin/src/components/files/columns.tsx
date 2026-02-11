import type { ColumnDef } from "@tanstack/react-table"
import { Checkbox } from "@/components/ui/checkbox"
import type { LsEntry } from "./types"
import { FolderIcon, FileTextIcon, MoreHorizontal, Pencil, Trash, Eye, Presentation, MessageSquare, Monitor, Move, CheckCircle, CloudOff } from "lucide-react"
import { formatDateTime, Button } from "@buildscale/sdk"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@buildscale/sdk"

import '@tanstack/react-table'

declare module '@tanstack/react-table' {
  interface TableMeta<TData> {
    onEdit?: (file: TData) => void
    onDelete?: (file: TData) => void
    onView?: (file: TData) => void
    onMove?: (file: TData) => void
  }
}

// Sync status indicator component
function SyncStatusIndicator({ synced }: { synced: boolean }) {
  return (
    <div className="flex items-center" title={synced ? "Synced to database" : "Not synced - exists on disk only"}>
      {synced ? (
        <CheckCircle className="h-4 w-4 text-green-500" />
      ) : (
        <CloudOff className="h-4 w-4 text-yellow-500" />
      )}
    </div>
  )
}

export const columns: ColumnDef<LsEntry>[] = [
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
    cell: ({ row }) => {
      const entry = row.original
      const fileType = entry.file_type

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
          <span className="font-medium">{entry.display_name || entry.name}</span>
        </div>
      )
    },
  },
  {
    accessorKey: "synced",
    header: "Sync",
    cell: ({ row }) => {
      const entry = row.original
      return <SyncStatusIndicator synced={entry.synced ?? true} />
    },
    size: 50,
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
    accessorKey: "updated_at",
    header: "Last Modified",
    cell: ({ row }) => {
      return <div className="text-muted-foreground whitespace-nowrap">{formatDateTime(row.getValue("updated_at"))}</div>
    },
    size: 180,
  },
  {
    id: "actions",
    cell: ({ row, table }) => {
      const entry = row.original
      const meta = table.options.meta

      return (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="h-8 w-8 p-0" onClick={(e) => {
              e.stopPropagation()
            }}>
              <span className="sr-only">Open menu</span>
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={(e) => {
              e.stopPropagation()
              meta?.onView?.(entry)
            }}>
              <Eye className="mr-2 h-4 w-4" />
              {entry.file_type === 'folder' ? 'Open' : 'View'}
            </DropdownMenuItem>
            {entry.file_type !== 'folder' && (
              <DropdownMenuItem onClick={(e) => {
                e.stopPropagation()
                meta?.onEdit?.(entry)
              }}>
                <Pencil className="mr-2 h-4 w-4" />
                Edit
              </DropdownMenuItem>
            )}
            <DropdownMenuItem onClick={(e) => {
              e.stopPropagation()
              meta?.onMove?.(entry)
            }}>
              <Move className="mr-2 h-4 w-4" />
              Move
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={(e) => {
              e.stopPropagation()
              meta?.onDelete?.(entry)
            }} className="text-destructive focus:text-destructive">
              <Trash className="mr-2 h-4 w-4" />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      )
    },
    size: 50,
  },
]
