/**
 * Authentication context provider for managing user auth state
 */

import { createContext, useContext, useState, useCallback, useMemo, useEffect } from 'react'
import type { ReactNode } from 'react'
import type { User, Workspace } from '../api/types'
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

export interface AuthResult {
  success: boolean
  error?: AuthError
}

export interface ApiResult<T = void> {
  success: boolean
  data?: T
  error?: AuthError
}

export interface AuthContextType {
  user: User | null
  isAuthenticated: boolean
  isRestoring: boolean  // Only for initial session restore
  redirectTarget: string
  login: (email: string, password: string) => Promise<AuthResult>
  register: (data: {
    email: string
    password: string
    confirm_password: string
    full_name?: string
  }) => Promise<AuthResult>
  logout: () => Promise<AuthResult>
  createWorkspace: (name: string) => Promise<ApiResult<Workspace>>
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export interface AuthProviderProps {
  children: ReactNode
  apiBaseUrl: string
  redirectTarget?: string  // Frontend URL to redirect to after successful auth
}

export function AuthProvider({ children, apiBaseUrl, redirectTarget: redirectTargetProp = '/' }: AuthProviderProps) {
  const [user, setUser] = useState<User | null>(null)
  const [isRestoring, setIsRestoring] = useState(true)
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

  const handleError = useCallback((err: unknown): AuthError => {
    if (err instanceof ApiError) {
      return {
        message: err.message,
        code: err.code,
        status: err.status,
        fields: err.fields
      }
    }
    return {
      message: err instanceof Error ? err.message : 'An error occurred'
    }
  }, [])

  const handleAuthSuccess = useCallback((user: User) => {
    setUser(user)
    setItem(STORAGE_KEYS.USER_ID, user.id.toString())
  }, [setItem])

  const login = useCallback(async (email: string, password: string): Promise<AuthResult> => {
    try {
      const response = await apiClient.login({ email, password })
      handleAuthSuccess(response.user)
      return { success: true }
    } catch (err) {
      return { success: false, error: handleError(err) }
    }
  }, [apiClient, handleAuthSuccess, handleError])

  const register = useCallback(async (data: {
    email: string
    password: string
    confirm_password: string
    full_name?: string
  }): Promise<AuthResult> => {
    try {
      const response = await apiClient.register(data)
      handleAuthSuccess(response.user)
      return { success: true }
    } catch (err) {
      return { success: false, error: handleError(err) }
    }
  }, [apiClient, handleAuthSuccess, handleError])

  const logout = useCallback(async (): Promise<AuthResult> => {
    try {
      await apiClient.logout()
      setUser(null)
      clearAuthData()
      return { success: true }
    } catch (err) {
      return { success: false, error: handleError(err) }
    }
  }, [apiClient, clearAuthData, handleError])

  const createWorkspace = useCallback(async (name: string): Promise<ApiResult<Workspace>> => {
    try {
      const response = await apiClient.createWorkspace(name)
      return { success: true, data: response.workspace }
    } catch (err) {
      return { success: false, error: handleError(err) }
    }
  }, [apiClient, handleError])

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
        setIsRestoring(false)
      }
    }

    restoreSession()
  }, [apiClient, restoreAttempted])

  const value: AuthContextType = {
    user,
    isAuthenticated: !!user,
    isRestoring,
    redirectTarget: redirectTargetProp,
    login,
    register,
    logout,
    createWorkspace,
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
