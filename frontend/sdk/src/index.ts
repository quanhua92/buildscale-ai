/**
 * @buildscale/sdk - Authentication SDK
 *
 * Compound Component Pattern authentication system with:
 * - API client with automatic token refresh
 * - Auth context and hooks
 * - Storage context for token and app data management
 * - Pre-built Login and Register components
 * - Fully composable auth UI components
 * - shadcn/ui components for consistent styling
 */

// Context and hooks
export { AuthProvider, useAuth } from './context'
export type { AuthProviderProps, AuthError, AuthResult, AuthContextType } from './context'
export { ThemeProvider, useTheme, useResolvedTheme } from './context'
export type { ThemeProviderProps, Theme } from './context'

// Storage context
export { StorageProvider, useStorage } from './context/StorageContext'

// Hooks
export { useProtectedRoute, useAuthRedirects } from './hooks'

// Components
export { default as Auth } from './components/auth'
export { MobileMenu } from './components/MobileMenu'
export type { MobileMenuProps } from './components/MobileMenu'

// shadcn/ui components (re-export for convenience)
export { Button } from './components/ui/button'
export { Input } from './components/ui/input'
export { Label } from './components/ui/label'
export { Card, CardContent, CardDescription, CardHeader, CardTitle } from './components/ui/card'
export { ThemeToggle } from './components/ui/theme-toggle'
export { Toaster } from './components/ui/sonner'
export {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectScrollDownButton,
  SelectScrollUpButton,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from './components/ui/select'
export {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from './components/ui/dialog'
export {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuPortal,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from './components/ui/dropdown-menu'
export {
  Sheet,
  SheetClose,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetOverlay,
  SheetPortal,
  SheetTitle,
  SheetTrigger,
} from './components/ui/sheet'
export {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableFooter,
  TableHead,
  TableHeader,
  TableRow,
} from './components/ui/table'

// API client
export { ApiClient } from './api'

// Storage types and implementation
export type { TokenCallbacks, StorageCallbacks, FullStorageCallbacks } from './utils/storage'
export { BrowserStorage } from './utils/storage'

// Constants
export { STORAGE_KEYS, STORAGE_PREFIX } from './utils/constants'
export { safeLocalStorage } from './utils'

// Types
export type {
  User,
  RegisterRequest,
  LoginRequest,
  AuthResponse,
  RefreshTokenResponse,
  ErrorResponse,
} from './api/types'

// Errors
export { ApiError, TokenTheftError } from './api/errors'

// Utils
export { cn } from './utils'
