import type { ColumnDef } from "@tanstack/react-table"
import { Checkbox } from "@/components/ui/checkbox"
import type { LsEntry } from "./types"
import { FolderIcon, FileTextIcon, MoreHorizontal, Pencil, Trash, Eye, Presentation, MessageSquare, Monitor } from "lucide-react"
import { formatDateTime } from "@buildscale/sdk"
import { Button } from "@buildscale/sdk"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@buildscale/sdk"

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
      const fileType = row.original.file_type
      let Icon = FileTextIcon
      let iconColor = "text-gray-500"

      switch (fileType) {
        case 'folder':
          Icon = FolderIcon
          iconColor = "text-blue-500"
          break
        case 'canvas':
          Icon = Presentation
          iconColor = "text-purple-500"
          break
        case 'chat':
          Icon = MessageSquare
          iconColor = "text-green-500"
          break
        case 'whiteboard':
          Icon = Monitor
          iconColor = "text-orange-500"
          break
      }
      
      return (
        <div className="flex items-center gap-2">
          <Icon className={`h-4 w-4 ${iconColor}`} />
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
      // We need a way to call the context functions. 
      // Since columns are defined outside the component, we can pass handlers via table.options.meta 
      // OR we can rely on the row actions in the parent component.
      // A common pattern is to just use a standard dropdown here that triggers events.
      // But we can't easily access the `deleteItem` from the context here without passing it down.
      // For simplicity, we will emit custom events or rely on the `meta` feature of TanStack Table.
      
      const meta = table.options.meta as any
      
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
                meta?.onView(entry)
              }}>
                <Eye className="mr-2 h-4 w-4" />
                {entry.file_type === 'folder' ? 'Open' : 'View'}
              </DropdownMenuItem>
              {entry.file_type !== 'folder' && (
                <DropdownMenuItem onClick={(e) => {
                  e.stopPropagation()
                  meta?.onEdit(entry)
                }}>
                  <Pencil className="mr-2 h-4 w-4" />
                  Edit
                </DropdownMenuItem>
              )}
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={(e) => {
                e.stopPropagation()
                meta?.onDelete(entry)
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
