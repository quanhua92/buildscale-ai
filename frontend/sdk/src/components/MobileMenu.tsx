/**
 * MobileMenu - Reusable hamburger menu component
 *
 * Wraps shadcn Sheet component to provide a consistent mobile navigation menu
 * across all consuming applications. Preserves the "Navigation" header design
 * while providing full accessibility (keyboard, focus trap, ARIA).
 *
 * Navigation structure follows shadcn sidebar patterns:
 * - Top-level items with text labels
 * - Collapsible sections with nested sub-items
 */

import { Home, ChevronRight, X } from 'lucide-react'
import * as React from 'react'
import {
  Sheet,
  SheetClose,
  SheetContent,
  SheetTitle,
  SheetTrigger,
} from './ui/sheet'
import { Button } from './ui/button'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from './ui/collapsible'

export interface MobileMenuProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  trigger?: React.ReactNode
  children: React.ReactNode
}

export function MobileMenu({
  open,
  onOpenChange,
  trigger,
  children,
}: MobileMenuProps) {
  const [isWorkspacesOpen, setIsWorkspacesOpen] = React.useState(false)

  // Default trigger is a Menu button
  const defaultTrigger = (
    <Button
      variant="ghost"
      size="icon"
      className="hover:bg-accent rounded-md"
      aria-label="Open menu"
    >
      <Home className="h-5 w-5" />
    </Button>
  )

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetTrigger asChild>{trigger || defaultTrigger}</SheetTrigger>
      <SheetContent
        side="left"
        className="w-[280px] p-0 z-50 flex flex-col gap-0 [&>button:first-child]:hidden"
      >
        {/* Custom header matching current design */}
        <div className="flex items-center justify-between p-4 border-b">
          <SheetTitle asChild>
            <h2 className="text-lg font-semibold">Navigation</h2>
          </SheetTitle>
          <SheetClose asChild>
            <Button
              variant="ghost"
              size="icon"
              className="hover:bg-accent rounded-md"
              aria-label="Close menu"
            >
              <X className="h-5 w-5" />
            </Button>
          </SheetClose>
        </div>

        {/* Navigation content */}
        <nav className="flex-1 overflow-y-auto px-3 py-4">
          <div className="space-y-1">
            {/* Navigation content from consumers */}
            {children}

            {/* Separator */}
            <div className="my-2 border-t" />

            {/* Workspaces collapsible section */}
            <Collapsible
              open={isWorkspacesOpen}
              onOpenChange={setIsWorkspacesOpen}
            >
              <CollapsibleTrigger asChild>
                <Button
                  variant="ghost"
                  className="w-full justify-between px-3 py-2 h-auto text-sm font-medium hover:bg-accent rounded-md group"
                >
                  <span>Workspaces</span>
                  <ChevronRight className="h-4 w-4 transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90" />
                </Button>
              </CollapsibleTrigger>
              <CollapsibleContent className="pl-4 pt-1 space-y-1">
                <SheetClose asChild>
                  <Button
                    variant="ghost"
                    className="w-full justify-start px-3 py-2 h-auto text-sm hover:bg-accent/50 rounded-md"
                    asChild
                  >
                    <a href="/admin/workspaces/all">All</a>
                  </Button>
                </SheetClose>
              </CollapsibleContent>
            </Collapsible>
          </div>
        </nav>
      </SheetContent>
    </Sheet>
  )
}
