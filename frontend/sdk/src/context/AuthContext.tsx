/**
 * Authentication context provider for managing user auth state
 */

import { createContext, useContext, useState, useCallback, useMemo, useEffect } from 'react'
import type { ReactNode } from 'react'
import type { User } from '../api/types'
import ApiClient from '../api/client'
import { BrowserTokenStorage } from '../utils/storage'

interface AuthContextType {
  user: User | null
  isAuthenticated: boolean
  isLoading: boolean
  error: string | null
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
}

export function AuthProvider({ children, apiBaseUrl }: AuthProviderProps) {
  const [user, setUser] = useState<User | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const storage = useMemo(() => new BrowserTokenStorage(), [])
  const apiClient = useMemo(
    () => new ApiClient({ baseURL: apiBaseUrl }, storage),
    [apiBaseUrl, storage]
  )

  const login = useCallback(async (email: string, password: string) => {
    setIsLoading(true)
    setError(null)
    try {
      const response = await apiClient.login({ email, password })
      setUser(response.user)
      // Store tokens
      storage.setTokens(response.access_token, response.refresh_token)
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Login failed'
      setError(message)
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [apiClient, storage])

  const register = useCallback(async (data: {
    email: string
    password: string
    confirm_password: string
    full_name?: string
  }) => {
    setIsLoading(true)
    setError(null)
    try {
      const response = await apiClient.register(data)
      setUser(response.user)
      // Store tokens
      storage.setTokens(response.access_token, response.refresh_token)
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Registration failed'
      setError(message)
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [apiClient, storage])

  const logout = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      await apiClient.logout()
      setUser(null)
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Logout failed'
      setError(message)
    } finally {
      setIsLoading(false)
    }
  }, [apiClient])

  const clearError = useCallback(() => {
    setError(null)
  }, [])

  // Restore session on mount
  useEffect(() => {
    const restoreSession = async () => {
      const token = storage.getAccessToken()
      if (token) {
        try {
          setIsLoading(true)
          const { user: profileUser } = await apiClient.getProfile()
          setUser(profileUser)
        } catch (error) {
          // Invalid or expired token - clear tokens and stay logged out
          storage.clearTokens()
        } finally {
          setIsLoading(false)
        }
      } else {
        // No token found - not logged in
        setIsLoading(false)
      }
    }

    restoreSession()
  }, [apiClient, storage])

  const value: AuthContextType = {
    user,
    isAuthenticated: !!user,
    isLoading,
    error,
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
