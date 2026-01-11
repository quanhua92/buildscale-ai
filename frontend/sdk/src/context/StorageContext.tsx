/**
 * Storage Context Provider
 * Provides BOTH token callbacks (for ApiClient) and generic storage callbacks (for app data)
 */

import { createContext, useContext, useMemo, type ReactNode } from 'react'
import type { FullStorageCallbacks } from '../utils/storage'
import { BrowserStorage } from '../utils/storage'

interface StorageContextType {
  // Token callbacks - for ApiClient
  getAccessToken: () => string | null | Promise<string | null>
  getRefreshToken: () => string | null | Promise<string | null>
  setTokens: (access: string, refresh: string) => void | Promise<void>
  clearTokens: () => void | Promise<void>

  // Generic storage - for app data
  getItem: (key: string) => string | null | Promise<string | null>
  setItem: (key: string, value: string) => void | Promise<void>
  removeItem: (key: string) => void | Promise<void>
  clearAuthData: () => void | Promise<void>
}

const StorageContext = createContext<StorageContextType | undefined>(undefined)

export interface StorageProviderProps {
  children: ReactNode
  storage?: FullStorageCallbacks // Defaults to BrowserStorage
}

export function StorageProvider({ children, storage = new BrowserStorage() }: StorageProviderProps) {
  const value: StorageContextType = useMemo(() => ({
    // Token callbacks
    getAccessToken: () => {
      const result = storage.getAccessToken()
      return result instanceof Promise ? result.then(r => r ?? null) : result ?? null
    },
    getRefreshToken: () => {
      const result = storage.getRefreshToken()
      return result instanceof Promise ? result.then(r => r ?? null) : result ?? null
    },
    setTokens: (access: string, refresh: string) => storage.setTokens(access, refresh),
    clearTokens: () => storage.clearTokens(),

    // Generic storage
    getItem: (key: string) => {
      const result = storage.getItem(key)
      return result instanceof Promise ? result.then(r => r ?? null) : result
    },
    setItem: (key: string, value: string) => storage.setItem(key, value),
    removeItem: (key: string) => storage.removeItem(key),
    clearAuthData: () => storage.clearAuthData(),
  }), [storage])

  return <StorageContext.Provider value={value}>{children}</StorageContext.Provider>
}

export function useStorage() {
  const context = useContext(StorageContext)
  if (!context) {
    throw new Error('useStorage must be used within a StorageProvider')
  }
  return context
}
