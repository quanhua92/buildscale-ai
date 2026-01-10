/**
 * API client with automatic token refresh on 401 responses
 * Implements token rotation with stolen token detection
 */

import type {
  RegisterRequest,
  LoginRequest,
  AuthResponse,
  RefreshTokenResponse,
  User,
} from './types'
import { ApiError, TokenTheftError } from './errors'
import type { TokenStorage } from '../utils/storage'

interface ApiClientConfig {
  baseURL: string
  timeout?: number
}

class ApiClient {
  private baseURL: string
  private timeout: number
  private storage: TokenStorage
  private isRefreshing = false
  private refreshPromise: Promise<string | null> | null = null

  constructor(config: ApiClientConfig, storage: TokenStorage) {
    this.baseURL = config.baseURL
    this.timeout = config.timeout || 10000
    this.storage = storage
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseURL}${endpoint}`
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.timeout)

    try {
      let headers: HeadersInit = {
        'Content-Type': 'application/json',
        ...options.headers,
      }

      // Add authorization header if access token exists
      const accessToken = this.storage.getAccessToken()
      if (accessToken) {
        headers = {
          ...headers,
          Authorization: `Bearer ${accessToken}`,
        }
      }

      const response = await this.fetchWithAuth(url, options, headers, controller.signal)

      clearTimeout(timeoutId)

      // Handle 401 - try token refresh (except for refresh endpoint itself)
      if (response.status === 401 && !endpoint.includes('/auth/refresh')) {
        const newToken = await this.refreshAccessToken()
        if (newToken) {
          // Retry original request with new token
          const retryHeaders = {
            ...headers,
            Authorization: `Bearer ${newToken}`,
          }
          const retryResponse = await this.fetchWithAuth(url, options, retryHeaders, controller.signal)
          return this.handleResponse<T>(retryResponse)
        }
      }

      return this.handleResponse<T>(response)
    } catch (error) {
      if (error instanceof Error) {
        if (error.name === 'AbortError') {
          throw new ApiError('Request timeout', 408)
        }
      }
      throw error
    }
  }

  private async fetchWithAuth(
    url: string,
    options: RequestInit,
    headers: HeadersInit,
    signal: AbortSignal
  ): Promise<Response> {
    return fetch(url, {
      ...options,
      headers,
      signal,
      credentials: 'include', // Include cookies for browser clients
    })
  }

  private async handleResponse<T>(response: Response): Promise<T> {
    const data = await response.json()

    if (!response.ok) {
      // Handle token theft detection (403 from refresh endpoint)
      if (response.status === 403 && data.code === 'TOKEN_THEFT') {
        throw new TokenTheftError(data.message || 'Token theft detected')
      }

      throw new ApiError(
        data.error || data.message || 'Request failed',
        response.status,
        data.code
      )
    }

    return data as T
  }

  private refreshAccessToken(): Promise<string | null> {
    // Prevent multiple refresh attempts by returning the existing promise
    if (this.isRefreshing) {
      return this.refreshPromise ?? Promise.resolve(null)
    }

    const refreshToken = this.storage.getRefreshToken()
    if (!refreshToken) {
      return Promise.resolve(null)
    }

    this.isRefreshing = true

    // Create and assign promise immediately (before async operations)
    // This ensures concurrent callers get the same promise and wait for the result
    this.refreshPromise = (async () => {
      try {
        const result = await this.performRefresh(refreshToken)
        return result.access_token
      } catch (error) {
        // Clear tokens on refresh failure
        this.storage.clearTokens()
        throw error
      } finally {
        this.isRefreshing = false
        this.refreshPromise = null
      }
    })()

    return this.refreshPromise
  }

  private async performRefresh(
    refreshToken: string
  ): Promise<RefreshTokenResponse> {
    const response = await fetch(`${this.baseURL}/auth/refresh`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      credentials: 'include',
      body: JSON.stringify({ refresh_token: refreshToken }),
    })

    const data = await response.json()

    if (!response.ok) {
      // Token theft detected
      if (response.status === 403) {
        throw new TokenTheftError(data.message || 'Token theft detected')
      }
      throw new ApiError(data.message || 'Refresh failed', response.status)
    }

    // Update tokens in storage (rotation)
    if (data.refresh_token) {
      // New refresh token provided (normal rotation)
      this.storage.setTokens(data.access_token, data.refresh_token)
    } else {
      // Only access token refreshed (within 5-minute grace period)
      this.storage.setTokens(data.access_token, refreshToken)
    }

    return data
  }

  // ============================================================================
  // Public API Methods
  // ============================================================================

  async register(data: RegisterRequest): Promise<AuthResponse> {
    return this.request<AuthResponse>('/auth/register', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  }

  async login(data: LoginRequest): Promise<AuthResponse> {
    return this.request<AuthResponse>('/auth/login', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  }

  async logout(): Promise<{ message: string }> {
    try {
      return this.request<{ message: string }>('/auth/logout', {
        method: 'POST',
      })
    } finally {
      this.storage.clearTokens()
    }
  }

  async refreshToken(): Promise<RefreshTokenResponse> {
    const refreshToken = this.storage.getRefreshToken()
    if (!refreshToken) {
      throw new ApiError('No refresh token available', 401)
    }
    return this.performRefresh(refreshToken)
  }

  async getProfile(): Promise<{ user: User }> {
    return this.request<{ user: User }>('/auth/me')
  }

  // Generic methods for other API calls
  async get<T>(endpoint: string): Promise<T> {
    return this.request<T>(endpoint, { method: 'GET' })
  }

  async post<T>(endpoint: string, data: unknown): Promise<T> {
    return this.request<T>(endpoint, {
      method: 'POST',
      body: JSON.stringify(data),
    })
  }

  async put<T>(endpoint: string, data: unknown): Promise<T> {
    return this.request<T>(endpoint, {
      method: 'PUT',
      body: JSON.stringify(data),
    })
  }

  async delete<T>(endpoint: string): Promise<T> {
    return this.request<T>(endpoint, { method: 'DELETE' })
  }
}

export default ApiClient
