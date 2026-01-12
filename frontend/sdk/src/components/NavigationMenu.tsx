/**
 * NavigationMenu - Compound component for mobile navigation
 *
 * Provides a flexible, composable navigation menu using the compound component pattern.
 * Allows consuming applications to define their own navigation structure while
 * maintaining consistent styling and behavior.
 *
 * ## Usage
 *
 * ```tsx
 * <NavigationMenu open={isOpen} onOpenChange={setIsOpen}>
 *   <NavigationMenu.Item to="/" icon={<Home />}>Home</NavigationMenu.Item>
 *   <NavigationMenu.Separator />
 *   <NavigationMenu.Section title="Workspaces">
 *     <NavigationMenu.Item to="/workspaces/all">All</NavigationMenu.Item>
 *   </NavigationMenu.Section>
 * </NavigationMenu>
 * ```
 */

import { ChevronRight, X, Menu } from 'lucide-react'
import * as React from 'react'
import { Link } from '@tanstack/react-router'
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
import { cn } from '../utils'

// ============================================================================
// TYPES
// ============================================================================

export interface NavigationMenuProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  trigger?: React.ReactNode
  children: React.ReactNode
}

export interface NavigationMenuItemProps {
  children: React.ReactNode
  to?: string
  href?: string
  icon?: React.ReactNode
  className?: string
  asChild?: boolean
  [key: string]: any
}

export interface NavigationMenuSectionProps {
  title: string
  defaultOpen?: boolean
  children: React.ReactNode
  className?: string
}

export interface NavigationMenuGroupProps {
  title?: string
  children: React.ReactNode
  className?: string
}

export interface NavigationMenuSeparatorProps {
  className?: string
}

// ============================================================================
// MAIN COMPONENT
// ============================================================================

function NavigationMenu({
  open,
  onOpenChange,
  trigger,
  children,
}: NavigationMenuProps) {
  // Default trigger is a Menu icon button
  const defaultTrigger = (
    <Button
      variant="ghost"
      size="icon"
      className="hover:bg-accent rounded-md"
      aria-label="Open menu"
    >
      <Menu className="h-5 w-5" />
    </Button>
  )

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetTrigger asChild>{trigger || defaultTrigger}</SheetTrigger>
      <SheetContent
        side="left"
        className="w-[280px] p-0 z-50 flex flex-col gap-0 [&>button:first-child]:hidden"
      >
        {/* Custom header */}
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
          <div className="space-y-1">{children}</div>
        </nav>
      </SheetContent>
    </Sheet>
  )
}

// ============================================================================
// ITEM SUB-COMPONENT
// ============================================================================

function Item({
  children,
  to,
  href,
  icon,
  className,
  asChild = false,
  ...props
}: NavigationMenuItemProps) {
  const baseClasses = cn(
    "flex w-full items-center gap-3 px-3 py-2 h-auto",
    "text-sm font-medium",
    "hover:bg-accent hover:text-accent-foreground",
    "rounded-md transition-colors",
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
    className
  )

  const iconClasses = "shrink-0 h-5 w-5"

  // If to is provided, render as Link (TanStack Router)
  if (to) {
    return (
      <SheetClose asChild>
        <Link to={to} className={baseClasses} {...props}>
          {icon && <span className={iconClasses}>{icon}</span>}
          {children}
        </Link>
      </SheetClose>
    )
  }

  // If href is provided, render as anchor
  if (href) {
    return (
      <SheetClose asChild>
        <a href={href} className={baseClasses} {...props}>
          {icon && <span className={iconClasses}>{icon}</span>}
          {children}
        </a>
      </SheetClose>
    )
  }

  // Default: render as button
  return (
    <SheetClose asChild>
      <button className={baseClasses} {...props}>
        {icon && <span className={iconClasses}>{icon}</span>}
        {children}
      </button>
    </SheetClose>
  )
}

// ============================================================================
// SECTION SUB-COMPONENT (Collapsible)
// ============================================================================

function Section({
  title,
  defaultOpen = false,
  children,
  className,
}: NavigationMenuSectionProps) {
  const [isOpen, setIsOpen] = React.useState(defaultOpen)

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen} className={cn("group/collapsible", className)}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          className={cn(
            "w-full justify-between px-3 py-2 h-auto",
            "text-sm font-medium",
            "hover:bg-accent hover:text-accent-foreground",
            "rounded-md group"
          )}
        >
          <span>{title}</span>
          <ChevronRight className="h-4 w-4 transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90" />
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent className={cn("pl-4 pt-1 space-y-1")}>
        {children}
      </CollapsibleContent>
    </Collapsible>
  )
}

// ============================================================================
// GROUP SUB-COMPONENT (Non-collapsible)
// ============================================================================

function Group({ title, children, className }: NavigationMenuGroupProps) {
  return (
    <div className={cn("space-y-1", className)}>
      {title && (
        <div className="px-3 py-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
          {title}
        </div>
      )}
      {children}
    </div>
  )
}

// ============================================================================
// SEPARATOR SUB-COMPONENT
// ============================================================================

function Separator({ className }: NavigationMenuSeparatorProps) {
  return <div className={cn("my-2 border-t border-border", className)} />
}

// ============================================================================
// ATTACH SUB-COMONENTS
// ============================================================================

NavigationMenu.Item = Item
NavigationMenu.Section = Section
NavigationMenu.Group = Group
NavigationMenu.Separator = Separator

// ============================================================================
// EXPORTS
// ============================================================================

export { NavigationMenu }
