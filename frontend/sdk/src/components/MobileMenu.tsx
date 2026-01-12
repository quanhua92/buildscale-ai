/**
 * MobileMenu - Reusable hamburger menu component
 *
 * Wraps shadcn Sheet component to provide a consistent mobile navigation menu
 * across all consuming applications. Preserves the "Navigation" header design
 * while providing full accessibility (keyboard, focus trap, ARIA).
 */

import { Menu, X } from 'lucide-react'
import * as React from 'react'
import {
  Sheet,
  SheetClose,
  SheetContent,
  SheetTitle,
  SheetTrigger,
} from './ui/sheet'
import { Button } from './ui/button'

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
  // Default trigger is a Menu button
  const defaultTrigger = (
    <Button
      variant="ghost"
      size="icon"
      className="hover:bg-accent rounded-lg transition-colors"
      aria-label="Open menu"
    >
      <Menu size={24} />
    </Button>
  )

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetTrigger asChild>{trigger || defaultTrigger}</SheetTrigger>
      <SheetContent
        side="left"
        showClose={false}
        className="w-80 p-0 z-50 data-[state=closed]:duration-300 data-[state=open]:duration-300 flex flex-col"
      >
        {/* Custom header matching current design */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <SheetTitle asChild>
            <h2 className="text-xl font-bold">Navigation</h2>
          </SheetTitle>
          <SheetClose asChild>
            <Button
              variant="ghost"
              size="icon"
              className="hover:bg-accent rounded-lg transition-colors"
              aria-label="Close menu"
            >
              <X className="h-6 w-6" />
            </Button>
          </SheetClose>
        </div>

        {/* Navigation content */}
        <nav className="flex-1 p-4 overflow-y-auto">{children}</nav>
      </SheetContent>
    </Sheet>
  )
}
