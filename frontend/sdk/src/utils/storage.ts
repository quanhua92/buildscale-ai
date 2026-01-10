/**
 * Storage abstraction for different client types
 * Split into token-specific and generic storage interfaces
 */

import { STORAGE_PREFIX } from './constants'

// Token-specific methods (for ApiClient)
export interface TokenCallbacks {
  getAccessToken: () => string | null | Promise<string | null>
  getRefreshToken: () => string | null | Promise<string | null>
  setTokens: (accessToken: string, refreshToken: string) => void | Promise<void>
  clearTokens: () => void | Promise<void>
}

// Generic storage methods (for app data)
export interface StorageCallbacks {
  getItem: (key: string) => string | null | Promise<string | null>
  setItem: (key: string, value: string) => void | Promise<void>
  removeItem: (key: string) => void | Promise<void>
  clearAuthData: () => void | Promise<void>
}

// Combined for non-browser clients
export interface FullStorageCallbacks extends TokenCallbacks, StorageCallbacks {}

/**
 * Default storage for browser clients
 * - Implements TokenCallbacks: No-ops (HttpOnly cookies handled by backend)
 * - Implements StorageCallbacks: localStorage for app data
 */
export class BrowserStorage implements TokenCallbacks, StorageCallbacks {
  // Token methods - no-ops (backend handles HttpOnly cookies)
  getAccessToken(): string | null {
    return null // HttpOnly cookies - can't access
  }

  getRefreshToken(): string | null {
    return null // HttpOnly cookies - can't access
  }

  setTokens(): void {
    // Backend sets cookies via Set-Cookie headers
    // No-op for browser clients
  }

  clearTokens(): void {
    // Backend clears cookies via logout endpoint
    // No-op for browser clients
  }

  // Generic storage methods - use localStorage
  getItem(key: string): string | null {
    return localStorage.getItem(key)
  }

  setItem(key: string, value: string): void {
    localStorage.setItem(key, value)
  }

  removeItem(key: string): void {
    localStorage.removeItem(key)
  }

  clearAuthData(): void {
    // Clear all app data with our prefix from localStorage
    const keys = Object.keys(localStorage)
    keys.forEach(key => {
      if (key.startsWith(STORAGE_PREFIX)) {
        localStorage.removeItem(key)
      }
    })
  }
}
