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
export type { AuthProviderProps, AuthError } from './context'

// Storage context
export { StorageProvider, useStorage } from './context/StorageContext'

// Hooks
export { useProtectedRoute } from './hooks'

// Components
export { default as Auth } from './components/auth'

// shadcn/ui components (re-export for convenience)
export { Button } from './components/ui/button'
export { Input } from './components/ui/input'
export { Label } from './components/ui/label'
export { Card, CardContent, CardDescription, CardHeader, CardTitle } from './components/ui/card'

// API client
export { ApiClient } from './api'

// Storage types and implementation
export type { TokenCallbacks, StorageCallbacks, FullStorageCallbacks } from './utils/storage'
export { BrowserStorage } from './utils/storage'

// Constants
export { STORAGE_KEYS } from './utils/constants'

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
