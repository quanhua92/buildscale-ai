import type { ColumnDef } from "@tanstack/react-table"
import { Checkbox } from "@/components/ui/checkbox"
import type { MemoryEntry } from "./types"
import { formatDateTime, Button } from "@buildscale/sdk"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@buildscale/sdk"
import { MoreHorizontal, Pencil, Trash, Eye, Lock, Globe, Tag } from "lucide-react"

import '@tanstack/react-table'

declare module '@tanstack/react-table' {
  interface TableMeta<TData> {
    onEdit?: (memory: TData) => void
    onDelete?: (memory: TData) => void
    onView?: (memory: TData) => void
  }
}

// Scope indicator component
function ScopeIndicator({ scope }: { scope: 'user' | 'global' }) {
  return scope === 'user' ? (
    <div className="flex items-center gap-1 text-blue-500" title="Private to you">
      <Lock className="h-3.5 w-3.5" />
      <span className="text-xs">User</span>
    </div>
  ) : (
    <div className="flex items-center gap-1 text-green-500" title="Shared with workspace">
      <Globe className="h-3.5 w-3.5" />
      <span className="text-xs">Global</span>
    </div>
  )
}

// Tags display component
function TagsDisplay({ tags }: { tags: string[] }) {
  if (!tags || tags.length === 0) return null

  return (
    <div className="flex items-center gap-1 flex-wrap">
      {tags.slice(0, 3).map((tag) => (
        <span
          key={tag}
          className="inline-flex items-center px-1.5 py-0.5 text-xs bg-muted rounded"
        >
          {tag}
        </span>
      ))}
      {tags.length > 3 && (
        <span className="text-xs text-muted-foreground">+{tags.length - 3}</span>
      )}
    </div>
  )
}

export const columns: ColumnDef<MemoryEntry>[] = [
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
    accessorKey: "scope",
    header: "Scope",
    cell: ({ row }) => {
      const entry = row.original
      return <ScopeIndicator scope={entry.scope} />
    },
    size: 100,
  },
  {
    accessorKey: "category",
    header: "Category",
    cell: ({ row }) => {
      return (
        <div className="font-medium text-muted-foreground">
          {row.getValue("category")}
        </div>
      )
    },
    size: 120,
  },
  {
    accessorKey: "key",
    header: "Key",
    cell: ({ row }) => {
      return <div className="font-mono text-sm">{row.getValue("key")}</div>
    },
    size: 150,
  },
  {
    accessorKey: "title",
    header: "Title",
    cell: ({ row }) => {
      return <div className="font-medium">{row.getValue("title")}</div>
    },
    size: 200,
  },
  {
    accessorKey: "tags",
    header: () => (
      <div className="flex items-center gap-1">
        <Tag className="h-3.5 w-3.5" />
        <span>Tags</span>
      </div>
    ),
    cell: ({ row }) => {
      const entry = row.original
      return <TagsDisplay tags={entry.tags} />
    },
    size: 150,
  },
  {
    accessorKey: "updated_at",
    header: "Updated",
    cell: ({ row }) => {
      return (
        <div className="text-muted-foreground whitespace-nowrap">
          {formatDateTime(row.getValue("updated_at"))}
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
              View
            </DropdownMenuItem>
            <DropdownMenuItem onClick={(e) => {
              e.stopPropagation()
              meta?.onEdit?.(entry)
            }}>
              <Pencil className="mr-2 h-4 w-4" />
              Edit
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
