/**
 * MobileMenu - Backward compatibility shim
 *
 * @deprecated Use `NavigationMenu` instead. This component is kept for
 * backward compatibility and will be removed in a future version.
 *
 * ## Migration
 *
 * ```tsx
 * // Before
 * import { MobileMenu } from '@buildscale/sdk'
 * <MobileMenu open={isOpen} onOpenChange={setIsOpen}>
 *   <SheetClose asChild>
 *     <Link to="/">Home</Link>
 *   </SheetClose>
 * </MobileMenu>
 *
 * // After
 * import { NavigationMenu } from '@buildscale/sdk'
 * <NavigationMenu open={isOpen} onOpenChange={setIsOpen}>
 *   <NavigationMenu.Item to="/">Home</NavigationMenu.Item>
 * </NavigationMenu>
 * ```
 */

// Re-export as MobileMenu for backward compatibility
export { NavigationMenu as MobileMenu } from './NavigationMenu'

export type {
  NavigationMenuProps as MobileMenuProps,
  NavigationMenuItemProps as MobileMenuItemProps,
  NavigationMenuSectionProps as MobileMenuSectionProps,
  NavigationMenuGroupProps as MobileMenuGroupProps,
  NavigationMenuSeparatorProps as MobileMenuSeparatorProps,
} from './NavigationMenu'
