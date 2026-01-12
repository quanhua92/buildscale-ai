/**
 * Authentication context provider for managing user auth state
 */

import { createContext, useContext, useState, useCallback, useMemo, useEffect } from 'react'
import type { ReactNode } from 'react'
import type { User } from '../api/types'
import ApiClient from '../api/client'
import { ApiError } from '../api/errors'
import { useStorage } from './StorageContext'
import { STORAGE_KEYS } from '../utils/constants'

export interface AuthError {
  message: string
  code?: string
  status?: number
  fields?: Record<string, string>  // Field-specific errors from backend
}

interface AuthContextType {
  user: User | null
  isAuthenticated: boolean
  isLoading: boolean
  error: AuthError | null
  success: boolean
  redirectTarget: string
  login: (email: string, password: string) => Promise<void>
  register: (data: {
    email: string
    password: string
    confirm_password: string
    full_name?: string
  }) => Promise<void>
  logout: () => Promise<void>
  clearError: () => void
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export interface AuthProviderProps {
  children: ReactNode
  apiBaseUrl: string
  redirectTarget?: string  // Frontend URL to redirect to after successful auth
}

export function AuthProvider({ children, apiBaseUrl, redirectTarget: redirectTargetProp = '/' }: AuthProviderProps) {
  const [user, setUser] = useState<User | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<AuthError | null>(null)
  const [success, setSuccess] = useState(false)
  // Track if we've already attempted to restore session (prevent multiple attempts)
  const [restoreAttempted, setRestoreAttempted] = useState(false)

  // Get ALL storage callbacks from context
  const {
    // Token callbacks - for ApiClient
    getAccessToken,
    getRefreshToken,
    setTokens,
    clearTokens,
    // Generic storage - for app data
    setItem,
    clearAuthData
  } = useStorage()

  // ApiClient gets token callbacks from context
  const apiClient = useMemo(
    () => new ApiClient({
      baseURL: apiBaseUrl,
      getAccessToken,
      getRefreshToken,
      setTokens,
      clearTokens
    }),
    [apiBaseUrl, getAccessToken, getRefreshToken, setTokens, clearTokens]
  )

  // Helper function to reduce duplication in auth operations
  const executeAuthOperation = useCallback(async (
    operation: () => Promise<{ user: User }>,
    fallbackErrorMessage: string
  ) => {
    setIsLoading(true)
    setError(null)
    setSuccess(false)
    try {
      const response = await operation()
      setUser(response.user)
      setItem(STORAGE_KEYS.USER_ID, response.user.id.toString())
      setSuccess(true)
    } catch (err) {
      if (err instanceof ApiError) {
        setError({
          message: err.message,
          code: err.code,
          status: err.status,
          fields: err.fields
        })
      } else {
        const message = err instanceof Error ? err.message : fallbackErrorMessage
        setError({ message })
      }
    } finally {
      setIsLoading(false)
    }
  }, [apiClient, setItem])

  const login = useCallback(async (email: string, password: string) => {
    await executeAuthOperation(
      () => apiClient.login({ email, password }),
      'Login failed'
    )
  }, [apiClient, executeAuthOperation])

  const register = useCallback(async (data: {
    email: string
    password: string
    confirm_password: string
    full_name?: string
  }) => {
    await executeAuthOperation(
      () => apiClient.register(data),
      'Registration failed'
    )
  }, [apiClient, executeAuthOperation])

  const logout = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    setSuccess(false)
    try {
      await apiClient.logout()
      setUser(null)
      clearAuthData()
      setSuccess(true)
    } catch (err) {
      if (err instanceof ApiError) {
        setError({ message: err.message, code: err.code, status: err.status })
      } else {
        setError({ message: err instanceof Error ? err.message : 'Logout failed' })
      }
    } finally {
      setIsLoading(false)
    }
  }, [apiClient, clearAuthData])

  const clearError = useCallback(() => {
    setError(null)
  }, [])

  // Restore session on mount (only once)
  useEffect(() => {
    if (restoreAttempted) {
      return  // Already attempted restoration, skip
    }

    const restoreSession = async () => {
      setRestoreAttempted(true)  // Mark as attempted
      console.log('[Auth] Attempting to restore session from cookies...')
      // Try to get user profile - backend will validate HttpOnly cookies
      try {
        const { user: profileUser } = await apiClient.getProfile()
        console.log('[Auth] Session restored successfully:', profileUser.id)
        setUser(profileUser)
      } catch (error) {
        // Invalid or expired token - not logged in
        console.log('[Auth] Session restoration failed:', error)
        // No need to clear tokens (HttpOnly cookies handled by backend)
      } finally {
        setIsLoading(false)
      }
    }

    restoreSession()
  }, [apiClient, restoreAttempted])

  const value: AuthContextType = {
    user,
    isAuthenticated: !!user,
    isLoading,
    error,
    success,
    redirectTarget: redirectTargetProp,
    login,
    register,
    logout,
    clearError,
  }

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}
